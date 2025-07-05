#!/usr/bin/env node

import express from 'express';
import { createServer } from 'http';
import { Server } from 'socket.io';
import sqlite3 from 'sqlite3';
import { v4 as uuidv4 } from 'uuid';
import cors from 'cors';
import path from 'path';
import { fileURLToPath } from 'url';
import participantRoutes from './participant-routes.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const app = express();
const server = createServer(app);
const io = new Server(server, {
  cors: {
    origin: "*",
    methods: ["GET", "POST"]
  }
});

// Database setup
const db = new sqlite3.Database(':memory:');

// Initialize database tables
db.serialize(() => {
  // Sessions table
  db.run(`CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    participant_id TEXT,
    session_type TEXT,
    status TEXT DEFAULT 'pending',
    start_time DATETIME DEFAULT CURRENT_TIMESTAMP,
    end_time DATETIME,
    vm_id TEXT,
    installer_version TEXT,
    metadata TEXT
  )`);

  // Events table for tracking user interactions
  db.run(`CREATE TABLE events (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    event_type TEXT,
    event_data TEXT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions (id)
  )`);

  // Luna VM instances table
  db.run(`CREATE TABLE luna_vms (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    status TEXT DEFAULT 'starting',
    vm_path TEXT,
    port INTEGER,
    pid INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions (id)
  )`);

  // Participants table for user testing program
  db.run(`CREATE TABLE participants (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL,
    role TEXT,
    experience TEXT,
    os TEXT,
    motivation TEXT,
    availability TEXT,
    newsletter INTEGER DEFAULT 0,
    testing_phase TEXT,
    status TEXT DEFAULT 'registered',
    session_type TEXT,
    scheduled_at DATETIME,
    notes TEXT,
    registered_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
  )`);

  console.log('✅ Database tables initialized');
});

// Middleware
app.use(cors());
app.use(express.json());
app.use(express.static('.'));

// Make database available to routes
app.set('database', db);

// Luna VM Manager Service
class LunaVMService {
  constructor() {
    this.activeVMs = new Map();
  }

  async startVM(sessionId) {
    const vmId = uuidv4();
    const vmPort = Math.floor(Math.random() * 1000) + 5000;
    
    console.log(`🚀 Starting Luna VM for session ${sessionId}`);
    
    // Simulate VM startup
    const vmData = {
      id: vmId,
      sessionId,
      status: 'starting',
      port: vmPort,
      pid: Math.floor(Math.random() * 10000) + 1000,
      startTime: new Date()
    };

    this.activeVMs.set(vmId, vmData);

    // Insert into database
    db.run(
      'INSERT INTO luna_vms (id, session_id, status, port, pid) VALUES (?, ?, ?, ?, ?)',
      [vmId, sessionId, 'starting', vmPort, vmData.pid]
    );

    // Simulate VM startup time
    setTimeout(() => {
      vmData.status = 'running';
      db.run('UPDATE luna_vms SET status = ? WHERE id = ?', ['running', vmId]);
      console.log(`✅ Luna VM ${vmId} is now running on port ${vmPort}`);
      
      // Notify session orchestrator
      io.emit('vm-status', { vmId, sessionId, status: 'running', port: vmPort });
    }, 2000);

    return vmData;
  }

  async stopVM(vmId) {
    const vm = this.activeVMs.get(vmId);
    if (vm) {
      console.log(`🛑 Stopping Luna VM ${vmId}`);
      vm.status = 'stopped';
      db.run('UPDATE luna_vms SET status = ? WHERE id = ?', ['stopped', vmId]);
      this.activeVMs.delete(vmId);
      return true;
    }
    return false;
  }

  getVM(vmId) {
    return this.activeVMs.get(vmId);
  }
}

// Session Orchestrator
class SessionOrchestrator {
  constructor(vmService) {
    this.vmService = vmService;
  }

  async startTestSession(participantId, sessionType = 'one-click-install') {
    const sessionId = uuidv4();
    
    console.log(`🧪 Starting test session ${sessionId} for participant ${participantId}`);

    // Create session record
    db.run(
      'INSERT INTO sessions (id, participant_id, session_type, status) VALUES (?, ?, ?, ?)',
      [sessionId, participantId, sessionType, 'starting']
    );

    // Start Luna VM for this session
    const vm = await this.vmService.startVM(sessionId);

    // Update session with VM info
    db.run(
      'UPDATE sessions SET vm_id = ?, status = ? WHERE id = ?',
      [vm.id, 'active', sessionId]
    );

    return {
      sessionId,
      vmId: vm.id,
      status: 'active',
      testingURL: `http://localhost:3000/test-session/${sessionId}`
    };
  }

  async endTestSession(sessionId) {
    return new Promise((resolve, reject) => {
      // Get VM ID for this session
      db.get('SELECT vm_id FROM sessions WHERE id = ?', [sessionId], async (err, row) => {
        if (err) {
          reject(err);
          return;
        }

        if (row && row.vm_id) {
          await this.vmService.stopVM(row.vm_id);
        }

        // Update session end time
        db.run(
          'UPDATE sessions SET status = ?, end_time = CURRENT_TIMESTAMP WHERE id = ?',
          ['completed', sessionId],
          (err) => {
            if (err) reject(err);
            else {
              console.log(`✅ Test session ${sessionId} completed`);
              resolve(true);
            }
          }
        );
      });
    });
  }

  logEvent(sessionId, eventType, eventData) {
    const eventId = uuidv4();
    db.run(
      'INSERT INTO events (id, session_id, event_type, event_data) VALUES (?, ?, ?, ?)',
      [eventId, sessionId, eventType, JSON.stringify(eventData)]
    );
    
    console.log(`📊 Event logged: ${eventType} for session ${sessionId}`);
    
    // Emit real-time event to connected clients
    io.emit('session-event', {
      sessionId,
      eventType,
      eventData,
      timestamp: new Date()
    });
  }
}

// Initialize services
const vmService = new LunaVMService();
const orchestrator = new SessionOrchestrator(vmService);

// Mount participant routes
app.use('/api/participants', participantRoutes);

// API Routes
app.get('/', (req, res) => {
  res.send(`
    <html>
      <head>
        <title>Luna Testing Infrastructure Demo</title>
        <style>
          body { font-family: Arial, sans-serif; margin: 40px; background: #f5f5f5; }
          .container { max-width: 800px; margin: 0 auto; background: white; padding: 30px; border-radius: 10px; }
          .luna-logo { font-size: 2em; color: #6366f1; margin-bottom: 20px; }
          .feature { margin: 15px 0; padding: 15px; background: #f8fafc; border-left: 4px solid #6366f1; }
          .status { padding: 10px; margin: 10px 0; border-radius: 5px; }
          .status.active { background: #dcfce7; color: #166534; }
          .button { background: #6366f1; color: white; padding: 12px 24px; border: none; border-radius: 6px; cursor: pointer; text-decoration: none; display: inline-block; margin: 5px; }
          .button:hover { background: #4f46e5; }
        </style>
      </head>
      <body>
        <div class="container">
          <div class="luna-logo">🌙 Luna Testing Infrastructure</div>
          <div class="status active">✅ Testing infrastructure is running</div>
          
          <h2>Integration Features</h2>
          <div class="feature">
            <strong>🚀 One-Click Installer Integration</strong><br>
            Seamlessly connects with Luna's VM-based installer for real-time monitoring
          </div>
          <div class="feature">
            <strong>📊 Real-Time Analytics</strong><br>
            Tracks installation progress, user interactions, and performance metrics
          </div>
          <div class="feature">
            <strong>🔄 Session Orchestration</strong><br>
            Manages VM lifecycle and coordinates testing sessions automatically
          </div>
          <div class="feature">
            <strong>💬 Live Event Streaming</strong><br>
            WebSocket-based real-time communication with Luna installer
          </div>
          
          <h2>Demo Actions</h2>
          <a href="/api/test-session/start" class="button">Start Test Session</a>
          <a href="/api/sessions" class="button">View Sessions</a>
          <a href="/api/events" class="button">View Events</a>
          <a href="/demo-installer" class="button">Demo Luna Installer</a>
        </div>
      </body>
    </html>
  `);
});

// Start a new test session
app.post('/api/test-session/start', async (req, res) => {
  try {
    const participantId = req.body.participantId || 'demo-participant-' + Date.now();
    const session = await orchestrator.startTestSession(participantId);
    res.json(session);
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

// End a test session
app.post('/api/test-session/:sessionId/end', async (req, res) => {
  try {
    await orchestrator.endTestSession(req.params.sessionId);
    res.json({ success: true });
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

// Get all sessions
app.get('/api/sessions', (req, res) => {
  db.all('SELECT * FROM sessions ORDER BY start_time DESC', (err, rows) => {
    if (err) {
      res.status(500).json({ error: err.message });
    } else {
      res.json(rows);
    }
  });
});

// Get all events
app.get('/api/events', (req, res) => {
  db.all(`
    SELECT e.*, s.participant_id 
    FROM events e 
    JOIN sessions s ON e.session_id = s.id 
    ORDER BY e.timestamp DESC 
    LIMIT 100
  `, (err, rows) => {
    if (err) {
      res.status(500).json({ error: err.message });
    } else {
      res.json(rows);
    }
  });
});

// Demo Luna Installer page
app.get('/demo-installer', (req, res) => {
  res.send(`
    <html>
      <head>
        <title>Luna One-Click Installer Demo</title>
        <script src="/socket.io/socket.io.js"></script>
        <style>
          body { font-family: Arial, sans-serif; margin: 0; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; min-height: 100vh; }
          .installer { max-width: 500px; margin: 50px auto; padding: 30px; background: rgba(255,255,255,0.1); border-radius: 20px; backdrop-filter: blur(10px); }
          .luna-logo { font-size: 3em; text-align: center; margin-bottom: 20px; }
          .tagline { text-align: center; margin-bottom: 30px; opacity: 0.9; }
          .install-button { width: 100%; padding: 15px; background: #10b981; border: none; border-radius: 10px; color: white; font-size: 1.2em; cursor: pointer; margin: 20px 0; }
          .install-button:hover { background: #059669; }
          .install-button:disabled { background: #6b7280; cursor: not-allowed; }
          .progress { width: 100%; height: 8px; background: rgba(255,255,255,0.2); border-radius: 4px; margin: 20px 0; overflow: hidden; }
          .progress-bar { height: 100%; background: #10b981; width: 0%; transition: width 0.3s; }
          .status { text-align: center; margin: 15px 0; min-height: 20px; }
          .logs { background: rgba(0,0,0,0.3); padding: 15px; border-radius: 8px; font-family: monospace; font-size: 0.9em; max-height: 200px; overflow-y: auto; margin-top: 20px; }
        </style>
      </head>
      <body>
        <div class="installer">
          <div class="luna-logo">🌙</div>
          <h1 style="text-align: center; margin: 0;">Luna</h1>
          <div class="tagline">Your AI that sees in the dark</div>
          
          <button id="installBtn" class="install-button">Install Luna</button>
          
          <div class="progress">
            <div id="progressBar" class="progress-bar"></div>
          </div>
          
          <div id="status" class="status">Ready to install</div>
          
          <div id="logs" class="logs" style="display: none;"></div>
        </div>

        <script>
          const socket = io();
          const installBtn = document.getElementById('installBtn');
          const progressBar = document.getElementById('progressBar');
          const status = document.getElementById('status');
          const logs = document.getElementById('logs');
          
          let currentSession = null;
          let progress = 0;

          // Luna Testing Client Integration
          class LunaTestingClient {
            constructor(sessionId) {
              this.sessionId = sessionId;
              this.socket = socket;
            }

            logEvent(eventType, eventData) {
              this.socket.emit('luna-installer-event', {
                sessionId: this.sessionId,
                eventType,
                eventData,
                timestamp: new Date().toISOString()
              });
            }

            updateProgress(percentage, message) {
              this.logEvent('installation-progress', { percentage, message });
            }

            logError(error) {
              this.logEvent('installation-error', { error: error.message || error });
            }

            logSuccess() {
              this.logEvent('installation-complete', { success: true });
            }
          }

          let testingClient = null;

          installBtn.addEventListener('click', async () => {
            // Start test session
            const response = await fetch('/api/test-session/start', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ participantId: 'demo-user' })
            });
            
            currentSession = await response.json();
            testingClient = new LunaTestingClient(currentSession.sessionId);
            
            installBtn.disabled = true;
            logs.style.display = 'block';
            
            // Simulate Luna installation process
            const steps = [
              { msg: 'Initializing Luna installer...', delay: 1000 },
              { msg: 'Downloading Luna VM image...', delay: 2000 },
              { msg: 'Extracting VM components...', delay: 1500 },
              { msg: 'Configuring Luna environment...', delay: 1000 },
              { msg: 'Installing dependencies...', delay: 2000 },
              { msg: 'Starting Luna VM...', delay: 1500 },
              { msg: 'Finalizing installation...', delay: 1000 }
            ];

            testingClient.logEvent('installation-started', { userAgent: navigator.userAgent });

            for (let i = 0; i < steps.length; i++) {
              const step = steps[i];
              progress = ((i + 1) / steps.length) * 100;
              
              status.textContent = step.msg;
              progressBar.style.width = progress + '%';
              logs.innerHTML += step.msg + '\\n';
              logs.scrollTop = logs.scrollHeight;
              
              testingClient.updateProgress(progress, step.msg);
              
              await new Promise(resolve => setTimeout(resolve, step.delay));
            }

            status.textContent = '✅ Luna installed successfully!';
            testingClient.logSuccess();
            
            setTimeout(() => {
              status.innerHTML = '🚀 <strong>Luna is ready!</strong> Your AI assistant is now running.';
            }, 1000);
          });

          // Listen for session events
          socket.on('session-event', (event) => {
            if (currentSession && event.sessionId === currentSession.sessionId) {
              console.log('Session event:', event);
            }
          });

          socket.on('vm-status', (data) => {
            if (currentSession && data.sessionId === currentSession.sessionId) {
              console.log('VM status update:', data);
            }
          });
        </script>
      </body>
    </html>
  `);
});

// Socket.IO for real-time communication
io.on('connection', (socket) => {
  console.log('📡 Client connected to testing infrastructure');

  // Handle events from Luna installer
  socket.on('luna-installer-event', (data) => {
    const { sessionId, eventType, eventData } = data;
    orchestrator.logEvent(sessionId, eventType, eventData);
  });

  socket.on('disconnect', () => {
    console.log('📡 Client disconnected from testing infrastructure');
  });
});

const PORT = process.env.PORT || 3000;
server.listen(PORT, () => {
  console.log(`
🌙 Luna Testing Infrastructure Demo
📡 Server running on http://localhost:${PORT}
🚀 Ready for seamless testing sessions!
  `);
});