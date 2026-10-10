//! One reviewed GET, with bounded response data and cancellable waiting.
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::{
    Request,
    client::conn::http1,
    header::{HeaderName, HeaderValue},
};
use hyper_util::rt::TokioIo;
use netscope_core::{ReplayReview, RequestId};
use std::{
    io,
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::oneshot,
};
use tokio_rustls::{
    TlsConnector,
    rustls::{ClientConfig, RootCertStore, pki_types::ServerName},
};

/// Maximum number of retained body bytes, excluding HTTP framing.
pub const BODY_LIMIT: usize = 2 * 1024 * 1024;
/// One deadline covers connection setup and the entire response.
pub const TIME_LIMIT: Duration = Duration::from_secs(10);
const HEADER_LIMIT: usize = 32 * 1024;

/// Why an operation stopped. HTTP error statuses can still be complete responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The response body ended normally.
    Complete,
    /// Local cancellation stopped waiting; remote work may already have occurred.
    Cancelled,
    /// The original total deadline expired.
    TimedOut,
    /// At least one byte existed beyond the retained limit.
    SizeLimited,
    /// Connection, TLS, or HTTP processing failed. Messages contain no request data.
    Failed(&'static str),
}
/// A separate result that never mutates the imported session.
#[derive(Debug)]
pub struct ReplayResult {
    /// Original captured request identity.
    pub source: RequestId,
    /// Reviewed destination for this new operation.
    pub destination: String,
    /// Terminal outcome, accepted once by the UI.
    pub outcome: Outcome,
    /// Received HTTP status, if headers arrived.
    pub status: Option<u16>,
    /// Actual response protocol, if headers arrived.
    pub version: Option<String>,
    /// Raw header values; rendering must escape controls.
    pub headers: Vec<(String, Vec<u8>)>,
    /// Bytes retained before the operation stopped.
    pub body: Vec<u8>,
    /// Total duration of this operation, not a browser timing reconstruction.
    pub elapsed: Duration,
}
impl ReplayResult {
    fn empty(review: &ReplayReview) -> Self {
        Self {
            source: review.source,
            destination: review.destination.clone(),
            outcome: Outcome::Failed("Worker stopped without a result"),
            status: None,
            version: None,
            headers: vec![],
            body: vec![],
            elapsed: Duration::ZERO,
        }
    }
}

fn configuration() -> Arc<ClientConfig> {
    let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Arc::new(config)
}

fn request(review: &ReplayReview) -> Result<(url::Url, Request<Empty<Bytes>>), &'static str> {
    let url = url::Url::parse(&review.destination).map_err(|_| "Invalid destination")?;
    if url.as_str() != review.destination
        || review.destination.len() > 16_384
        || !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("Invalid reviewed destination");
    }
    let authority = &url[url::Position::BeforeHost..url::Position::AfterPort];
    if authority != review.authority {
        return Err("Destination and Host disagree");
    }
    let mut request = Request::builder()
        .method("GET")
        .uri(&url[url::Position::BeforePath..])
        .version(hyper::Version::HTTP_11)
        .body(Empty::new())
        .map_err(|_| "Invalid request target")?;
    let mut size = 0usize;
    if review.headers.len() > 100 {
        return Err("Limit: 100 outgoing headers");
    }
    for h in &review.headers {
        netscope_core::validate_replay_header(h)?;
        size = size.saturating_add(h.name.len() + h.value.len() + 4);
        if size > HEADER_LIMIT {
            return Err("Outgoing headers exceed 32 KiB");
        }
        request.headers_mut().append(
            HeaderName::from_bytes(h.name.as_bytes()).map_err(|_| "Invalid header name")?,
            HeaderValue::from_str(&h.value).map_err(|_| "Invalid header value")?,
        );
    }
    request.headers_mut().insert(
        "host",
        HeaderValue::from_str(authority).map_err(|_| "Invalid Host")?,
    );
    request
        .headers_mut()
        .insert("accept-encoding", HeaderValue::from_static("identity"));
    Ok((url, request))
}
/// Check the concrete client's representation before offering Send. No I/O occurs.
/// # Errors
/// Rejects unsupported destinations, header syntax, or combined header budgets.
pub fn validate(review: &ReplayReview) -> Result<(), &'static str> {
    request(review).map(|_| ())
}

trait Stream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Stream for T {}

async fn exchange(
    review: &ReplayReview,
    result: &mut ReplayResult,
    tls: Arc<ClientConfig>,
) -> Outcome {
    match exchange_inner(review, result, tls).await {
        Ok(outcome) => outcome,
        Err(reason) => Outcome::Failed(reason),
    }
}
async fn exchange_inner(
    review: &ReplayReview,
    result: &mut ReplayResult,
    tls: Arc<ClientConfig>,
) -> Result<Outcome, &'static str> {
    let (url, request) = request(review)?;
    let host = url
        .host_str()
        .ok_or("Missing host")?
        .trim_matches(['[', ']']);
    let port = url.port_or_known_default().ok_or("Missing port")?;
    let tcp = TcpStream::connect((host, port))
        .await
        .map_err(|_| "Could not connect to destination")?;
    let stream: Box<dyn Stream> = if url.scheme() == "https" {
        let name = ServerName::try_from(host.to_owned()).map_err(|_| "Invalid TLS hostname")?;
        Box::new(
            TlsConnector::from(tls)
                .connect(name, tcp)
                .await
                .map_err(|_| "TLS handshake or certificate verification failed")?,
        )
    } else {
        Box::new(tcp)
    };
    let (mut sender, connection) = http1::Builder::new()
        .max_headers(100)
        .max_header_size(HEADER_LIMIT)
        .max_buf_size(HEADER_LIMIT)
        .handshake(TokioIo::new(stream))
        .await
        .map_err(|_| "Could not initialize HTTP connection")?;
    // Both futures live here: dropping this exchange also drops the connection's socket.
    let response = async {
        let response = sender
            .send_request(request)
            .await
            .map_err(|_| "Could not read valid response headers")?;
        result.status = Some(response.status().as_u16());
        result.version = Some(format!("{:?}", response.version()));
        result.headers = response
            .headers()
            .iter()
            .map(|(n, v)| (n.to_string(), v.as_bytes().to_vec()))
            .collect();
        retain_body(response.into_body(), &mut result.body).await
    };
    tokio::pin!(response, connection);
    tokio::select! {
        biased;
        result = &mut response => result,
        _ = &mut connection => response.await,
    }
}
async fn retain_body(
    mut body: impl hyper::body::Body<Data = Bytes> + Unpin,
    retained: &mut Vec<u8>,
) -> Result<Outcome, &'static str> {
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|_| "Response body ended with a read error")?;
        if let Ok(data) = frame.into_data() {
            let remaining = BODY_LIMIT - retained.len();
            retained.extend_from_slice(&data[..remaining.min(data.len())]);
            if data.len() > remaining {
                return Ok(Outcome::SizeLimited);
            }
        }
    }
    Ok(Outcome::Complete)
}
async fn execute(
    review: &ReplayReview,
    cancel: oneshot::Receiver<()>,
    deadline: tokio::time::Instant,
    tls: Arc<ClientConfig>,
) -> ReplayResult {
    let mut result = ReplayResult::empty(review);
    result.outcome = wait_for(exchange(review, &mut result, tls), cancel, deadline).await;
    result
}
async fn wait_for(
    work: impl std::future::Future<Output = Outcome>,
    cancel: oneshot::Receiver<()>,
    deadline: tokio::time::Instant,
) -> Outcome {
    tokio::select! {
        biased;
        _ = cancel => Outcome::Cancelled,
        _ = tokio::time::sleep_until(deadline) => Outcome::TimedOut,
        outcome = work => outcome,
    }
}

/// Owns one worker. Dropping it cancels I/O and waits for the worker's cleanup.
#[derive(Debug)]
pub struct Worker {
    cancel: Option<oneshot::Sender<()>>,
    result: mpsc::Receiver<ReplayResult>,
    thread: Option<thread::JoinHandle<()>>,
    fallback: Option<ReplayResult>,
}
impl Worker {
    /// Start only after explicit Send. The deadline starts here, before thread setup.
    /// # Errors
    /// Returns invalid review or worker-thread creation errors without sending.
    pub fn start(review: ReplayReview) -> io::Result<Self> {
        Self::start_at(review, Instant::now())
    }
    /// Start using the instant when the UI accepted Send, including dispatch delay.
    /// # Errors
    /// Returns validation or thread-creation errors without sending.
    pub fn start_at(review: ReplayReview, started: Instant) -> io::Result<Self> {
        validate(&review).map_err(io::Error::other)?;
        let fallback = Some(ReplayResult::empty(&review));
        let (cancel, cancelled) = oneshot::channel();
        let (sent, result) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("netscope-replay".into())
            .spawn(move || {
                let mut result = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => {
                        let result = runtime.block_on(execute(
                            &review,
                            cancelled,
                            (started + TIME_LIMIT).into(),
                            configuration(),
                        ));
                        // OS DNS resolution can outlive cancellation; it cannot access UI state.
                        runtime.shutdown_timeout(Duration::ZERO);
                        result
                    }
                    Err(_) => ReplayResult::empty(&review),
                };
                result.elapsed = started.elapsed();
                let _ = sent.send(result);
            })?;
        Ok(Self {
            cancel: Some(cancel),
            result,
            thread: Some(thread),
            fallback,
        })
    }
    /// Ask the worker to stop. Repeated cancellation is harmless.
    pub fn cancel(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
    }
    /// Poll once without blocking terminal input. A result is returned at most once.
    pub fn poll(&mut self) -> Option<ReplayResult> {
        match self.result.try_recv() {
            Ok(result) => {
                self.fallback = None;
                Some(result)
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => self.fallback.take(),
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests;

#[doc = include_str!("../../../docs/chapters/17-waiting-for-a-response.md")]
pub mod waiting_lesson {}
