#!/usr/bin/env python3
"""A loopback-only server for the book's POST walkthrough."""

import argparse
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


class EchoHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        self.connection.settimeout(10)
        if self.headers.get("Transfer-Encoding"):
            self.send_error(400, "Use Content-Length for this example")
            return
        try:
            length = int(self.headers.get("Content-Length", "-1"))
            if not 0 <= length <= 16_384:
                raise ValueError
        except ValueError:
            self.send_error(400, "Expected Content-Length from 0 to 16384")
            return
        try:
            body = self.rfile.read(length)
            if len(body) != length:
                raise ValueError
            text = body.decode("utf-8")
        except (OSError, ValueError):
            self.send_error(400, "Expected the complete UTF-8 body")
            return
        response = json.dumps(
            {
                "method": "POST",
                "body_bytes": len(body),
                "content_type": self.headers.get("Content-Type"),
                "text": text,
            },
            ensure_ascii=False,
            indent=2,
        ).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(response)))
        self.end_headers()
        self.wfile.write(response)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8080)
    args = parser.parse_args()
    with ThreadingHTTPServer(("127.0.0.1", args.port), EchoHandler) as server:
        print(f"POST echo ready at http://127.0.0.1:{server.server_port}/books", flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    main()
