use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn review(address: std::net::SocketAddr) -> ReplayReview {
    ReplayReview {
        method: netscope_core::ReplayMethod::Get,
        format: netscope_core::BodyFormat::Plain,
        body: String::new(),
        source: RequestId(7),
        destination: format!("http://{address}/a?q=1"),
        authority: address.to_string(),
        fragment: None,
        headers: vec![],
    }
}
async fn read_head(stream: &mut (impl AsyncRead + Unpin)) -> io::Result<String> {
    let mut bytes = vec![];
    while !bytes.ends_with(b"\r\n\r\n") {
        bytes.push(stream.read_u8().await?);
        if bytes.len() > HEADER_LIMIT {
            return Err(io::Error::other("request too large"));
        }
    }
    String::from_utf8(bytes).map_err(io::Error::other)
}
async fn response(bytes: Vec<u8>) -> Result<(ReplayResult, String), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut review = review(listener.local_addr()?);
    review.headers = vec![
        netscope_core::NameValue {
            name: "X-Example".into(),
            value: "one".into(),
        },
        netscope_core::NameValue {
            name: "X-Example".into(),
            value: "two".into(),
        },
    ];
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await?;
        let head = read_head(&mut socket).await?;
        let _ = socket.write_all(&bytes).await;
        socket.shutdown().await?;
        // This listener sees exactly one connection, even for redirects and failures.
        Ok::<_, io::Error>(head)
    });
    let (_cancel, cancelled) = oneshot::channel();
    let result = execute(
        &review,
        cancelled,
        tokio::time::Instant::now() + TIME_LIMIT,
        configuration(),
    )
    .await;
    Ok((result, tokio::time::timeout(TIME_LIMIT, server).await???))
}
#[tokio::test]
async fn exact_body_boundaries_and_explicit_wire_headers() -> TestResult {
    for size in [BODY_LIMIT - 1, BODY_LIMIT, BODY_LIMIT + 1] {
        let mut wire = format!("HTTP/1.1 200 OK\r\nContent-Length: {size}\r\n\r\n").into_bytes();
        wire.extend(std::iter::repeat_n(b'x', size));
        let (result, head) = response(wire).await?;
        assert_eq!(
            result.outcome,
            if size > BODY_LIMIT {
                Outcome::SizeLimited
            } else {
                Outcome::Complete
            }
        );
        assert_eq!(result.body.len(), size.min(BODY_LIMIT));
        assert_eq!(result.status, Some(200));
        assert_eq!(result.version.as_deref(), Some("HTTP/1.1"));
        let lines: Vec<_> = head.lines().collect();
        assert_eq!(lines[0], "GET /a?q=1 HTTP/1.1");
        assert!(lines.contains(&"x-example: one"));
        assert!(lines.contains(&"x-example: two"));
        assert!(lines.contains(&"accept-encoding: identity"));
        assert_eq!(lines.len(), 6); // Request line, four fields, and the empty line.
    }
    Ok(())
}
#[tokio::test]
async fn redirect_error_status_chunking_encoding_and_disconnect() -> TestResult {
    for status in [302, 503] {
        let (result, _) = response(format!("HTTP/1.1 {status} Test\r\nLocation: http://127.0.0.1:1/never\r\nContent-Length: 0\r\n\r\n").into_bytes()).await?;
        assert_eq!(result.outcome, Outcome::Complete);
        assert_eq!(result.status, Some(status));
    }
    let (result, _) = response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Encoding: gzip\r\n\r\n3\r\nabc\r\n2\r\nde\r\n0\r\nX-End: yes\r\n\r\n".to_vec()).await?;
    assert_eq!(result.outcome, Outcome::Complete);
    assert_eq!(result.body, b"abcde");
    let (result, _) =
        response(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nabc".to_vec()).await?;
    assert!(matches!(result.outcome, Outcome::Failed(_)));
    assert_eq!(result.body, b"abc");
    let (result, _) = response(b"HTTP/1.0 200 OK\r\n\r\nclose-delimited".to_vec()).await?;
    assert_eq!(result.outcome, Outcome::Complete);
    assert_eq!(result.body, b"close-delimited");
    Ok(())
}
#[tokio::test]
async fn response_header_count_and_byte_limits() -> TestResult {
    let mut wire = String::from("HTTP/1.1 200 OK\r\n");
    for _ in 0..101 {
        wire.push_str("X-A: b\r\n");
    }
    wire.push_str("\r\n");
    let (result, _) = response(wire.into_bytes()).await?;
    assert!(matches!(result.outcome, Outcome::Failed(_)));
    assert_eq!(result.status, None);
    let (result, _) = response(
        format!(
            "HTTP/1.1 200 OK\r\nX-Big: {}\r\n\r\n",
            "x".repeat(HEADER_LIMIT)
        )
        .into_bytes(),
    )
    .await?;
    assert!(matches!(result.outcome, Outcome::Failed(_)));
    Ok(())
}
#[tokio::test(start_paused = true)]
async fn absolute_deadline_keeps_partial_data_and_does_not_restart() -> TestResult {
    let start = tokio::time::Instant::now();
    let (cancel, cancelled) = oneshot::channel();
    let mut retained = vec![];
    let work = async {
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            retained.push(b'x');
        }
        Outcome::Complete
    };
    let outcome = wait_for(work, cancelled, start + TIME_LIMIT).await;
    assert_eq!(outcome, Outcome::TimedOut);
    assert_eq!(retained, b"xxxx");
    assert_eq!(tokio::time::Instant::now() - start, TIME_LIMIT);
    drop(cancel);
    let (_cancel, cancelled) = oneshot::channel();
    assert_eq!(
        wait_for(std::future::pending(), cancelled, start + TIME_LIMIT).await,
        Outcome::TimedOut
    );
    Ok(())
}
#[tokio::test(start_paused = true)]
async fn cancellation_precedes_simultaneous_completion_and_drops_work() -> TestResult {
    let (cancel, cancelled) = oneshot::channel();
    cancel.send(()).map_err(|_| "receiver gone")?;
    let dropped = Arc::new(std::sync::atomic::AtomicBool::new(false));
    struct Guard(Arc<std::sync::atomic::AtomicBool>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let guard = Guard(dropped.clone());
    let work = async move {
        let _guard = guard;
        Outcome::Complete
    };
    assert_eq!(
        wait_for(work, cancelled, tokio::time::Instant::now() + TIME_LIMIT).await,
        Outcome::Cancelled
    );
    assert!(dropped.load(std::sync::atomic::Ordering::SeqCst));
    Ok(())
}
#[tokio::test]
async fn cancel_disposes_the_live_socket() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let review = review(listener.local_addr()?);
    let (cancel, cancelled) = oneshot::channel();
    let (sent, received) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await?;
        read_head(&mut socket).await?;
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nabc")
            .await?;
        let _ = sent.send(());
        let mut byte = [0];
        match socket.read(&mut byte).await {
            Ok(0) => {}
            Err(e) if e.kind() == io::ErrorKind::ConnectionReset => {}
            other => panic!("connection was not closed: {other:?}"),
        }
        Ok::<_, io::Error>(())
    });
    // Poll the exchange and inspect application state at an explicit suspension point.
    let mut result = ReplayResult::empty(&review);
    {
        let exchange = exchange(&review, &mut result, configuration());
        tokio::pin!(exchange);
        tokio::select! { outcome = &mut exchange => panic!("unexpected {outcome:?}"), _ = received => {} }
        // Drive until the transport is pending, which incorporates already-ready frames.
        std::future::poll_fn(|cx| {
            assert!(exchange.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        cancel.send(()).map_err(|_| "receiver gone")?;
        assert_eq!(
            wait_for(
                exchange,
                cancelled,
                tokio::time::Instant::now() + TIME_LIMIT
            )
            .await,
            Outcome::Cancelled
        );
    }
    // Scheduling may deliver headers after the server signal; test exact partial retention separately.
    assert!(result.body.is_empty() || result.body == b"abc");
    tokio::time::timeout(TIME_LIMIT, server).await???;
    Ok(())
}
#[tokio::test]
async fn tls_verifies_trust_and_hostname() -> TestResult {
    use tokio_rustls::{
        TlsAcceptor,
        rustls::{ServerConfig, pki_types::PrivatePkcs8KeyDer},
    };
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let key = PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der());
    let server_config = Arc::new(
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert.cert.der().clone()], key.into())?,
    );
    let mut roots = RootCertStore::empty();
    roots.add(cert.cert.der().clone())?;
    let trusted = Arc::new(
        ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    );
    for (host, tls, expected) in [
        ("localhost", trusted.clone(), Outcome::Complete),
        (
            "127.0.0.1",
            trusted.clone(),
            Outcome::Failed("TLS certificate does not match the host name"),
        ),
        (
            "localhost",
            configuration(),
            Outcome::Failed("TLS certificate issuer is not trusted"),
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let review = ReplayReview {
            method: netscope_core::ReplayMethod::Get,
            format: netscope_core::BodyFormat::Plain,
            body: String::new(),
            source: RequestId(0),
            destination: format!("https://{host}:{port}/"),
            authority: format!("{host}:{port}"),
            fragment: None,
            headers: vec![],
        };
        let acceptor = TlsAcceptor::from(server_config.clone());
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await?;
            if let Ok(mut tls) = acceptor.accept(stream).await {
                read_head(&mut tls).await?;
                tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                    .await?;
            }
            Ok::<_, io::Error>(())
        });
        let (_cancel, cancelled) = oneshot::channel();
        let result = execute(
            &review,
            cancelled,
            tokio::time::Instant::now() + TIME_LIMIT,
            tls,
        )
        .await;
        assert_eq!(result.outcome, expected);
        tokio::time::timeout(TIME_LIMIT, server).await???;
    }
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn retained_bytes_survive_deadlines_including_exact_cap_without_eof() -> TestResult {
    struct StalledBody(Option<Bytes>);
    impl hyper::body::Body for StalledBody {
        type Data = Bytes;
        type Error = io::Error;
        fn poll_frame(
            mut self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Result<hyper::body::Frame<Bytes>, io::Error>>> {
            match self.0.take() {
                Some(bytes) => std::task::Poll::Ready(Some(Ok(hyper::body::Frame::data(bytes)))),
                None => std::task::Poll::Pending,
            }
        }
    }
    for size in [0, 3, BODY_LIMIT] {
        let mut bytes = vec![];
        let (_cancel, cancelled) = oneshot::channel();
        let work = async {
            retain_body(StalledBody(Some(Bytes::from(vec![b'a'; size]))), &mut bytes)
                .await
                .unwrap_or(Outcome::Failed("read"))
        };
        assert_eq!(
            wait_for(work, cancelled, tokio::time::Instant::now() + TIME_LIMIT).await,
            Outcome::TimedOut
        );
        assert_eq!(bytes.len(), size);
    }
    Ok(())
}
#[test]
fn combined_outgoing_budget_and_managed_fields_are_checked_without_io() -> TestResult {
    let mut r = review("127.0.0.1:1".parse()?);
    r.headers = vec![
        netscope_core::NameValue {
            name: "X-A".into(),
            value: "x".repeat(8192)
        };
        4
    ];
    assert_eq!(validate(&r), Err("Outgoing headers exceed 32 KiB"));
    r.headers = vec![netscope_core::NameValue {
        name: "Host".into(),
        value: "elsewhere".into(),
    }];
    assert!(validate(&r).is_err());
    r.headers.clear();
    r.authority = "elsewhere".into();
    assert!(validate(&r).is_err());
    Ok(())
}
#[test]
fn worker_cancellation_and_channel_closure_finish_without_late_results() -> TestResult {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let mut worker = Worker::start(review(listener.local_addr()?))?;
    let (ready, received) = mpsc::sync_channel(1);
    let server = std::thread::spawn(move || -> io::Result<()> {
        let (mut stream, _) = listener.accept()?;
        stream.set_read_timeout(Some(TIME_LIMIT))?;
        let mut head = vec![];
        let mut byte = [0];
        use std::io::Read;
        while !head.ends_with(b"\r\n\r\n") {
            if stream.read(&mut byte)? == 0 {
                return Ok(());
            }
            head.push(byte[0]);
        }
        ready.send(()).map_err(io::Error::other)?;
        assert_eq!(stream.read(&mut byte)?, 0);
        Ok(())
    });
    received.recv_timeout(TIME_LIMIT)?;
    worker.cancel();
    drop(worker);
    server.join().map_err(|_| "server panicked")??;
    let (sender, receiver) = mpsc::sync_channel(1);
    drop(sender);
    let mut worker = Worker {
        cancel: None,
        result: receiver,
        thread: None,
        fallback: Some(ReplayResult::empty(&review("127.0.0.1:1".parse()?))),
        progress: None,
    };
    assert!(matches!(
        worker.poll().map(|r| r.outcome),
        Some(Outcome::Failed(_))
    ));
    assert!(worker.poll().is_none());
    Ok(())
}

#[tokio::test]
async fn unknown_length_and_chunked_bodies_obey_the_same_cap() -> TestResult {
    for chunked in [false, true] {
        for size in [BODY_LIMIT, BODY_LIMIT + 1] {
            let mut wire = if chunked {
                format!("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{size:x}\r\n")
                    .into_bytes()
            } else {
                b"HTTP/1.0 200 OK\r\n\r\n".to_vec()
            };
            wire.extend(std::iter::repeat_n(b'a', size));
            if chunked {
                wire.extend_from_slice(b"\r\n0\r\n\r\n");
            }
            let (result, _) = response(wire).await?;
            assert_eq!(
                result.outcome,
                if size == BODY_LIMIT {
                    Outcome::Complete
                } else {
                    Outcome::SizeLimited
                }
            );
            assert_eq!(result.body.len(), BODY_LIMIT);
        }
    }
    Ok(())
}
#[tokio::test]
async fn cancellation_before_headers_preserves_absence() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let review = review(listener.local_addr()?);
    let (cancel, cancelled) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await?;
        read_head(&mut socket).await?;
        let _ = cancel.send(());
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).await?, 0);
        Ok::<_, io::Error>(())
    });
    let result = execute(
        &review,
        cancelled,
        tokio::time::Instant::now() + TIME_LIMIT,
        configuration(),
    )
    .await;
    assert_eq!(result.outcome, Outcome::Cancelled);
    assert_eq!(result.status, None);
    assert!(result.body.is_empty());
    tokio::time::timeout(TIME_LIMIT, server).await???;
    Ok(())
}
#[tokio::test(start_paused = true)]
async fn cancellation_retains_data_already_copied_by_the_body_reader() -> TestResult {
    struct CancelAfterData {
        first: bool,
        cancel: Option<oneshot::Sender<()>>,
    }
    impl hyper::body::Body for CancelAfterData {
        type Data = Bytes;
        type Error = io::Error;
        fn poll_frame(
            mut self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Result<hyper::body::Frame<Bytes>, io::Error>>> {
            if self.first {
                self.first = false;
                return std::task::Poll::Ready(Some(Ok(hyper::body::Frame::data(
                    Bytes::from_static(b"abc"),
                ))));
            }
            if let Some(cancel) = self.cancel.take() {
                let _ = cancel.send(());
            }
            std::task::Poll::Pending
        }
    }
    let (cancel, cancelled) = oneshot::channel();
    let mut retained = vec![];
    let work = async {
        retain_body(
            CancelAfterData {
                first: true,
                cancel: Some(cancel),
            },
            &mut retained,
        )
        .await
        .unwrap_or(Outcome::Failed("read"))
    };
    assert_eq!(
        wait_for(work, cancelled, tokio::time::Instant::now() + TIME_LIMIT).await,
        Outcome::Cancelled
    );
    assert_eq!(retained, b"abc");
    Ok(())
}

#[tokio::test]
async fn redirects_and_disconnects_never_open_a_second_connection() -> TestResult {
    for redirect in [true, false] {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let review = review(listener.local_addr()?);
        let location = format!("http://{}/other", listener.local_addr()?);
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await?;
            read_head(&mut socket).await?;
            if redirect {
                socket.write_all(format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\n\r\n").as_bytes()).await?;
            }
            drop(socket);
            Ok::<_, io::Error>(listener)
        });
        let (_cancel, cancelled) = oneshot::channel();
        let result = execute(
            &review,
            cancelled,
            tokio::time::Instant::now() + TIME_LIMIT,
            configuration(),
        )
        .await;
        assert_eq!(result.outcome == Outcome::Complete, redirect);
        let listener = tokio::time::timeout(TIME_LIMIT, server)
            .await???
            .into_std()?;
        assert!(matches!(listener.accept(),Err(e) if e.kind() == io::ErrorKind::WouldBlock));
    }
    Ok(())
}
#[test]
fn an_expired_send_deadline_does_not_open_a_connection() -> TestResult {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let mut worker = Worker::start_at(review(listener.local_addr()?), Instant::now() - TIME_LIMIT)?;
    let result = worker.result.recv_timeout(TIME_LIMIT)?;
    worker.fallback = None;
    assert_eq!(result.outcome, Outcome::TimedOut);
    assert_eq!(result.status, None);
    assert!(matches!(listener.accept(),Err(e) if e.kind() == io::ErrorKind::WouldBlock));
    Ok(())
}

#[tokio::test]
async fn post_sends_exact_utf8_bytes_and_managed_headers() -> TestResult {
    for (format, body) in [
        (netscope_core::BodyFormat::Plain, ""),
        (netscope_core::BodyFormat::Plain, "café\n🙂"),
        (
            netscope_core::BodyFormat::Json,
            " {\"message\":\"café\",\"n\":1.00} \n",
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let mut review = review(listener.local_addr()?);
        review.method = netscope_core::ReplayMethod::Post;
        review.format = format;
        review.body = body.into();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let head = read_head(&mut stream).await?;
            let mut bytes = vec![0; body.len()];
            stream.read_exact(&mut bytes).await?;
            assert!(head.starts_with("POST /a?q=1 HTTP/1.1\r\n"));
            assert!(head.contains(&format!("content-length: {}\r\n", body.len())));
            assert!(head.contains(&format!("content-type: {}\r\n", format.content_type())));
            assert!(!head.contains("transfer-encoding:"));
            assert_eq!(bytes, body.as_bytes());
            stream
                .write_all(b"HTTP/1.1 201 Created\r\nContent-Length: 2\r\n\r\nok")
                .await?;
            Ok::<_, io::Error>(())
        });
        let (_cancel, cancelled) = oneshot::channel();
        let result = execute(
            &review,
            cancelled,
            tokio::time::Instant::now() + TIME_LIMIT,
            configuration(),
        )
        .await;
        assert_eq!(result.outcome, Outcome::Complete);
        assert_eq!(result.status, Some(201));
        assert_eq!(result.method, netscope_core::ReplayMethod::Post);
        assert_eq!(result.request_body_bytes, body.len());
        tokio::time::timeout(TIME_LIMIT, server).await???;
    }
    Ok(())
}
#[tokio::test(start_paused = true)]
async fn cancellation_and_deadline_drop_a_backpressured_upload() -> TestResult {
    for cancel_now in [true, false] {
        let mut review = review("127.0.0.1:1".parse()?);
        review.method = netscope_core::ReplayMethod::Post;
        review.body = "x".repeat(netscope_core::REQUEST_BODY_LIMIT);
        let (_, request) = request(&review)?;
        let (client, mut server) = tokio::io::duplex(1);
        let (cancel, cancelled) = oneshot::channel();
        let (finished, finish) = oneshot::channel();
        let peer = tokio::spawn(async move {
            read_head(&mut server).await?;
            assert_eq!(server.read_u8().await?, b'x');
            if cancel_now {
                let _ = cancel.send(());
            } else {
                let _hold = cancel;
                finish.await.map_err(io::Error::other)?;
            }
            let mut remainder = vec![];
            server.read_to_end(&mut remainder).await?;
            assert!(remainder.len() < netscope_core::REQUEST_BODY_LIMIT - 1);
            Ok::<_, io::Error>(())
        });
        let mut result = ReplayResult::empty(&review);
        let work = async {
            exchange_on(Box::new(client), request, &mut result)
                .await
                .unwrap_or(Outcome::Failed("exchange"))
        };
        let outcome = wait_for(work, cancelled, tokio::time::Instant::now() + TIME_LIMIT).await;
        assert_eq!(
            outcome,
            if cancel_now {
                Outcome::Cancelled
            } else {
                Outcome::TimedOut
            }
        );
        assert_eq!(result.status, None);
        let _ = finished.send(());
        peer.await??;
    }
    Ok(())
}
#[test]
fn transport_rejects_invalid_bodies_without_io() -> TestResult {
    let mut review = review("127.0.0.1:1".parse()?);
    review.body = "unexpected GET body".into();
    assert!(validate(&review).is_err());
    review.method = netscope_core::ReplayMethod::Post;
    review.format = netscope_core::BodyFormat::Json;
    assert!(validate(&review).is_err());
    review.body = "null".into();
    assert!(validate(&review).is_ok());
    review.body = "0".repeat(netscope_core::REQUEST_BODY_LIMIT + 1);
    assert!(validate(&review).is_err());
    Ok(())
}

#[tokio::test]
async fn live_events_are_published_before_eof_and_cancel_disposes_the_socket() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let review = review(listener.local_addr()?);
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await?;
        let head = read_head(&mut socket).await?;
        assert!(head.lines().any(|line| line == "accept: text/event-stream"));
        assert!(head.lines().any(|line| line == "accept-encoding: identity"));
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nContent-Length: 1000\r\n\r\nid: first\ndata: caf\xc3").await?;
        socket.write_all(b"\xa9\n\ndata: unfinished\n").await?;
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).await?, 0);
        Ok::<_, io::Error>(())
    });
    let (tx, mut rx) = tokio::sync::watch::channel(None);
    let mut capture = streaming::LiveCapture::new(Instant::now(), tx);
    let mut result = ReplayResult::empty(&review);
    let (cancel, cancelled) = oneshot::channel();
    let operation = wait_for(
        exchange_mode(&review, &mut result, configuration(), Some(&mut capture)),
        cancelled,
        tokio::time::Instant::now() + STREAM_TIME_LIMIT,
    );
    tokio::pin!(operation);
    loop {
        tokio::select! {
            outcome = &mut operation => return Err(format!("Stopped before observation: {outcome:?}").into()),
            changed = rx.changed() => {
                changed?;
                if rx.borrow().as_ref().is_some_and(|s| s.observed.len() == 1) { break; }
            }
        }
    }
    let snapshot = rx.borrow().clone().ok_or("Missing progress")?;
    assert!(
        snapshot
            .body
            .starts_with(b"id: first\ndata: caf\xc3\xa9\n\n")
    );
    cancel.send(()).map_err(|_| "Cancel channel closed")?;
    assert_eq!(operation.await, Outcome::Cancelled);
    tokio::time::timeout(TIME_LIMIT, server).await???;
    Ok(())
}

#[tokio::test]
async fn live_response_validation_and_utf8_eof_are_explicit() -> TestResult {
    for (head, expected) in [
        (
            "HTTP/1.1 302 Found\r\nContent-Type: text/event-stream\r\n",
            "Live SSE requires HTTP status 200",
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n",
            "Live SSE requires one text/event-stream Content-Type",
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Type: text/event-stream\r\n",
            "Live SSE requires one text/event-stream Content-Type",
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Encoding: gzip\r\n",
            "Live SSE does not decode compressed content",
        ),
    ] {
        let (client, mut server) = tokio::io::duplex(4096);
        let task = tokio::spawn(async move {
            read_head(&mut server).await?;
            server
                .write_all(format!("{head}Content-Length: 0\r\n\r\n").as_bytes())
                .await
        });
        let review = review("127.0.0.1:1".parse()?);
        let (_, request) = request(&review)?;
        let mut result = ReplayResult::empty(&review);
        let (tx, _rx) = tokio::sync::watch::channel(None);
        let mut capture = streaming::LiveCapture::new(Instant::now(), tx);
        assert_eq!(
            exchange_on_mode(Box::new(client), request, &mut result, Some(&mut capture)).await,
            Err(expected)
        );
        task.await??;
        assert!(result.status.is_some());
        assert!(result.body.is_empty());
    }
    let (tx, _rx) = tokio::sync::watch::channel(None);
    let mut capture = streaming::LiveCapture::new(Instant::now(), tx);
    let mut body = vec![];
    assert_eq!(
        capture
            .retain(Full::new(Bytes::from_static(b"data: x\n\n\xc3")), &mut body)
            .await,
        Err("Live SSE ended inside a UTF-8 character")
    );
    assert_eq!(body, b"data: x\n\n\xc3");
    Ok(())
}

#[test]
fn live_validation_rejects_post_and_selected_accept_without_io() -> TestResult {
    let mut review = review("127.0.0.1:1".parse()?);
    assert!(validate_stream(&review).is_ok());
    review.method = netscope_core::ReplayMethod::Post;
    assert_eq!(validate_stream(&review), Err("Live SSE requires GET"));
    review.method = netscope_core::ReplayMethod::Get;
    review.headers.push(netscope_core::NameValue {
        name: "Accept".into(),
        value: "application/json".into(),
    });
    assert!(validate_stream(&review).is_err());
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn live_deadline_is_thirty_seconds_and_does_not_restart_on_progress() {
    let (_cancel, cancelled) = oneshot::channel();
    let start = tokio::time::Instant::now();
    let work = async {
        tokio::time::sleep(Duration::from_secs(29)).await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        Outcome::Complete
    };
    assert_eq!(
        wait_for(work, cancelled, start + STREAM_TIME_LIMIT).await,
        Outcome::TimedOut
    );
    assert_eq!(tokio::time::Instant::now() - start, Duration::from_secs(30));
}

#[tokio::test]
async fn live_empty_and_unfinished_bodies_keep_transport_and_event_completion_distinct()
-> TestResult {
    for bytes in [b"".as_slice(), b"data: unfinished\n"] {
        let (tx, rx) = tokio::sync::watch::channel(None);
        let mut capture = streaming::LiveCapture::new(Instant::now(), tx);
        let mut retained = vec![];
        assert_eq!(
            capture
                .retain(Full::new(Bytes::copy_from_slice(bytes)), &mut retained)
                .await?,
            Outcome::Complete
        );
        assert_eq!(retained, bytes);
        let snapshot = rx.borrow().clone().ok_or("Missing empty publication")?;
        assert!(snapshot.observed.is_empty());
        assert_eq!(snapshot.valid_bytes, bytes.len());
    }
    Ok(())
}
