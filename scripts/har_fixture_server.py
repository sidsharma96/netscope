"""Serve synthetic loopback traffic for manually recording browser HAR files."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PAGE = b'''<!doctype html><meta charset="utf-8"><title>Netscope HAR compatibility</title>
<script>
setTimeout(async () => {
  for (const [url, options] of [
    ['/text', {}],
    ['/json', {method: 'POST', headers: {'Content-Type': 'application/json'},
               body: '{"title":"caf\\u00e9"}'}],
    ['/form', {method: 'POST', headers: {'Content-Type': 'application/x-www-form-urlencoded'},
               body: 'tag=one&tag=two&empty='}],
    ['/binary', {}], ['/redirect', {}], ['/missing', {}]
  ]) {
    await fetch(url, options).then(response => response.arrayBuffer());
  }
  document.title = 'Netscope capture complete';
}, 3000);
</script>Local synthetic requests only.'''


class Handler(BaseHTTPRequestHandler):
    def reply(self, status, mime, body, cookie=False):
        self.send_response(status)
        self.send_header('Content-Type', mime)
        self.send_header('Content-Length', str(len(body)))
        self.send_header('Cache-Control', 'no-store')
        if cookie:
            self.send_header('Set-Cookie', 'netscope_fixture=synthetic; Path=/; SameSite=Lax')
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == '/redirect':
            self.send_response(302)
            self.send_header('Location', '/text')
            self.send_header('Content-Length', '0')
            self.end_headers()
            return
        responses = {
            '/': (200, 'text/html; charset=utf-8', PAGE),
            '/text': (200, 'text/plain; charset=utf-8', 'hello café'.encode()),
            '/binary': (200, 'application/octet-stream', bytes([0, 255, 1, 254])),
        }
        self.reply(*responses.get(self.path, (404, 'text/plain', b'not found')), cookie=True)

    def do_POST(self):
        length = int(self.headers.get('Content-Length', '0'))
        if not 0 <= length <= 16 * 1024:
            self.send_error(413)
            return
        body = self.rfile.read(length)
        self.reply(200, self.headers.get('Content-Type', 'application/octet-stream'), body)


if __name__ == '__main__':
    with ThreadingHTTPServer(('127.0.0.1', 0), Handler) as server:
        print(f'Open http://127.0.0.1:{server.server_port}/ with the Network panel recording.', flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
