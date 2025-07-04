/**
 * Session Service - Manages testing sessions, real-time tracking, and data collection
 */

import { v4 as uuidv4 } from 'uuid';

export class SessionService {
    constructor(database, socketIO) {
        this.db = database;
        this.io = socketIO;
        this.activeSessions = new Map(); // Track active sessions in memory
        this.taskDefinitions = this.getTaskDefinitions();
    }

    getTaskDefinitions() {
        return {
            'first-time': [
                { id: 1, name: 'Website Discovery', description: 'Visit Luna website and understand what it does' },
                { id: 2, name: 'Download Process', description: 'Download Luna installer' },
                { id: 3, name: 'Installation', description: 'Install Luna on computer' },
                { id: 4, name: 'First Launch', description: 'Launch Luna for the first time' },
                { id: 5, name: 'Initial Setup', description: 'Complete setup and onboarding' },
                { id: 6, name: 'First Automation', description: 'Create first automation task' }
            ],
            'daily-usage': [
                { id: 1, name: 'Launch Check', description: 'Launch Luna and check existing automations' },
                { id: 2, name: 'New Automation', description: 'Create a new automation' },
                { id: 3, name: 'Modify Settings', description: 'Change settings on existing automation' },
                { id: 4, name: 'Monitor Activity', description: 'Review Luna activity and performance' }
            ],
            'problem-recovery': [
                { id: 1, name: 'Network Issue', description: 'Resolve network disconnection problem' },
                { id: 2, name: 'App Recovery', description: 'Handle unresponsive application' },
                { id: 3, name: 'Find Help', description: 'Find help and support resources' },
                { id: 4, name: 'Contact Support', description: 'Use support contact methods' }
            ]
        };
    }

    async create(data) {
        try {
            this.validateSessionData(data);

            const session = await this.db.createSession({
                participantId: data.participantId,
                scenarioType: data.scenarioType,
                scheduledAt: data.scheduledAt,
                observerId: data.observerId,
                notes: data.notes || ''
            });

            // Initialize session tracking
            this.activeSessions.set(session.id, {
                id: session.id,
                participantId: session.participant_id,
                scenarioType: session.scenario_type,
                startTime: null,
                currentTask: 0,
                tasks: this.taskDefinitions[session.scenario_type] || [],
                events: [],
                observers: new Set()
            });

            // Log session creation
            await this.logEvent(session.id, 'session_created', {
                participantId: session.participant_id,
                scenarioType: session.scenario_type,
                observerId: session.observer_id
            });

            return session;

        } catch (error) {
            console.error('Error creating session:', error);
            throw error;
        }
    }

    validateSessionData(data) {
        const required = ['participantId', 'scenarioType'];
        const missing = required.filter(field => !data[field]);
        
        if (missing.length > 0) {
            throw new Error(`Missing required fields: ${missing.join(', ')}`);
        }

        const validScenarios = ['first-time', 'daily-usage', 'problem-recovery'];
        if (!validScenarios.includes(data.scenarioType)) {
            throw new Error('Invalid scenario type');
        }
    }

    async getById(id) {
        try {
            const session = await this.db.getSession(id);
            if (!session) {
                return null;
            }

            // Get related data
            const participant = await this.db.getParticipant(session.participant_id);
            const events = await this.db.getSessionEvents(id);
            const taskCompletions = await this.getTaskCompletions(id);

            // Get active session data if available
            const activeData = this.activeSessions.get(id);

            return {
                ...session,
                participant,
                events,
                taskCompletions,
                tasks: this.taskDefinitions[session.scenario_type] || [],
                activeData: activeData || null,
                statistics: await this.getSessionStatistics(id)
            };

        } catch (error) {
            console.error('Error getting session:', error);
            throw error;
        }
    }

    async update(id, data) {
        try {
            const session = await this.db.updateSession(id, data);

            // Update active session if it exists
            const activeSession = this.activeSessions.get(id);
            if (activeSession) {
                Object.assign(activeSession, data);
            }

            // Broadcast update to observers
            this.io.to(`session_${id}`).emit('session_updated', {
                sessionId: id,
                updates: data,
                timestamp: new Date().toISOString()
            });

            return session;

        } catch (error) {
            console.error('Error updating session:', error);
            throw error;
        }
    }

    async start(sessionId, observerId) {
        try {
            const startTime = new Date().toISOString();
            
            // Update database
            await this.db.updateSession(sessionId, {
                status: 'in-progress',
                started_at: startTime,
                observer_id: observerId
            });

            // Update active session
            const activeSession = this.activeSessions.get(sessionId);
            if (activeSession) {
                activeSession.startTime = Date.now();
                activeSession.observers.add(observerId);
            }

            // Record start event
            await this.recordEvent(sessionId, {
                type: 'session_started',
                data: { observerId },
                relativeTime: 0
            });

            // Notify observers
            this.io.to(`session_${sessionId}`).emit('session_started', {
                sessionId,
                startTime,
                observerId
            });

            return true;

        } catch (error) {
            console.error('Error starting session:', error);
            throw error;
        }
    }

    async complete(sessionId, completionData = {}) {
        try {
            const session = await this.db.getSession(sessionId);
            if (!session) {
                throw new Error('Session not found');
            }

            const completedAt = new Date().toISOString();
            const duration = session.started_at ? 
                Math.round((new Date(completedAt) - new Date(session.started_at)) / 1000 / 60) : 
                null;

            // Update database
            await this.db.updateSession(sessionId, {
                status: 'completed',
                completed_at: completedAt,
                duration_minutes: duration,
                notes: completionData.notes || session.notes
            });

            // Record completion event
            await this.recordEvent(sessionId, {
                type: 'session_completed',
                data: { 
                    duration,
                    completionNotes: completionData.notes 
                },
                relativeTime: duration ? duration * 60 * 1000 : 0
            });

            // Clean up active session
            this.activeSessions.delete(sessionId);

            // Notify observers
            this.io.to(`session_${sessionId}`).emit('session_completed', {
                sessionId,
                completedAt,
                duration
            });

            return true;

        } catch (error) {
            console.error('Error completing session:', error);
            throw error;
        }
    }

    async recordEvent(sessionId, eventData) {
        try {
            // Add to database
            const event = await this.db.recordEvent(sessionId, eventData);

            // Update active session
            const activeSession = this.activeSessions.get(sessionId);
            if (activeSession) {
                activeSession.events.push(event);
            }

            // Broadcast to observers
            this.io.to(`session_${sessionId}`).emit('session_event', {
                sessionId,
                event,
                timestamp: new Date().toISOString()
            });

            // Trigger any automated analysis
            await this.analyzeEvent(sessionId, event);

            return event;

        } catch (error) {
            console.error('Error recording event:', error);
            throw error;
        }
    }

    async startTask(sessionId, taskIndex) {
        try {
            const activeSession = this.activeSessions.get(sessionId);
            if (!activeSession) {
                throw new Error('Session not active');
            }

            const task = activeSession.tasks[taskIndex];
            if (!task) {
                throw new Error('Invalid task index');
            }

            // Update current task
            activeSession.currentTask = taskIndex;

            // Record task start
            const taskCompletion = await this.db.db.run(
                `INSERT INTO task_completions 
                 (id, session_id, task_index, task_name, status, time_started)
                 VALUES (?, ?, ?, ?, ?, ?)`,
                [
                    `task_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
                    sessionId,
                    taskIndex,
                    task.name,
                    'in-progress',
                    new Date().toISOString()
                ]
            );

            // Record event
            await this.recordEvent(sessionId, {
                type: 'task_started',
                data: {
                    taskIndex,
                    taskName: task.name,
                    description: task.description
                },
                relativeTime: activeSession.startTime ? Date.now() - activeSession.startTime : 0
            });

            return taskCompletion;

        } catch (error) {
            console.error('Error starting task:', error);
            throw error;
        }
    }

    async completeTask(sessionId, taskIndex, success = true, notes = '') {
        try {
            const activeSession = this.activeSessions.get(sessionId);
            if (!activeSession) {
                throw new Error('Session not active');
            }

            const completedAt = new Date().toISOString();

            // Update task completion
            await this.db.db.run(
                `UPDATE task_completions 
                 SET status = ?, time_completed = ?, success = ?, notes = ?
                 WHERE session_id = ? AND task_index = ? AND status = 'in-progress'`,
                [success ? 'completed' : 'failed', completedAt, success, notes, sessionId, taskIndex]
            );

            // Calculate duration
            const taskStart = await this.db.db.get(
                `SELECT time_started FROM task_completions 
                 WHERE session_id = ? AND task_index = ?`,
                [sessionId, taskIndex]
            );

            const duration = taskStart ? 
                Math.round((new Date(completedAt) - new Date(taskStart.time_started)) / 1000) : 
                null;

            // Record event
            await this.recordEvent(sessionId, {
                type: 'task_completed',
                data: {
                    taskIndex,
                    success,
                    duration,
                    notes
                },
                relativeTime: activeSession.startTime ? Date.now() - activeSession.startTime : 0
            });

            return true;

        } catch (error) {
            console.error('Error completing task:', error);
            throw error;
        }
    }

    async submitFeedback(sessionId, feedbackData) {
        try {
            const session = await this.db.getSession(sessionId);
            if (!session) {
                throw new Error('Session not found');
            }

            // Save feedback to database
            const feedback = await this.db.submitFeedback(
                sessionId,
                session.participant_id,
                feedbackData
            );

            // Record feedback event
            await this.recordEvent(sessionId, {
                type: 'feedback_submitted',
                data: {
                    npsScore: feedbackData.npsScore,
                    installRating: feedbackData.installRating,
                    trustLevel: feedbackData.trustLevel
                },
                relativeTime: 0
            });

            // Notify observers
            this.io.to(`session_${sessionId}`).emit('feedback_submitted', {
                sessionId,
                feedback
            });

            return feedback;

        } catch (error) {
            console.error('Error submitting feedback:', error);
            throw error;
        }
    }

    async getEvents(sessionId) {
        return await this.db.getSessionEvents(sessionId);
    }

    async getTaskCompletions(sessionId) {
        try {
            return await this.db.db.all(
                `SELECT * FROM task_completions 
                 WHERE session_id = ? 
                 ORDER BY task_index ASC`,
                [sessionId]
            );

        } catch (error) {
            console.error('Error getting task completions:', error);
            return [];
        }
    }

    async getSessionStatistics(sessionId) {
        try {
            const eventStats = await this.db.db.get(`
                SELECT 
                    COUNT(*) as total_events,
                    SUM(CASE WHEN event_type = 'task_completed' THEN 1 ELSE 0 END) as completed_tasks,
                    SUM(CASE WHEN event_type = 'error' THEN 1 ELSE 0 END) as errors
                FROM session_events 
                WHERE session_id = ?
            `, [sessionId]);

            const taskStats = await this.db.db.get(`
                SELECT 
                    COUNT(*) as total_tasks,
                    SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as successful_tasks,
                    AVG(duration_seconds) as avg_task_duration
                FROM task_completions 
                WHERE session_id = ?
            `, [sessionId]);

            return {
                events: eventStats,
                tasks: taskStats,
                completionRate: taskStats.total_tasks > 0 ? 
                    (taskStats.successful_tasks / taskStats.total_tasks) * 100 : 0
            };

        } catch (error) {
            console.error('Error getting session statistics:', error);
            return {};
        }
    }

    async analyzeEvent(sessionId, event) {
        try {
            // Automated analysis based on event type
            switch (event.event_type) {
                case 'error':
                    await this.handleErrorEvent(sessionId, event);
                    break;
                
                case 'task_completed':
                    await this.analyzeTaskCompletion(sessionId, event);
                    break;
                
                case 'user_frustration':
                    await this.handleFrustrationEvent(sessionId, event);
                    break;
                
                case 'long_pause':
                    await this.handleLongPause(sessionId, event);
                    break;
            }

        } catch (error) {
            console.error('Error analyzing event:', error);
            // Don't throw - analysis errors shouldn't break main functionality
        }
    }

    async handleErrorEvent(sessionId, event) {
        try {
            // Create issue ticket for errors
            const issueId = `issue_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
            
            await this.db.db.run(
                `INSERT INTO issues 
                 (id, session_id, issue_type, severity, title, description, status)
                 VALUES (?, ?, ?, ?, ?, ?, ?)`,
                [
                    issueId,
                    sessionId,
                    'error',
                    'medium',
                    'Session Error Detected',
                    JSON.stringify(event.event_data),
                    'open'
                ]
            );

            // Notify observers about critical errors
            if (event.event_data.severity === 'critical') {
                this.io.to(`session_${sessionId}`).emit('critical_error', {
                    sessionId,
                    error: event.event_data,
                    issueId
                });
            }

        } catch (error) {
            console.error('Error handling error event:', error);
        }
    }

    async analyzeTaskCompletion(sessionId, event) {
        try {
            const taskData = event.event_data;
            
            // Check for unusually long completion times
            if (taskData.duration > 600) { // 10 minutes
                await this.recordEvent(sessionId, {
                    type: 'analysis_alert',
                    data: {
                        alert: 'long_task_duration',
                        taskIndex: taskData.taskIndex,
                        duration: taskData.duration
                    },
                    relativeTime: event.relative_time
                });
            }

            // Check for task failure patterns
            if (!taskData.success) {
                const recentFailures = await this.db.db.get(`
                    SELECT COUNT(*) as count
                    FROM session_events 
                    WHERE session_id = ? 
                    AND event_type = 'task_completed'
                    AND JSON_EXTRACT(event_data, '$.success') = 0
                    AND timestamp > datetime('now', '-5 minutes')
                `, [sessionId]);

                if (recentFailures.count >= 2) {
                    this.io.to(`session_${sessionId}`).emit('failure_pattern', {
                        sessionId,
                        consecutiveFailures: recentFailures.count
                    });
                }
            }

        } catch (error) {
            console.error('Error analyzing task completion:', error);
        }
    }

    async handleFrustrationEvent(sessionId, event) {
        try {
            // Alert observers to potential user frustration
            this.io.to(`session_${sessionId}`).emit('user_frustration', {
                sessionId,
                level: event.event_data.level,
                context: event.event_data.context
            });

            // Log for analysis
            await this.logEvent(sessionId, 'frustration_detected', event.event_data);

        } catch (error) {
            console.error('Error handling frustration event:', error);
        }
    }

    async handleLongPause(sessionId, event) {
        try {
            // Check if user needs help
            const pauseDuration = event.event_data.duration;
            
            if (pauseDuration > 120) { // 2 minutes
                this.io.to(`session_${sessionId}`).emit('long_pause_detected', {
                    sessionId,
                    duration: pauseDuration,
                    suggestion: 'Consider offering assistance to participant'
                });
            }

        } catch (error) {
            console.error('Error handling long pause:', error);
        }
    }

    async joinSessionAsObserver(sessionId, observerId, socketId) {
        try {
            // Add to active session observers
            const activeSession = this.activeSessions.get(sessionId);
            if (activeSession) {
                activeSession.observers.add(observerId);
            }

            // Join socket room
            const socket = this.io.sockets.sockets.get(socketId);
            if (socket) {
                socket.join(`session_${sessionId}`);
            }

            // Send current session state to new observer
            const sessionData = await this.getById(sessionId);
            this.io.to(socketId).emit('session_state', sessionData);

            return true;

        } catch (error) {
            console.error('Error joining session as observer:', error);
            throw error;
        }
    }

    async leaveSessionAsObserver(sessionId, observerId, socketId) {
        try {
            // Remove from active session observers
            const activeSession = this.activeSessions.get(sessionId);
            if (activeSession) {
                activeSession.observers.delete(observerId);
            }

            // Leave socket room
            const socket = this.io.sockets.sockets.get(socketId);
            if (socket) {
                socket.leave(`session_${sessionId}`);
            }

            return true;

        } catch (error) {
            console.error('Error leaving session as observer:', error);
            throw error;
        }
    }

    async getActiveSessions() {
        return Array.from(this.activeSessions.values());
    }

    async logEvent(sessionId, eventType, data) {
        try {
            await this.db.db.run(
                `INSERT INTO system_logs (level, component, message, data)
                 VALUES (?, ?, ?, ?)`,
                ['info', 'SessionService', eventType, JSON.stringify({
                    sessionId,
                    ...data
                })]
            );
        } catch (error) {
            console.error('Error logging event:', error);
        }
    }

    async generateSessionReport(sessionId) {
        try {
            const session = await this.getById(sessionId);
            if (!session) {
                throw new Error('Session not found');
            }

            const report = {
                session: session,
                summary: {
                    duration: session.duration_minutes,
                    completionRate: session.statistics.completionRate,
                    errorCount: session.statistics.events.errors,
                    taskSuccessRate: session.statistics.tasks.successful_tasks / session.statistics.tasks.total_tasks * 100
                },
                timeline: session.events.map(event => ({
                    timestamp: event.timestamp,
                    relativeTime: event.relative_time,
                    type: event.event_type,
                    description: this.getEventDescription(event)
                })),
                recommendations: this.generateRecommendations(session),
                generatedAt: new Date().toISOString()
            };

            return report;

        } catch (error) {
            console.error('Error generating session report:', error);
            throw error;
        }
    }

    getEventDescription(event) {
        const descriptions = {
            'session_started': 'Session began',
            'task_started': `Started task: ${event.event_data.taskName}`,
            'task_completed': `Completed task ${event.event_data.success ? 'successfully' : 'with issues'}`,
            'error': `Error occurred: ${event.event_data.message}`,
            'feedback_submitted': 'Feedback submitted',
            'session_completed': 'Session completed'
        };

        return descriptions[event.event_type] || `Event: ${event.event_type}`;
    }

    generateRecommendations(session) {
        const recommendations = [];

        // Based on completion rate
        if (session.statistics.completionRate < 70) {
            recommendations.push('Low completion rate - review user interface design');
        }

        // Based on error count
        if (session.statistics.events.errors > 5) {
            recommendations.push('High error count - investigate technical issues');
        }

        // Based on duration
        if (session.duration_minutes > 60) {
            recommendations.push('Session exceeded expected duration - simplify user flow');
        }

        // Based on task failures
        const failedTasks = session.taskCompletions.filter(task => !task.success);
        if (failedTasks.length > 0) {
            recommendations.push(`Tasks with issues: ${failedTasks.map(t => t.task_name).join(', ')}`);
        }

        return recommendations;
    }
}