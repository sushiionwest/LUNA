/**
 * Database Service - SQLite database management for Luna Testing Infrastructure
 */

import Database from 'sqlite3';
import { promisify } from 'util';
import path from 'path';
import fs from 'fs/promises';

export class DatabaseService {
    constructor() {
        this.dbPath = path.join(process.cwd(), 'data', 'luna-testing.db');
        this.db = null;
        this.isInitialized = false;
    }

    async initialize() {
        try {
            // Ensure data directory exists
            await fs.mkdir(path.dirname(this.dbPath), { recursive: true });

            // Connect to database
            this.db = new Database.Database(this.dbPath);
            
            // Promisify database methods
            this.db.run = promisify(this.db.run.bind(this.db));
            this.db.get = promisify(this.db.get.bind(this.db));
            this.db.all = promisify(this.db.all.bind(this.db));

            // Create tables
            await this.createTables();
            
            // Insert seed data
            await this.seedData();

            this.isInitialized = true;
            console.log('✅ Database initialized successfully');
        } catch (error) {
            console.error('❌ Database initialization failed:', error);
            throw error;
        }
    }

    async createTables() {
        const tables = [
            // Participants table
            `CREATE TABLE IF NOT EXISTS participants (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                first_name TEXT NOT NULL,
                last_name TEXT,
                phone TEXT,
                operating_system TEXT NOT NULL,
                tech_level TEXT NOT NULL,
                use_case_interest TEXT,
                segment TEXT NOT NULL,
                status TEXT DEFAULT 'pending',
                notes TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )`,

            // Testing sessions table
            `CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                participant_id TEXT NOT NULL,
                scenario_type TEXT NOT NULL,
                status TEXT DEFAULT 'scheduled',
                scheduled_at DATETIME,
                started_at DATETIME,
                completed_at DATETIME,
                duration_minutes INTEGER,
                observer_id TEXT,
                notes TEXT,
                recording_path TEXT,
                screenshot_paths TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (participant_id) REFERENCES participants (id)
            )`,

            // Session events table (detailed tracking)
            `CREATE TABLE IF NOT EXISTS session_events (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                event_data TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                relative_time INTEGER,
                FOREIGN KEY (session_id) REFERENCES sessions (id)
            )`,

            // Task completion tracking
            `CREATE TABLE IF NOT EXISTS task_completions (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                task_index INTEGER NOT NULL,
                task_name TEXT NOT NULL,
                status TEXT NOT NULL,
                time_started DATETIME,
                time_completed DATETIME,
                duration_seconds INTEGER,
                success BOOLEAN,
                error_message TEXT,
                notes TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions (id)
            )`,

            // Feedback submissions
            `CREATE TABLE IF NOT EXISTS feedback (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                participant_id TEXT NOT NULL,
                tech_level TEXT,
                use_case TEXT,
                describe_luna TEXT,
                install_rating INTEGER,
                pain_points TEXT,
                trust_level TEXT,
                nps_score INTEGER,
                improvements TEXT,
                additional_comments TEXT,
                submitted_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES sessions (id),
                FOREIGN KEY (participant_id) REFERENCES participants (id)
            )`,

            // Issues and bug reports
            `CREATE TABLE IF NOT EXISTS issues (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                participant_id TEXT,
                issue_type TEXT NOT NULL,
                severity TEXT DEFAULT 'medium',
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                steps_to_reproduce TEXT,
                expected_behavior TEXT,
                actual_behavior TEXT,
                environment_info TEXT,
                status TEXT DEFAULT 'open',
                assigned_to TEXT,
                resolved_at DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES sessions (id),
                FOREIGN KEY (participant_id) REFERENCES participants (id)
            )`,

            // Email tracking
            `CREATE TABLE IF NOT EXISTS emails (
                id TEXT PRIMARY KEY,
                participant_id TEXT,
                email_type TEXT NOT NULL,
                subject TEXT NOT NULL,
                template_name TEXT,
                sent_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                delivered_at DATETIME,
                opened_at DATETIME,
                clicked_at DATETIME,
                status TEXT DEFAULT 'sent',
                error_message TEXT,
                FOREIGN KEY (participant_id) REFERENCES participants (id)
            )`,

            // File uploads tracking
            `CREATE TABLE IF NOT EXISTS files (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                file_type TEXT NOT NULL,
                original_name TEXT NOT NULL,
                stored_path TEXT NOT NULL,
                file_size INTEGER,
                mime_type TEXT,
                uploaded_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES sessions (id)
            )`,

            // A/B testing variants
            `CREATE TABLE IF NOT EXISTS ab_test_variants (
                id TEXT PRIMARY KEY,
                participant_id TEXT NOT NULL,
                test_name TEXT NOT NULL,
                variant TEXT NOT NULL,
                assigned_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (participant_id) REFERENCES participants (id)
            )`,

            // System logs
            `CREATE TABLE IF NOT EXISTS system_logs (
                id TEXT PRIMARY KEY,
                level TEXT NOT NULL,
                component TEXT NOT NULL,
                message TEXT NOT NULL,
                data TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )`
        ];

        for (const table of tables) {
            await this.db.run(table);
        }

        // Create indexes for performance
        const indexes = [
            'CREATE INDEX IF NOT EXISTS idx_participants_email ON participants(email)',
            'CREATE INDEX IF NOT EXISTS idx_participants_segment ON participants(segment)',
            'CREATE INDEX IF NOT EXISTS idx_participants_status ON participants(status)',
            'CREATE INDEX IF NOT EXISTS idx_sessions_participant ON sessions(participant_id)',
            'CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status)',
            'CREATE INDEX IF NOT EXISTS idx_sessions_scenario ON sessions(scenario_type)',
            'CREATE INDEX IF NOT EXISTS idx_events_session ON session_events(session_id)',
            'CREATE INDEX IF NOT EXISTS idx_events_type ON session_events(event_type)',
            'CREATE INDEX IF NOT EXISTS idx_feedback_session ON feedback(session_id)',
            'CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status)',
            'CREATE INDEX IF NOT EXISTS idx_emails_participant ON emails(participant_id)',
            'CREATE INDEX IF NOT EXISTS idx_files_session ON files(session_id)'
        ];

        for (const index of indexes) {
            await this.db.run(index);
        }
    }

    async seedData() {
        // Insert test data for development
        const testParticipants = [
            {
                id: 'test-001',
                email: 'alice@example.com',
                first_name: 'Alice',
                last_name: 'Johnson',
                operating_system: 'Windows',
                tech_level: 'non-technical',
                use_case_interest: 'social-media',
                segment: 'non-technical'
            },
            {
                id: 'test-002',
                email: 'bob@example.com',
                first_name: 'Bob',
                last_name: 'Smith',
                operating_system: 'macOS',
                tech_level: 'intermediate',
                use_case_interest: 'productivity',
                segment: 'semi-technical'
            },
            {
                id: 'test-003',
                email: 'carol@example.com',
                first_name: 'Carol',
                last_name: 'Davis',
                operating_system: 'Linux',
                tech_level: 'advanced',
                use_case_interest: 'business',
                segment: 'technical'
            }
        ];

        for (const participant of testParticipants) {
            try {
                await this.db.run(
                    `INSERT OR IGNORE INTO participants 
                     (id, email, first_name, last_name, operating_system, tech_level, use_case_interest, segment)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
                    [participant.id, participant.email, participant.first_name, participant.last_name,
                     participant.operating_system, participant.tech_level, participant.use_case_interest, participant.segment]
                );
            } catch (error) {
                // Ignore duplicate key errors
                if (!error.message.includes('UNIQUE constraint failed')) {
                    throw error;
                }
            }
        }

        // Insert sample sessions
        const testSessions = [
            {
                id: 'session-001',
                participant_id: 'test-001',
                scenario_type: 'first-time',
                status: 'completed',
                started_at: new Date(Date.now() - 3600000).toISOString(),
                completed_at: new Date(Date.now() - 3000000).toISOString(),
                duration_minutes: 35
            },
            {
                id: 'session-002',
                participant_id: 'test-002',
                scenario_type: 'daily-usage',
                status: 'in-progress',
                started_at: new Date(Date.now() - 1800000).toISOString()
            }
        ];

        for (const session of testSessions) {
            try {
                await this.db.run(
                    `INSERT OR IGNORE INTO sessions 
                     (id, participant_id, scenario_type, status, started_at, completed_at, duration_minutes)
                     VALUES (?, ?, ?, ?, ?, ?, ?)`,
                    [session.id, session.participant_id, session.scenario_type, session.status,
                     session.started_at, session.completed_at, session.duration_minutes]
                );
            } catch (error) {
                if (!error.message.includes('UNIQUE constraint failed')) {
                    throw error;
                }
            }
        }
    }

    // Participant methods
    async createParticipant(data) {
        const id = `participant_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        
        await this.db.run(
            `INSERT INTO participants 
             (id, email, first_name, last_name, phone, operating_system, tech_level, use_case_interest, segment)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
            [id, data.email, data.firstName, data.lastName, data.phone,
             data.operatingSystem, data.techLevel, data.useCaseInterest, data.segment]
        );

        return this.getParticipant(id);
    }

    async getParticipant(id) {
        return await this.db.get('SELECT * FROM participants WHERE id = ?', [id]);
    }

    async getParticipantByEmail(email) {
        return await this.db.get('SELECT * FROM participants WHERE email = ?', [email]);
    }

    async updateParticipant(id, data) {
        const updates = Object.keys(data).map(key => `${key} = ?`).join(', ');
        const values = Object.values(data);
        values.push(id);

        await this.db.run(
            `UPDATE participants SET ${updates}, updated_at = CURRENT_TIMESTAMP WHERE id = ?`,
            values
        );

        return this.getParticipant(id);
    }

    async listParticipants(filters = {}) {
        let query = 'SELECT * FROM participants WHERE 1=1';
        const params = [];

        if (filters.segment) {
            query += ' AND segment = ?';
            params.push(filters.segment);
        }

        if (filters.status) {
            query += ' AND status = ?';
            params.push(filters.status);
        }

        if (filters.techLevel) {
            query += ' AND tech_level = ?';
            params.push(filters.techLevel);
        }

        query += ' ORDER BY created_at DESC';

        if (filters.limit) {
            query += ' LIMIT ?';
            params.push(filters.limit);
        }

        if (filters.offset) {
            query += ' OFFSET ?';
            params.push(filters.offset);
        }

        return await this.db.all(query, params);
    }

    // Session methods
    async createSession(data) {
        const id = `session_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        
        await this.db.run(
            `INSERT INTO sessions 
             (id, participant_id, scenario_type, scheduled_at, observer_id, notes)
             VALUES (?, ?, ?, ?, ?, ?)`,
            [id, data.participantId, data.scenarioType, data.scheduledAt, data.observerId, data.notes]
        );

        return this.getSession(id);
    }

    async getSession(id) {
        return await this.db.get('SELECT * FROM sessions WHERE id = ?', [id]);
    }

    async updateSession(id, data) {
        const updates = Object.keys(data).map(key => `${key} = ?`).join(', ');
        const values = Object.values(data);
        values.push(id);

        await this.db.run(
            `UPDATE sessions SET ${updates}, updated_at = CURRENT_TIMESTAMP WHERE id = ?`,
            values
        );

        return this.getSession(id);
    }

    async listSessions(filters = {}) {
        let query = `
            SELECT s.*, p.first_name, p.last_name, p.email, p.segment
            FROM sessions s
            JOIN participants p ON s.participant_id = p.id
            WHERE 1=1
        `;
        const params = [];

        if (filters.status) {
            query += ' AND s.status = ?';
            params.push(filters.status);
        }

        if (filters.scenarioType) {
            query += ' AND s.scenario_type = ?';
            params.push(filters.scenarioType);
        }

        if (filters.segment) {
            query += ' AND p.segment = ?';
            params.push(filters.segment);
        }

        query += ' ORDER BY s.created_at DESC';

        if (filters.limit) {
            query += ' LIMIT ?';
            params.push(filters.limit);
        }

        return await this.db.all(query, params);
    }

    // Event tracking methods
    async recordEvent(sessionId, eventData) {
        const id = `event_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        
        await this.db.run(
            `INSERT INTO session_events 
             (id, session_id, event_type, event_data, relative_time)
             VALUES (?, ?, ?, ?, ?)`,
            [id, sessionId, eventData.type, JSON.stringify(eventData.data), eventData.relativeTime]
        );

        return this.getEvent(id);
    }

    async getEvent(id) {
        const event = await this.db.get('SELECT * FROM session_events WHERE id = ?', [id]);
        if (event) {
            event.event_data = JSON.parse(event.event_data || '{}');
        }
        return event;
    }

    async getSessionEvents(sessionId) {
        const events = await this.db.all(
            'SELECT * FROM session_events WHERE session_id = ? ORDER BY timestamp ASC',
            [sessionId]
        );
        
        return events.map(event => ({
            ...event,
            event_data: JSON.parse(event.event_data || '{}')
        }));
    }

    // Feedback methods
    async submitFeedback(sessionId, participantId, feedbackData) {
        const id = `feedback_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        
        await this.db.run(
            `INSERT INTO feedback 
             (id, session_id, participant_id, tech_level, use_case, describe_luna, 
              install_rating, pain_points, trust_level, nps_score, improvements, additional_comments)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
            [id, sessionId, participantId, feedbackData.techLevel, feedbackData.useCase,
             feedbackData.describeLuna, feedbackData.installRating, feedbackData.painPoints,
             feedbackData.trustLevel, feedbackData.npsScore, feedbackData.improvements,
             feedbackData.additionalComments]
        );

        return this.getFeedback(id);
    }

    async getFeedback(id) {
        return await this.db.get('SELECT * FROM feedback WHERE id = ?', [id]);
    }

    // Analytics methods
    async getCompletionRates(filters = {}) {
        let query = `
            SELECT 
                s.scenario_type,
                p.segment,
                COUNT(*) as total_sessions,
                SUM(CASE WHEN s.status = 'completed' THEN 1 ELSE 0 END) as completed_sessions,
                ROUND(
                    SUM(CASE WHEN s.status = 'completed' THEN 1 ELSE 0 END) * 100.0 / COUNT(*), 
                    2
                ) as completion_rate
            FROM sessions s
            JOIN participants p ON s.participant_id = p.id
            WHERE 1=1
        `;
        const params = [];

        if (filters.startDate) {
            query += ' AND s.created_at >= ?';
            params.push(filters.startDate);
        }

        if (filters.endDate) {
            query += ' AND s.created_at <= ?';
            params.push(filters.endDate);
        }

        if (filters.segment) {
            query += ' AND p.segment = ?';
            params.push(filters.segment);
        }

        query += ' GROUP BY s.scenario_type, p.segment ORDER BY s.scenario_type, p.segment';

        return await this.db.all(query, params);
    }

    async getTimeMetrics(filters = {}) {
        let query = `
            SELECT 
                s.scenario_type,
                p.segment,
                COUNT(*) as session_count,
                AVG(s.duration_minutes) as avg_duration,
                MIN(s.duration_minutes) as min_duration,
                MAX(s.duration_minutes) as max_duration
            FROM sessions s
            JOIN participants p ON s.participant_id = p.id
            WHERE s.status = 'completed' AND s.duration_minutes IS NOT NULL
        `;
        const params = [];

        if (filters.startDate) {
            query += ' AND s.completed_at >= ?';
            params.push(filters.startDate);
        }

        if (filters.endDate) {
            query += ' AND s.completed_at <= ?';
            params.push(filters.endDate);
        }

        if (filters.scenario) {
            query += ' AND s.scenario_type = ?';
            params.push(filters.scenario);
        }

        query += ' GROUP BY s.scenario_type, p.segment ORDER BY s.scenario_type, p.segment';

        return await this.db.all(query, params);
    }

    async getFeedbackAnalysis() {
        const npsAnalysis = await this.db.get(`
            SELECT 
                COUNT(*) as total_responses,
                AVG(nps_score) as avg_nps,
                SUM(CASE WHEN nps_score >= 9 THEN 1 ELSE 0 END) as promoters,
                SUM(CASE WHEN nps_score >= 7 AND nps_score <= 8 THEN 1 ELSE 0 END) as passives,
                SUM(CASE WHEN nps_score <= 6 THEN 1 ELSE 0 END) as detractors
            FROM feedback 
            WHERE nps_score IS NOT NULL
        `);

        const installRatings = await this.db.get(`
            SELECT 
                AVG(install_rating) as avg_rating,
                COUNT(*) as total_ratings
            FROM feedback 
            WHERE install_rating IS NOT NULL
        `);

        const trustLevels = await this.db.all(`
            SELECT 
                trust_level,
                COUNT(*) as count,
                ROUND(COUNT(*) * 100.0 / (SELECT COUNT(*) FROM feedback WHERE trust_level IS NOT NULL), 2) as percentage
            FROM feedback 
            WHERE trust_level IS NOT NULL
            GROUP BY trust_level
        `);

        return {
            nps: npsAnalysis,
            installRating: installRatings,
            trustLevels: trustLevels
        };
    }

    async isHealthy() {
        try {
            await this.db.get('SELECT 1');
            return true;
        } catch (error) {
            console.error('Database health check failed:', error);
            return false;
        }
    }

    getConnectionString() {
        return this.dbPath;
    }

    async close() {
        if (this.db) {
            await new Promise((resolve, reject) => {
                this.db.close((err) => {
                    if (err) reject(err);
                    else resolve();
                });
            });
        }
    }
}