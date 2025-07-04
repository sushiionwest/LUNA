#!/usr/bin/env /home/scrapybara/.bun/bin/bun
import express from 'express';
import { createServer } from 'http';
import { Server } from 'socket.io';
import path from 'path';
import dotenv from 'dotenv';

// Load environment
dotenv.config({ path: '.env.development' });

const app = express();
const server = createServer(app);
const io = new Server(server, {
  cors: {
    origin: "*",
    methods: ["GET", "POST"]
  }
});

const PORT = process.env.WEB_PORT || 8080;

// Middleware
app.use(express.json());
app.use(express.static('public'));

// Basic routes
app.get('/', (req, res) => {
  res.send(`
    <html>
      <head><title>🌙 Luna Agent - Development</title></head>
      <body style="font-family: Arial, sans-serif; max-width: 800px; margin: 50px auto; text-align: center;">
        <h1>🌙 Luna Agent</h1>
        <p><strong>Development Environment Ready!</strong></p>
        <p>Luna is running in development mode.</p>
        <div style="background: #f0f0f0; padding: 20px; margin: 20px; border-radius: 10px;">
          <h3>✅ VM Status: Running</h3>
          <p>Environment: Ubuntu 22.04</p>
          <p>Node.js: ${process.version}</p>
          <p>Mode: ${process.env.NODE_ENV}</p>
        </div>
        <div style="margin-top: 30px;">
          <h3>🚀 Ready for Luna Development!</h3>
          <p>Start building your autonomous agent here.</p>
        </div>
      </body>
    </html>
  `);
});

app.get('/health', (req, res) => {
  res.json({
    status: 'ok',
    environment: 'development',
    version: '1.0.0',
    uptime: process.uptime(),
    memory: process.memoryUsage()
  });
});

// Socket.io for real-time communication
io.on('connection', (socket) => {
  console.log('Client connected:', socket.id);
  
  socket.emit('luna-status', {
    status: 'ready',
    environment: 'development',
    message: 'Luna development environment is ready!'
  });
  
  socket.on('disconnect', () => {
    console.log('Client disconnected:', socket.id);
  });
});

// Start server
server.listen(PORT, '0.0.0.0', () => {
  console.log(`🌙 Luna Agent running on http://0.0.0.0:${PORT}`);
  console.log(`📊 Development mode active`);
  console.log(`🔗 Access from host: http://localhost:${PORT}`);
});
