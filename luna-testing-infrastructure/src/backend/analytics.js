/**
 * Analytics Service - Tracks user behavior, events, and generates insights
 */

import { v4 as uuidv4 } from 'uuid';

export class AnalyticsService {
    constructor(database) {
        this.db = database;
        this.realTimeEvents = new Map(); // Store recent events for real-time processing
        this.aggregationCache = new Map(); // Cache for expensive aggregations
    }

    async trackEvent(sessionId, eventType, eventData = {}) {
        try {
            const eventId = uuidv4();
            const timestamp = new Date();

            // Enrich event data with context
            const enrichedData = await this.enrichEventData(sessionId, eventType, eventData);

            // Store in database
            await this.db.db.run(
                `INSERT INTO analytics_events (id, session_id, event_type, timestamp, data, user_agent, ip_address)
                 VALUES (?, ?, ?, ?, ?, ?, ?)`,
                [
                    eventId,
                    sessionId,
                    eventType,
                    timestamp,
                    JSON.stringify(enrichedData),
                    eventData.userAgent || null,
                    eventData.ipAddress || null
                ]
            );

            // Update real-time cache
            this.updateRealTimeCache(sessionId, eventType, enrichedData, timestamp);

            // Process event for real-time insights
            await this.processRealTimeEvent(sessionId, eventType, enrichedData);

            return eventId;

        } catch (error) {
            console.error('Event tracking error:', error);
            throw error;
        }
    }

    async enrichEventData(sessionId, eventType, originalData) {
        try {
            // Get session context
            const session = await this.db.getSession(sessionId);
            const participant = session ? await this.db.getParticipant(session.participant_id) : null;

            // Get previous events for context
            const recentEvents = await this.db.db.all(
                `SELECT event_type, timestamp FROM analytics_events 
                 WHERE session_id = ? AND timestamp > datetime('now', '-5 minutes')
                 ORDER BY timestamp DESC LIMIT 10`,
                [sessionId]
            );

            // Calculate timing metrics
            const sessionEvents = await this.db.db.all(
                `SELECT timestamp FROM analytics_events 
                 WHERE session_id = ? ORDER BY timestamp`,
                [sessionId]
            );

            const timingMetrics = this.calculateTimingMetrics(sessionEvents);

            return {
                ...originalData,
                context: {
                    participantSegment: participant?.segment || 'unknown',
                    participantTechLevel: participant?.tech_level || 'unknown',
                    sessionType: session?.session_type || 'unknown',
                    sessionDuration: timingMetrics.sessionDuration,
                    eventSequence: sessionEvents.length + 1,
                    timeSinceLastEvent: timingMetrics.timeSinceLastEvent,
                    recentEventTypes: recentEvents.map(e => e.event_type)
                },
                timing: timingMetrics,
                enrichedAt: new Date()
            };

        } catch (error) {
            console.error('Event enrichment error:', error);
            return originalData; // Return original data if enrichment fails
        }
    }

    calculateTimingMetrics(sessionEvents) {
        if (sessionEvents.length === 0) {
            return {
                sessionDuration: 0,
                timeSinceLastEvent: 0,
                averageTimeBetweenEvents: 0
            };
        }

        const now = new Date();
        const firstEvent = new Date(sessionEvents[0].timestamp);
        const lastEvent = sessionEvents.length > 1 ? new Date(sessionEvents[sessionEvents.length - 1].timestamp) : firstEvent;

        const sessionDuration = (now.getTime() - firstEvent.getTime()) / 1000; // seconds
        const timeSinceLastEvent = (now.getTime() - lastEvent.getTime()) / 1000;

        // Calculate average time between events
        let totalTimeBetween = 0;
        if (sessionEvents.length > 1) {
            for (let i = 1; i < sessionEvents.length; i++) {
                const prevTime = new Date(sessionEvents[i - 1].timestamp);
                const currTime = new Date(sessionEvents[i].timestamp);
                totalTimeBetween += (currTime.getTime() - prevTime.getTime()) / 1000;
            }
        }
        const averageTimeBetweenEvents = sessionEvents.length > 1 ? 
            totalTimeBetween / (sessionEvents.length - 1) : 0;

        return {
            sessionDuration: Math.round(sessionDuration),
            timeSinceLastEvent: Math.round(timeSinceLastEvent),
            averageTimeBetweenEvents: Math.round(averageTimeBetweenEvents * 100) / 100
        };
    }

    updateRealTimeCache(sessionId, eventType, data, timestamp) {
        if (!this.realTimeEvents.has(sessionId)) {
            this.realTimeEvents.set(sessionId, []);
        }

        const sessionEvents = this.realTimeEvents.get(sessionId);
        sessionEvents.push({
            type: eventType,
            data: data,
            timestamp: timestamp
        });

        // Keep only last 50 events per session to prevent memory bloat
        if (sessionEvents.length > 50) {
            sessionEvents.splice(0, sessionEvents.length - 50);
        }

        // Clean up old sessions (older than 4 hours)
        const fourHoursAgo = new Date(Date.now() - 4 * 60 * 60 * 1000);
        for (const [sessionId, events] of this.realTimeEvents.entries()) {
            if (events.length > 0 && new Date(events[events.length - 1].timestamp) < fourHoursAgo) {
                this.realTimeEvents.delete(sessionId);
            }
        }
    }

    async processRealTimeEvent(sessionId, eventType, data) {
        try {
            // Detect user struggles
            await this.detectUserStruggles(sessionId, eventType, data);

            // Update session progress
            await this.updateSessionProgress(sessionId, eventType, data);

            // Generate real-time alerts
            await this.generateRealTimeAlerts(sessionId, eventType, data);

            // Update live metrics
            await this.updateLiveMetrics(sessionId, eventType, data);

        } catch (error) {
            console.error('Real-time event processing error:', error);
            // Don't throw - this is background processing
        }
    }

    async detectUserStruggles(sessionId, eventType, data) {
        const sessionEvents = this.realTimeEvents.get(sessionId) || [];
        const recentEvents = sessionEvents.slice(-10); // Last 10 events

        // Detect struggle patterns
        const struggles = [];

        // Pattern 1: Multiple error events
        const errorEvents = recentEvents.filter(e => 
            e.type.includes('error') || e.type.includes('fail')
        );
        if (errorEvents.length >= 3) {
            struggles.push({
                type: 'multiple_errors',
                severity: 'high',
                evidence: errorEvents.map(e => e.type)
            });
        }

        // Pattern 2: Repeated same action
        const actionCounts = {};
        recentEvents.forEach(e => {
            actionCounts[e.type] = (actionCounts[e.type] || 0) + 1;
        });
        const repeatedActions = Object.entries(actionCounts)
            .filter(([action, count]) => count >= 4)
            .map(([action, count]) => ({ action, count }));

        if (repeatedActions.length > 0) {
            struggles.push({
                type: 'repeated_actions',
                severity: 'medium',
                evidence: repeatedActions
            });
        }

        // Pattern 3: Long inactivity followed by burst of activity
        if (recentEvents.length >= 3) {
            const lastThreeEvents = recentEvents.slice(-3);
            const timeDiffs = [];
            for (let i = 1; i < lastThreeEvents.length; i++) {
                const diff = (new Date(lastThreeEvents[i].timestamp).getTime() - 
                             new Date(lastThreeEvents[i-1].timestamp).getTime()) / 1000;
                timeDiffs.push(diff);
            }

            if (timeDiffs[0] > 120 && timeDiffs[1] < 5) { // 2+ min gap, then <5 sec
                struggles.push({
                    type: 'inactivity_burst',
                    severity: 'medium',
                    evidence: { inactivityTime: timeDiffs[0], burstTime: timeDiffs[1] }
                });
            }
        }

        // Record struggles
        for (const struggle of struggles) {
            await this.trackEvent(sessionId, 'user_struggle_detected', {
                struggleType: struggle.type,
                severity: struggle.severity,
                evidence: struggle.evidence,
                detectedAt: new Date(),
                autoDetected: true
            });
        }
    }

    async updateSessionProgress(sessionId, eventType, data) {
        try {
            // Define progress milestones
            const milestones = {
                'download_started': 10,
                'download_completed': 25,
                'install_started': 30,
                'install_completed': 60,
                'luna_launched': 80,
                'first_task_completed': 100
            };

            if (milestones[eventType]) {
                const progressValue = milestones[eventType];
                
                // Update cached progress
                const cacheKey = `session_progress_${sessionId}`;
                this.aggregationCache.set(cacheKey, {
                    progress: progressValue,
                    lastMilestone: eventType,
                    updatedAt: new Date()
                });

                // Track progress event
                await this.trackEvent(sessionId, 'progress_milestone', {
                    milestone: eventType,
                    progressValue: progressValue,
                    timestamp: new Date()
                });
            }

        } catch (error) {
            console.error('Session progress update error:', error);
        }
    }

    async generateRealTimeAlerts(sessionId, eventType, data) {
        const alerts = [];

        // Critical error alert
        if (eventType.includes('error') || eventType.includes('crash')) {
            alerts.push({
                type: 'critical_error',
                priority: 'high',
                message: `Critical error detected in session ${sessionId}: ${eventType}`,
                data: data
            });
        }

        // User requesting help
        if (eventType === 'help_requested' || eventType.includes('support')) {
            alerts.push({
                type: 'help_request',
                priority: 'medium',
                message: `User in session ${sessionId} has requested help`,
                data: data
            });
        }

        // Session taking too long
        const sessionEvents = this.realTimeEvents.get(sessionId) || [];
        if (sessionEvents.length > 0) {
            const sessionStart = sessionEvents[0].timestamp;
            const sessionDuration = (new Date().getTime() - new Date(sessionStart).getTime()) / (60 * 1000);
            
            if (sessionDuration > 45) { // 45 minutes
                alerts.push({
                    type: 'session_extended',
                    priority: 'low',
                    message: `Session ${sessionId} has been running for ${Math.round(sessionDuration)} minutes`,
                    data: { duration: sessionDuration }
                });
            }
        }

        // Process alerts
        for (const alert of alerts) {
            await this.processAlert(sessionId, alert);
        }
    }

    async processAlert(sessionId, alert) {
        try {
            // Store alert in database
            await this.db.db.run(
                `INSERT INTO analytics_alerts (id, session_id, alert_type, priority, message, data, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)`,
                [
                    uuidv4(),
                    sessionId,
                    alert.type,
                    alert.priority,
                    alert.message,
                    JSON.stringify(alert.data),
                    new Date()
                ]
            );

            // Log for real-time monitoring
            console.log(`ALERT [${alert.priority.toUpperCase()}]: ${alert.message}`);

        } catch (error) {
            console.error('Alert processing error:', error);
        }
    }

    async updateLiveMetrics(sessionId, eventType, data) {
        try {
            const cacheKey = `live_metrics_${sessionId}`;
            let metrics = this.aggregationCache.get(cacheKey) || {
                totalEvents: 0,
                uniqueEventTypes: new Set(),
                errorCount: 0,
                lastActivity: null,
                startTime: new Date()
            };

            // Update metrics
            metrics.totalEvents++;
            metrics.uniqueEventTypes.add(eventType);
            metrics.lastActivity = new Date();

            if (eventType.includes('error') || eventType.includes('fail')) {
                metrics.errorCount++;
            }

            // Calculate additional metrics
            const sessionDuration = (metrics.lastActivity.getTime() - metrics.startTime.getTime()) / (60 * 1000);
            const eventsPerMinute = metrics.totalEvents / Math.max(sessionDuration, 1);

            metrics.sessionDuration = Math.round(sessionDuration * 100) / 100;
            metrics.eventsPerMinute = Math.round(eventsPerMinute * 100) / 100;
            metrics.uniqueEventCount = metrics.uniqueEventTypes.size;

            // Update cache
            this.aggregationCache.set(cacheKey, metrics);

        } catch (error) {
            console.error('Live metrics update error:', error);
        }
    }

    async getSessionAnalytics(sessionId) {
        try {
            // Get all events for the session
            const events = await this.db.db.all(
                `SELECT * FROM analytics_events 
                 WHERE session_id = ? 
                 ORDER BY timestamp`,
                [sessionId]
            );

            if (events.length === 0) {
                return {
                    sessionId,
                    totalEvents: 0,
                    eventTimeline: [],
                    userBehavior: {},
                    performance: {}
                };
            }

            // Analyze event timeline
            const eventTimeline = this.analyzeEventTimeline(events);

            // Analyze user behavior patterns
            const userBehavior = this.analyzeUserBehavior(events);

            // Analyze performance metrics
            const performance = this.analyzePerformance(events);

            // Get alerts for this session
            const alerts = await this.db.db.all(
                `SELECT * FROM analytics_alerts 
                 WHERE session_id = ? 
                 ORDER BY created_at DESC`,
                [sessionId]
            );

            return {
                sessionId,
                totalEvents: events.length,
                eventTimeline,
                userBehavior,
                performance,
                alerts,
                generatedAt: new Date()
            };

        } catch (error) {
            console.error('Session analytics error:', error);
            throw error;
        }
    }

    analyzeEventTimeline(events) {
        const timeline = [];
        let currentPhase = 'setup';
        
        events.forEach((event, index) => {
            const eventData = JSON.parse(event.data || '{}');
            
            // Determine phase
            if (event.event_type.includes('download')) {
                currentPhase = 'download';
            } else if (event.event_type.includes('install')) {
                currentPhase = 'installation';
            } else if (event.event_type.includes('launch')) {
                currentPhase = 'first_use';
            } else if (event.event_type.includes('task')) {
                currentPhase = 'task_completion';
            }

            timeline.push({
                timestamp: event.timestamp,
                eventType: event.event_type,
                phase: currentPhase,
                sequenceNumber: index + 1,
                data: eventData.context || {}
            });
        });

        return timeline;
    }

    analyzeUserBehavior(events) {
        const behavior = {
            eventTypes: {},
            phases: {},
            errorPatterns: [],
            interactionPatterns: [],
            timingPatterns: {}
        };

        // Count event types
        events.forEach(event => {
            behavior.eventTypes[event.event_type] = (behavior.eventTypes[event.event_type] || 0) + 1;
        });

        // Analyze timing patterns
        if (events.length > 1) {
            const timeDiffs = [];
            for (let i = 1; i < events.length; i++) {
                const diff = (new Date(events[i].timestamp).getTime() - 
                             new Date(events[i-1].timestamp).getTime()) / 1000;
                timeDiffs.push(diff);
            }

            behavior.timingPatterns = {
                averageTimeBetweenEvents: timeDiffs.reduce((a, b) => a + b, 0) / timeDiffs.length,
                minTimeBetweenEvents: Math.min(...timeDiffs),
                maxTimeBetweenEvents: Math.max(...timeDiffs),
                totalSessionTime: timeDiffs.reduce((a, b) => a + b, 0)
            };
        }

        // Detect error patterns
        const errorEvents = events.filter(e => e.event_type.includes('error'));
        if (errorEvents.length > 0) {
            behavior.errorPatterns = errorEvents.map(event => ({
                type: event.event_type,
                timestamp: event.timestamp,
                context: JSON.parse(event.data || '{}').context || {}
            }));
        }

        return behavior;
    }

    analyzePerformance(events) {
        const performance = {
            efficiency: {},
            completion: {},
            quality: {}
        };

        // Task completion metrics
        const taskEvents = events.filter(e => 
            ['download_completed', 'install_completed', 'luna_launched', 'task_completed'].includes(e.event_type)
        );

        if (taskEvents.length > 0 && events.length > 0) {
            const sessionStart = new Date(events[0].timestamp);
            const taskCompletionTimes = {};

            taskEvents.forEach(event => {
                const completionTime = (new Date(event.timestamp).getTime() - sessionStart.getTime()) / (60 * 1000);
                taskCompletionTimes[event.event_type] = Math.round(completionTime * 100) / 100;
            });

            performance.completion = {
                taskCompletionTimes,
                completedTasks: taskEvents.length,
                completionRate: (taskEvents.length / 4) * 100 // Assuming 4 main tasks
            };
        }

        // Efficiency metrics
        const errorCount = events.filter(e => e.event_type.includes('error')).length;
        const helpRequests = events.filter(e => e.event_type === 'help_requested').length;

        performance.efficiency = {
            errorRate: (errorCount / events.length) * 100,
            helpRequestRate: (helpRequests / events.length) * 100,
            eventsPerTask: taskEvents.length > 0 ? events.length / taskEvents.length : 0
        };

        // Quality metrics
        const successEvents = events.filter(e => e.event_type.includes('completed') || e.event_type.includes('success'));
        performance.quality = {
            successRate: (successEvents.length / events.length) * 100,
            overallScore: Math.max(0, 100 - (errorCount * 10) - (helpRequests * 5))
        };

        return performance;
    }

    async getUserJourney(participantId) {
        try {
            // Get all sessions for this participant
            const sessions = await this.db.db.all(
                `SELECT * FROM sessions WHERE participant_id = ? ORDER BY created_at`,
                [participantId]
            );

            if (sessions.length === 0) {
                return { participantId, sessions: [], overallJourney: {} };
            }

            // Get analytics for each session
            const sessionAnalytics = [];
            for (const session of sessions) {
                const analytics = await this.getSessionAnalytics(session.id);
                sessionAnalytics.push({
                    session: session,
                    analytics: analytics
                });
            }

            // Analyze overall journey
            const overallJourney = this.analyzeOverallJourney(sessionAnalytics);

            return {
                participantId,
                sessions: sessionAnalytics,
                overallJourney,
                generatedAt: new Date()
            };

        } catch (error) {
            console.error('User journey analysis error:', error);
            throw error;
        }
    }

    analyzeOverallJourney(sessionAnalytics) {
        const journey = {
            progression: {},
            patterns: {},
            improvements: {},
            recommendations: []
        };

        if (sessionAnalytics.length === 0) return journey;

        // Analyze progression across sessions
        const errorRates = sessionAnalytics.map(sa => sa.analytics.performance.efficiency.errorRate || 0);
        const completionRates = sessionAnalytics.map(sa => sa.analytics.performance.completion.completionRate || 0);
        const sessionDurations = sessionAnalytics.map(sa => sa.session.duration_minutes || 0);

        journey.progression = {
            errorRateImprovement: errorRates.length > 1 ? errorRates[0] - errorRates[errorRates.length - 1] : 0,
            completionRateImprovement: completionRates.length > 1 ? completionRates[completionRates.length - 1] - completionRates[0] : 0,
            timeEfficiencyImprovement: sessionDurations.length > 1 ? sessionDurations[0] - sessionDurations[sessionDurations.length - 1] : 0
        };

        // Generate recommendations
        if (journey.progression.errorRateImprovement < 0) {
            journey.recommendations.push('Consider additional user guidance to reduce error rates');
        }
        if (journey.progression.completionRateImprovement > 20) {
            journey.recommendations.push('User shows strong learning curve - good candidate for advanced features');
        }

        return journey;
    }

    async getAggregatedInsights(options = {}) {
        try {
            const {
                dateFrom = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000), // 30 days ago
                dateTo = new Date(),
                segment,
                sessionType
            } = options;

            // Build base query
            let sessionFilter = 'WHERE s.created_at BETWEEN ? AND ?';
            const params = [dateFrom, dateTo];

            if (segment) {
                sessionFilter += ' AND p.segment = ?';
                params.push(segment);
            }
            if (sessionType) {
                sessionFilter += ' AND s.session_type = ?';
                params.push(sessionType);
            }

            // Get session overview
            const sessionOverview = await this.db.db.get(`
                SELECT 
                    COUNT(*) as total_sessions,
                    SUM(CASE WHEN s.status = 'completed' THEN 1 ELSE 0 END) as completed_sessions,
                    AVG(s.duration_minutes) as avg_duration,
                    AVG(CASE WHEN s.metrics IS NOT NULL 
                        THEN JSON_EXTRACT(s.metrics, '$.completionRate') 
                        ELSE NULL END) as avg_completion_rate
                FROM sessions s
                JOIN participants p ON s.participant_id = p.id
                ${sessionFilter}
            `, params);

            // Get event insights
            const eventInsights = await this.db.db.all(`
                SELECT 
                    ae.event_type,
                    COUNT(*) as event_count,
                    COUNT(DISTINCT ae.session_id) as sessions_with_event
                FROM analytics_events ae
                JOIN sessions s ON ae.session_id = s.id
                JOIN participants p ON s.participant_id = p.id
                ${sessionFilter.replace('s.created_at', 'ae.timestamp')}
                GROUP BY ae.event_type
                ORDER BY event_count DESC
                LIMIT 20
            `, params);

            // Get error analysis
            const errorAnalysis = await this.db.db.all(`
                SELECT 
                    ae.event_type,
                    COUNT(*) as error_count,
                    COUNT(DISTINCT ae.session_id) as affected_sessions
                FROM analytics_events ae
                JOIN sessions s ON ae.session_id = s.id
                JOIN participants p ON s.participant_id = p.id
                ${sessionFilter.replace('s.created_at', 'ae.timestamp')}
                AND ae.event_type LIKE '%error%'
                GROUP BY ae.event_type
                ORDER BY error_count DESC
            `, params);

            // Get performance by segment
            const segmentPerformance = await this.db.db.all(`
                SELECT 
                    p.segment,
                    COUNT(*) as session_count,
                    AVG(s.duration_minutes) as avg_duration,
                    AVG(CASE WHEN s.metrics IS NOT NULL 
                        THEN JSON_EXTRACT(s.metrics, '$.completionRate') 
                        ELSE NULL END) as avg_completion_rate,
                    SUM(CASE WHEN s.status = 'completed' THEN 1 ELSE 0 END) as completed_count
                FROM sessions s
                JOIN participants p ON s.participant_id = p.id
                ${sessionFilter}
                GROUP BY p.segment
            `, params);

            return {
                overview: sessionOverview,
                eventInsights: eventInsights,
                errorAnalysis: errorAnalysis,
                segmentPerformance: segmentPerformance,
                dateRange: { from: dateFrom, to: dateTo },
                generatedAt: new Date()
            };

        } catch (error) {
            console.error('Aggregated insights error:', error);
            throw error;
        }
    }

    async generateReport(reportType, options = {}) {
        try {
            switch (reportType) {
                case 'session_summary':
                    return await this.generateSessionSummaryReport(options);
                case 'user_experience':
                    return await this.generateUserExperienceReport(options);
                case 'technical_issues':
                    return await this.generateTechnicalIssuesReport(options);
                case 'performance_trends':
                    return await this.generatePerformanceTrendsReport(options);
                default:
                    throw new Error(`Unknown report type: ${reportType}`);
            }
        } catch (error) {
            console.error('Report generation error:', error);
            throw error;
        }
    }

    async generateSessionSummaryReport(options) {
        const insights = await this.getAggregatedInsights(options);
        
        return {
            reportType: 'session_summary',
            title: 'Luna User Testing Session Summary',
            summary: {
                totalSessions: insights.overview.total_sessions,
                completionRate: (insights.overview.completed_sessions / insights.overview.total_sessions) * 100,
                averageDuration: insights.overview.avg_duration,
                averageCompletionScore: insights.overview.avg_completion_rate
            },
            topEvents: insights.eventInsights.slice(0, 10),
            segmentBreakdown: insights.segmentPerformance,
            recommendations: this.generateRecommendations(insights),
            generatedAt: new Date()
        };
    }

    generateRecommendations(insights) {
        const recommendations = [];

        // Completion rate recommendations
        const completionRate = (insights.overview.completed_sessions / insights.overview.total_sessions) * 100;
        if (completionRate < 70) {
            recommendations.push({
                type: 'completion',
                priority: 'high',
                message: 'Session completion rate is below 70%. Consider simplifying the installation process or providing better guidance.'
            });
        }

        // Error rate recommendations
        const totalEvents = insights.eventInsights.reduce((sum, event) => sum + event.event_count, 0);
        const totalErrors = insights.errorAnalysis.reduce((sum, error) => sum + error.error_count, 0);
        const errorRate = (totalErrors / totalEvents) * 100;

        if (errorRate > 15) {
            recommendations.push({
                type: 'errors',
                priority: 'high',
                message: 'High error rate detected. Focus on improving error handling and user feedback.'
            });
        }

        // Segment-specific recommendations
        insights.segmentPerformance.forEach(segment => {
            const segmentCompletionRate = (segment.completed_count / segment.session_count) * 100;
            if (segmentCompletionRate < 50) {
                recommendations.push({
                    type: 'segment',
                    priority: 'medium',
                    message: `${segment.segment} users have low completion rates. Consider targeted improvements for this user group.`
                });
            }
        });

        return recommendations;
    }

    async getRealTimeMetrics() {
        const realTimeData = {
            activeSessions: this.realTimeEvents.size,
            totalEventsLastHour: 0,
            topEventTypes: {},
            strugglingSessionsCount: 0,
            averageSessionDuration: 0
        };

        // Aggregate data from real-time cache
        for (const [sessionId, events] of this.realTimeEvents.entries()) {
            const lastHourEvents = events.filter(e => 
                new Date(e.timestamp).getTime() > Date.now() - 60 * 60 * 1000
            );
            
            realTimeData.totalEventsLastHour += lastHourEvents.length;

            // Count event types
            lastHourEvents.forEach(event => {
                realTimeData.topEventTypes[event.type] = (realTimeData.topEventTypes[event.type] || 0) + 1;
            });

            // Check for struggling sessions
            const errorEvents = events.filter(e => e.type.includes('error'));
            if (errorEvents.length >= 3) {
                realTimeData.strugglingSessionsCount++;
            }

            // Calculate session duration
            if (events.length > 0) {
                const duration = (new Date().getTime() - new Date(events[0].timestamp).getTime()) / (60 * 1000);
                realTimeData.averageSessionDuration = 
                    (realTimeData.averageSessionDuration + duration) / 2;
            }
        }

        // Convert topEventTypes to sorted array
        realTimeData.topEventTypes = Object.entries(realTimeData.topEventTypes)
            .sort(([,a], [,b]) => b - a)
            .slice(0, 10)
            .map(([type, count]) => ({ type, count }));

        realTimeData.averageSessionDuration = Math.round(realTimeData.averageSessionDuration * 100) / 100;

        return realTimeData;
    }

    async cleanup() {
        try {
            // Clear old real-time events (older than 4 hours)
            const fourHoursAgo = new Date(Date.now() - 4 * 60 * 60 * 1000);
            for (const [sessionId, events] of this.realTimeEvents.entries()) {
                if (events.length > 0 && new Date(events[events.length - 1].timestamp) < fourHoursAgo) {
                    this.realTimeEvents.delete(sessionId);
                }
            }

            // Clear old aggregation cache (older than 1 hour)
            const oneHourAgo = new Date(Date.now() - 60 * 60 * 1000);
            for (const [key, data] of this.aggregationCache.entries()) {
                if (data.updatedAt && new Date(data.updatedAt) < oneHourAgo) {
                    this.aggregationCache.delete(key);
                }
            }

            console.log(`Analytics cleanup completed. Real-time events: ${this.realTimeEvents.size}, Cache entries: ${this.aggregationCache.size}`);

        } catch (error) {
            console.error('Analytics cleanup error:', error);
        }
    }
}