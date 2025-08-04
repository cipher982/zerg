import http.server
import socketserver
import sys

class CacheControlHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        # Disable caching for HTML files to ensure CSP updates are loaded
        if self.path.endswith('.html') or self.path == '/':
            self.send_header('Cache-Control', 'no-cache, no-store, must-revalidate')
            self.send_header('Pragma', 'no-cache')
            self.send_header('Expires', '0')
        super().end_headers()

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8000
with socketserver.TCPServer(("", PORT), CacheControlHandler) as httpd:
    print(f"Serving at http://localhost:{PORT}")
    httpd.serve_forever()
