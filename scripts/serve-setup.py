#!/usr/bin/env python3
"""
Simple HTTP server to serve the Syla setup script with basic authentication.
For production, use a proper web server with authentication.

Usage:
    python3 serve-setup.py [port]
"""

import http.server
import socketserver
import base64
import sys
import os
from functools import partial

# Configuration
DEFAULT_PORT = 8443
REALM = "Syla Setup"
# In production, store these securely
USERS = {
    "dev": "syla-dev-2024",  # Change these!
    "admin": "syla-admin-secure"
}

class AuthHandler(http.server.SimpleHTTPRequestHandler):
    """HTTP handler with basic authentication."""
    
    def __init__(self, *args, directory=None, **kwargs):
        self.setup_directory = directory or os.path.dirname(os.path.abspath(__file__))
        super().__init__(*args, directory=self.setup_directory, **kwargs)
    
    def do_GET(self):
        """Handle GET requests with authentication."""
        auth_header = self.headers.get('Authorization')
        
        if not self.authenticate(auth_header):
            self.send_auth_request()
            return
        
        # Only serve setup.sh
        if self.path == '/' or self.path == '/setup.sh':
            self.path = '/setup.sh'
            super().do_GET()
        else:
            self.send_error(404, "Not Found")
    
    def authenticate(self, auth_header):
        """Check if the authorization header is valid."""
        if not auth_header:
            return False
        
        try:
            auth_type, credentials = auth_header.split(' ', 1)
            if auth_type.lower() != 'basic':
                return False
            
            decoded = base64.b64decode(credentials).decode('utf-8')
            username, password = decoded.split(':', 1)
            
            return USERS.get(username) == password
        except:
            return False
    
    def send_auth_request(self):
        """Send 401 response requesting authentication."""
        self.send_response(401)
        self.send_header('WWW-Authenticate', f'Basic realm="{REALM}"')
        self.send_header('Content-type', 'text/html')
        self.end_headers()
        self.wfile.write(b'Authentication required.')
    
    def log_message(self, format, *args):
        """Override to add more detailed logging."""
        sys.stderr.write(f"[{self.address_string()}] {format % args}\n")

def main():
    """Run the authentication server."""
    port = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_PORT
    
    print(f"Starting Syla setup server on port {port}")
    print(f"Setup URL: http://localhost:{port}/setup.sh")
    print("\nExample usage:")
    print(f"  curl -u dev:password http://localhost:{port}/setup.sh | sh")
    print(f"  curl -u dev:password http://localhost:{port}/setup.sh | sh -s -- --path /custom/path")
    print("\nPress Ctrl+C to stop the server")
    
    handler = partial(AuthHandler, directory=os.path.dirname(os.path.abspath(__file__)))
    
    with socketserver.TCPServer(("", port), handler) as httpd:
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nShutting down server...")
            httpd.shutdown()

if __name__ == "__main__":
    main()