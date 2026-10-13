"""Finite loopback SSE and a classic WebSocket handshake for browser exports."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import base64
import hashlib
import socket
import time

EVENTS = '\ufeff: fixture\n\nid: book-1\nevent: added\ndata: café\ndata: shelf 3\nretry: 1500\n\ndata: another book\n\nevent: done\ndata: [DONE]\n\n'.encode()
PAGE = b'''<!doctype html><meta charset="utf-8"><link rel="icon" href="data:,">
<title>Netscope streaming fixture</title><script>
setTimeout(async () => {
 await Promise.all([
  new Promise(resolve => {const s = new EventSource('/events');
    s.addEventListener('done', () => {s.close(); resolve();});}),
  new Promise(resolve => {const w = new WebSocket('ws://' + location.host + '/socket');
    w.onmessage = () => {w.close(); resolve();};})
 ]);
 document.title = 'Netscope capture complete';
}, 500);
</script>Only synthetic loopback streams.'''

class Handler(BaseHTTPRequestHandler):
    protocol_version = 'HTTP/1.1'
    def do_GET(self):
        if self.path == '/socket':
            key = self.headers.get('Sec-WebSocket-Key', '')
            accept = base64.b64encode(hashlib.sha1((key + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').encode()).digest()).decode()
            self.send_response(101)
            self.send_header('Upgrade', 'websocket')
            self.send_header('Connection', 'Upgrade')
            self.send_header('Sec-WebSocket-Accept', accept)
            self.end_headers()
            self.wfile.write(b'\x81\x11synthetic message')
            self.wfile.flush()
            self.connection.settimeout(2)
            try:
                self.connection.recv(1024)
                self.wfile.write(b'\x88\x00')
                self.wfile.flush()
            except (OSError, socket.timeout):
                pass
            self.close_connection = True
            return
        body = EVENTS if self.path == '/events' else PAGE if self.path == '/' else b'not found'
        self.send_response(200 if self.path in ('/', '/events') else 404)
        self.send_header('Content-Type', 'text/event-stream; charset=utf-8' if self.path == '/events' else 'text/html; charset=utf-8')
        self.send_header('Content-Length', str(len(body)))
        self.send_header('Cache-Control', 'no-store')
        self.end_headers()
        try:
            if self.path == '/events':
                for part in (body[:1], body[1:7], body[7:70], body[70:]):
                    self.wfile.write(part)
                    self.wfile.flush()
                    time.sleep(.05)
            else:
                self.wfile.write(body)
        except (BrokenPipeError, ConnectionResetError):
            pass

if __name__ == '__main__':
    with ThreadingHTTPServer(('127.0.0.1', 0), Handler) as server:
        print(f'http://127.0.0.1:{server.server_port}/', flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
