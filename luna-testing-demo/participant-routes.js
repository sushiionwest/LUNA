import express from 'express';
import sqlite3 from 'sqlite3';
import { v4 as uuidv4 } from 'uuid';
import nodemailer from 'nodemailer';

const router = express.Router();

// Email configuration (using nodemailer)
const emailTransporter = nodemailer.createTransport({
  // Configure with your email service
  service: 'gmail', // or your preferred service
  auth: {
    user: process.env.EMAIL_USER || 'luna.testing@example.com',
    pass: process.env.EMAIL_PASS || 'your-app-password'
  }
});

// Participant registration endpoint
router.post('/register', async (req, res) => {
  try {
    const {
      name, email, role, experience, os, motivation, 
      availability, newsletter, terms, timestamp
    } = req.body;

    // Validation
    if (!name || !email || !role || !experience || !os || !motivation || !terms) {
      return res.status(400).json({ 
        error: 'Missing required fields', 
        required: ['name', 'email', 'role', 'experience', 'os', 'motivation', 'terms']
      });
    }

    const participantId = uuidv4();
    
    // Determine testing phase based on experience level
    let testingPhase = 'phase-3'; // Default to consumer testing
    if (experience === 'expert' || (experience === 'advanced' && role === 'developer')) {
      testingPhase = 'phase-1'; // Technical testing
    } else if (experience === 'advanced' || experience === 'intermediate') {
      testingPhase = 'phase-2'; // Business testing
    }

    // Insert participant into database
    const db = req.app.get('database');
    
    await new Promise((resolve, reject) => {
      db.run(`
        INSERT INTO participants (
          id, name, email, role, experience, os, motivation, 
          availability, newsletter, testing_phase, status, 
          registered_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      `, [
        participantId, name, email, role, experience, os, motivation,
        availability, newsletter ? 1 : 0, testingPhase, 'registered',
        new Date().toISOString()
      ], function(err) {
        if (err) reject(err);
        else resolve(this.lastID);
      });
    });

    // Send welcome email
    await sendWelcomeEmail(email, name, testingPhase);

    // Log registration event
    console.log(`✅ New participant registered: ${name} (${email}) - ${testingPhase}`);

    res.json({
      success: true,
      participantId,
      testingPhase,
      message: 'Registration successful! Check your email for next steps.'
    });

  } catch (error) {
    console.error('❌ Registration error:', error);
    res.status(500).json({ error: 'Registration failed. Please try again.' });
  }
});

// Get all participants
router.get('/', (req, res) => {
  const db = req.app.get('database');
  
  db.all(`
    SELECT id, name, email, role, experience, os, testing_phase, 
           status, registered_at, scheduled_at
    FROM participants 
    ORDER BY registered_at DESC
  `, (err, rows) => {
    if (err) {
      res.status(500).json({ error: err.message });
    } else {
      res.json(rows);
    }
  });
});

// Get participant by ID
router.get('/:id', (req, res) => {
  const db = req.app.get('database');
  
  db.get(`
    SELECT * FROM participants WHERE id = ?
  `, [req.params.id], (err, row) => {
    if (err) {
      res.status(500).json({ error: err.message });
    } else if (!row) {
      res.status(404).json({ error: 'Participant not found' });
    } else {
      res.json(row);
    }
  });
});

// Update participant status
router.patch('/:id/status', (req, res) => {
  const { status, notes } = req.body;
  const db = req.app.get('database');
  
  db.run(`
    UPDATE participants 
    SET status = ?, notes = ?, updated_at = CURRENT_TIMESTAMP 
    WHERE id = ?
  `, [status, notes, req.params.id], function(err) {
    if (err) {
      res.status(500).json({ error: err.message });
    } else if (this.changes === 0) {
      res.status(404).json({ error: 'Participant not found' });
    } else {
      res.json({ success: true, updated: this.changes });
    }
  });
});

// Schedule participant for testing
router.post('/:id/schedule', async (req, res) => {
  try {
    const { scheduledDate, sessionType, notes } = req.body;
    const db = req.app.get('database');
    
    // Update participant with scheduling info
    await new Promise((resolve, reject) => {
      db.run(`
        UPDATE participants 
        SET scheduled_at = ?, session_type = ?, status = 'scheduled', 
            notes = ?, updated_at = CURRENT_TIMESTAMP 
        WHERE id = ?
      `, [scheduledDate, sessionType, notes, req.params.id], function(err) {
        if (err) reject(err);
        else resolve(this.changes);
      });
    });

    // Get participant details for confirmation email
    const participant = await new Promise((resolve, reject) => {
      db.get('SELECT * FROM participants WHERE id = ?', [req.params.id], (err, row) => {
        if (err) reject(err);
        else resolve(row);
      });
    });

    if (participant) {
      await sendSchedulingEmail(participant.email, participant.name, scheduledDate, sessionType);
    }

    res.json({ success: true, message: 'Participant scheduled successfully' });

  } catch (error) {
    console.error('❌ Scheduling error:', error);
    res.status(500).json({ error: 'Scheduling failed. Please try again.' });
  }
});

// Get participants by testing phase
router.get('/phase/:phase', (req, res) => {
  const db = req.app.get('database');
  
  db.all(`
    SELECT id, name, email, role, experience, status, registered_at
    FROM participants 
    WHERE testing_phase = ?
    ORDER BY registered_at ASC
  `, [req.params.phase], (err, rows) => {
    if (err) {
      res.status(500).json({ error: err.message });
    } else {
      res.json(rows);
    }
  });
});

// Analytics endpoint
router.get('/analytics/summary', (req, res) => {
  const db = req.app.get('database');
  
  // Get registration statistics
  db.all(`
    SELECT 
      testing_phase,
      status,
      COUNT(*) as count
    FROM participants 
    GROUP BY testing_phase, status
  `, (err, rows) => {
    if (err) {
      res.status(500).json({ error: err.message });
    } else {
      // Transform data for easier consumption
      const summary = {
        total: 0,
        by_phase: {},
        by_status: {},
        registration_trend: []
      };
      
      rows.forEach(row => {
        summary.total += row.count;
        
        if (!summary.by_phase[row.testing_phase]) {
          summary.by_phase[row.testing_phase] = 0;
        }
        summary.by_phase[row.testing_phase] += row.count;
        
        if (!summary.by_status[row.status]) {
          summary.by_status[row.status] = 0;
        }
        summary.by_status[row.status] += row.count;
      });
      
      res.json(summary);
    }
  });
});

// Email templates and sending functions
async function sendWelcomeEmail(email, name, testingPhase) {
  const phaseInfo = {
    'phase-1': {
      name: 'Technical User Testing',
      description: 'You\'ll test Luna\'s installation and core features from a developer perspective.',
      timeCommitment: '5-minute testing session',
      timing: 'Week 1'
    },
    'phase-2': {
      name: 'Business User Testing',
      description: 'You\'ll test Luna for real-world business automation workflows.',
      timeCommitment: '30-minute session + 3 days of usage',
      timing: 'Weeks 2-3'
    },
    'phase-3': {
      name: 'Consumer Testing',
      description: 'You\'ll test Luna\'s ease-of-use and accessibility.',
      timeCommitment: '15-minute session + feedback',
      timing: 'Week 4'
    }
  };

  const phase = phaseInfo[testingPhase];
  
  const emailHTML = `
    <!DOCTYPE html>
    <html>
    <head>
        <style>
            body { font-family: 'Segoe UI', Arial, sans-serif; line-height: 1.6; color: #333; }
            .container { max-width: 600px; margin: 0 auto; padding: 20px; }
            .header { text-align: center; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; border-radius: 10px; margin-bottom: 30px; }
            .luna-logo { font-size: 3em; margin-bottom: 10px; }
            .content { background: #f8fafc; padding: 25px; border-radius: 10px; margin-bottom: 20px; }
            .phase-info { background: white; padding: 20px; border-radius: 8px; border-left: 4px solid #667eea; }
            .button { display: inline-block; background: #667eea; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; margin: 10px 0; }
            .footer { text-align: center; color: #666; margin-top: 30px; font-size: 0.9em; }
        </style>
    </head>
    <body>
        <div class="container">
            <div class="header">
                <div class="luna-logo">🌙</div>
                <h1>Welcome to Luna's Testing Program!</h1>
                <p>Thank you for joining our early access community</p>
            </div>
            
            <div class="content">
                <h2>Hi ${name},</h2>
                <p>We're excited to have you as part of Luna's testing program! Your feedback will directly shape the future of AI automation.</p>
                
                <div class="phase-info">
                    <h3>📋 Your Testing Assignment</h3>
                    <p><strong>Phase:</strong> ${phase.name}</p>
                    <p><strong>Description:</strong> ${phase.description}</p>
                    <p><strong>Time Commitment:</strong> ${phase.timeCommitment}</p>
                    <p><strong>Timing:</strong> ${phase.timing}</p>
                </div>
                
                <h3>🚀 What Happens Next?</h3>
                <ol>
                    <li><strong>Confirmation Call:</strong> We'll contact you within 24 hours to confirm your participation and schedule your session.</li>
                    <li><strong>Pre-Test Setup:</strong> We'll send you any necessary preparation instructions.</li>
                    <li><strong>Testing Session:</strong> You'll experience Luna's one-click installation firsthand.</li>
                    <li><strong>Feedback:</strong> Share your thoughts through a brief interview or survey.</li>
                    <li><strong>Keep Luna:</strong> Enjoy lifetime access to Luna as our thank you!</li>
                </ol>
                
                <h3>🎁 Your Benefits</h3>
                <ul>
                    <li>Free lifetime Luna license (normally $99/year)</li>
                    <li>Early adopter recognition and credits</li>
                    <li>Direct influence on Luna's development</li>
                    <li>First access to new features and updates</li>
                </ul>
                
                <p>Have questions? Simply reply to this email or visit our FAQ page.</p>
                
                <a href="http://localhost:3000/participants/${email}/profile" class="button">View Your Profile</a>
            </div>
            
            <div class="footer">
                <p>Luna Testing Program | Powered by AI that sees in the dark 🌙</p>
                <p>If you no longer wish to participate, <a href="#">click here to withdraw</a></p>
            </div>
        </div>
    </body>
    </html>
  `;

  try {
    await emailTransporter.sendMail({
      from: '"Luna Testing Team" <luna.testing@example.com>',
      to: email,
      subject: `🌙 Welcome to Luna's Testing Program - ${phase.name}`,
      html: emailHTML
    });
    
    console.log(`📧 Welcome email sent to ${email}`);
  } catch (error) {
    console.error(`❌ Failed to send welcome email to ${email}:`, error);
  }
}

async function sendSchedulingEmail(email, name, scheduledDate, sessionType) {
  const emailHTML = `
    <!DOCTYPE html>
    <html>
    <head>
        <style>
            body { font-family: 'Segoe UI', Arial, sans-serif; line-height: 1.6; color: #333; }
            .container { max-width: 600px; margin: 0 auto; padding: 20px; }
            .header { text-align: center; background: linear-gradient(135deg, #10b981 0%, #059669 100%); color: white; padding: 30px; border-radius: 10px; margin-bottom: 30px; }
            .content { background: #f8fafc; padding: 25px; border-radius: 10px; margin-bottom: 20px; }
            .session-info { background: white; padding: 20px; border-radius: 8px; border-left: 4px solid #10b981; margin: 20px 0; }
            .button { display: inline-block; background: #10b981; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; margin: 10px 0; }
            .prep-list { background: #fef3c7; padding: 15px; border-radius: 8px; border-left: 4px solid #f59e0b; }
        </style>
    </head>
    <body>
        <div class="container">
            <div class="header">
                <h1>🗓️ Your Luna Testing Session is Scheduled!</h1>
                <p>Get ready to experience the future of AI automation</p>
            </div>
            
            <div class="content">
                <h2>Hi ${name},</h2>
                <p>Great news! Your Luna testing session has been scheduled. We're looking forward to your feedback!</p>
                
                <div class="session-info">
                    <h3>📅 Session Details</h3>
                    <p><strong>Date & Time:</strong> ${new Date(scheduledDate).toLocaleString()}</p>
                    <p><strong>Session Type:</strong> ${sessionType}</p>
                    <p><strong>Duration:</strong> 15-30 minutes</p>
                    <p><strong>Format:</strong> Screen sharing + interview</p>
                </div>
                
                <div class="prep-list">
                    <h3>⚡ Before Your Session</h3>
                    <ul>
                        <li>Ensure stable internet connection</li>
                        <li>Close unnecessary applications</li>
                        <li>Have your thinking voice ready - we love hearing your thought process!</li>
                        <li>Prepare any questions about Luna or AI automation</li>
                    </ul>
                </div>
                
                <h3>🎯 What We'll Cover</h3>
                <ol>
                    <li><strong>Brief Introduction:</strong> Tell us about your background (2 minutes)</li>
                    <li><strong>Luna Installation:</strong> Download and install Luna while thinking aloud (5-10 minutes)</li>
                    <li><strong>First Impressions:</strong> Initial interaction with Luna (5-10 minutes)</li>
                    <li><strong>Feedback Session:</strong> Share your thoughts and suggestions (5 minutes)</li>
                </ol>
                
                <p>We'll send you the video call link 1 hour before your session.</p>
                
                <a href="mailto:luna.testing@example.com?subject=Reschedule Request" class="button">Need to Reschedule?</a>
            </div>
        </div>
    </body>
    </html>
  `;

  try {
    await emailTransporter.sendMail({
      from: '"Luna Testing Team" <luna.testing@example.com>',
      to: email,
      subject: `🗓️ Luna Testing Session Scheduled - ${new Date(scheduledDate).toLocaleDateString()}`,
      html: emailHTML
    });
    
    console.log(`📧 Scheduling email sent to ${email}`);
  } catch (error) {
    console.error(`❌ Failed to send scheduling email to ${email}:`, error);
  }
}

export default router;