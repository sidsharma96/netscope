/** Controlled HTTPS/H2 responses for manually recording native browser exports. */
import {createSecureServer} from 'node:http2';
import {readFileSync} from 'node:fs';
import {gzipSync, brotliCompressSync} from 'node:zlib';

const [keyPath, certPath] = process.argv.slice(2);
if (!keyPath || !certPath) {
  console.error('Usage: node scripts/har_https_fixture_server.mjs <key.pem> <cert.pem>');
  process.exit(2);
}
const text = Buffer.from('A book about café.\n'.repeat(64));
const binary = Buffer.from([0, 255, 1, 254]);
const page = Buffer.from(`<!doctype html><meta charset="utf-8">
<title>Netscope HTTPS compatibility</title><link rel="icon" href="data:,">
<script>
setTimeout(async () => {
  for (const path of ['/plain', '/gzip', '/br', '/gzip-binary', '/plain?again=1']) {
    await fetch(path).then(response => response.arrayBuffer());
  }
  document.title = 'Netscope capture complete';
}, 1000);
</script>Controlled HTTPS responses only.`);
const responses = new Map([
  ['/', [page, page, 'text/html; charset=utf-8', undefined]],
  ['/plain', [text, text, 'text/plain; charset=utf-8', undefined]],
  ['/gzip', [text, gzipSync(text), 'text/plain; charset=utf-8', 'gzip']],
  ['/br', [text, brotliCompressSync(text), 'text/plain; charset=utf-8', 'br']],
  ['/gzip-binary', [binary, gzipSync(binary), 'application/octet-stream', 'gzip']],
]);
const server = createSecureServer({key: readFileSync(keyPath), cert: readFileSync(certPath), allowHTTP1: true});
const sessions = new WeakMap();
let nextSession = 0;
server.on('session', session => sessions.set(session, ++nextSession));
server.on('request', (request, response) => {
  const path = request.url.split('?')[0];
  const found = responses.get(path);
  const [decoded, encoded, mime, encoding] = found ?? [Buffer.from('not found'), Buffer.from('not found'), 'text/plain', undefined];
  const headers = {'content-type': mime, 'content-length': encoded.length, 'cache-control': 'no-store'};
  if (encoding) headers['content-encoding'] = encoding;
  console.log(JSON.stringify({path: request.url, httpVersion: request.httpVersion,
    alpn: request.socket.alpnProtocol, session: sessions.get(request.stream?.session) ?? null,
    encoding: encoding ?? 'identity', decodedBytes: decoded.length, encodedBytes: encoded.length}));
  response.writeHead(found ? 200 : 404, headers);
  response.end(encoded);
});
server.listen(0, '127.0.0.1', () => console.log(`https://127.0.0.1:${server.address().port}/`));
