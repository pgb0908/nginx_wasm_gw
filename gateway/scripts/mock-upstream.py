#!/usr/bin/env python3
"""Mock upstream servers for integration testing.

svc_v1: 127.0.0.1:8081
svc_v2: 127.0.0.1:8082
"""
import http.server
import threading
import signal
import sys


class V1Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        body = b"svc_v1\n"
        self.send_response(200)
        self.send_header("Content-Length", len(body))
        self.send_header("Server", "mock-v1")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


class V2Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        body = b"svc_v2\n"
        self.send_response(200)
        self.send_header("Content-Length", len(body))
        self.send_header("Server", "mock-v2")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


v1 = http.server.HTTPServer(("127.0.0.1", 8081), V1Handler)
v2 = http.server.HTTPServer(("127.0.0.1", 8082), V2Handler)

threading.Thread(target=v1.serve_forever, daemon=True).start()
threading.Thread(target=v2.serve_forever, daemon=True).start()

print("mock upstream ready: svc_v1=127.0.0.1:8081, svc_v2=127.0.0.1:8082", flush=True)

signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
signal.pause()
