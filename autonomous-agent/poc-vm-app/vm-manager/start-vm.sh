#!/bin/bash
# VM Manager - Simulates starting a VM with Luna

echo "🔧 VM Manager: Initializing Luna environment..."

# Simulate VM operations
sleep 2
echo "   📦 Loading Luna VM image..."
sleep 3
echo "   🌐 Configuring networking..."
sleep 2
echo "   🚀 Starting Luna services..."

# Simulate Luna agent starting
cat > /tmp/mock-luna-health << 'HEALTH_EOF'
{
  "status": "ready",
  "version": "1.0.0",
  "agent": "Luna",
  "uptime": "just started"
}
HEALTH_EOF

# Start mock Luna server
python3 -c "
import http.server
import socketserver
import json
import threading
import time

class MockLunaHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/health':
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            with open('/tmp/mock-luna-health', 'r') as f:
                self.wfile.write(f.read().encode())
        elif self.path == '/':
            self.send_response(200)
            self.send_header('Content-type', 'text/html')
            self.end_headers()
            html = '''<!DOCTYPE html>
<html>
<head><title>Luna Agent (VM Mode)</title></head>
<body style=\"font-family: Arial, sans-serif; max-width: 800px; margin: 50px auto; text-align: center;\">
    <h1>🌙 Luna Agent</h1>
    <p><strong>Running in Virtual Machine</strong></p>
    <p>This would be the full Luna dashboard interface.</p>
    <p>VM simulation - normally this would be the actual Luna agent running in a Linux VM.</p>
    <div style=\"background: #f0f0f0; padding: 20px; margin: 20px; border-radius: 10px;\">
        <h3>VM Status: ✅ Running</h3>
        <p>Port: 8080</p>
        <p>Memory: 1GB allocated</p>
        <p>OS: Ubuntu 22.04 (simulated)</p>
    </div>
</body>
</html>'''
            self.wfile.write(html.encode())
        else:
            super().do_GET()

PORT = 8080
with socketserver.TCPServer(('', PORT), MockLunaHandler) as httpd:
    print(f'   ✅ Luna VM ready on port {PORT}')
    httpd.serve_forever()
" &

echo "   ✅ VM startup complete"
