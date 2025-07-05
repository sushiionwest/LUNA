/**
 * Session Orchestrator - Coordinates testing sessions with Luna VM instances
 */

export class SessionOrchestrator {
    constructor(sessionService, lunaInstallerService, analyticsService, fileService) {
        this.sessions = sessionService;
        this.installer = lunaInstallerService;
        this.analytics = analyticsService;
        this.files = fileService;
        
        this.orchestratedSessions = new Map(); // Track session orchestration data
    }

    async startIntegratedSession(sessionId, facilitatorId, options = {}) {
        try {
            // Get session and participant data
            const session = await this.sessions.getById(sessionId);
            if (!session) {
                throw new Error('Session not found');
            }

            const participant = session.participant;
            if (!participant) {
                throw new Error('Participant data not found');
            }

            // Start the testing session in the infrastructure
            const startedSession = await this.sessions.start(sessionId, facilitatorId);

            // Start the Luna installer VM
            const vmInstance = await this.installer.startLunaInstaller(sessionId, participant, {
                enableScreenRecording: options.enableScreenRecording !== false,
                enableAutoScreenshots: options.enableAutoScreenshots !== false,
                debugMode: options.debugMode || participant.tech_level === 'advanced',
                guidedMode: options.guidedMode || participant.tech_level === 'non-technical'
            });

            // Create orchestration tracking
            const orchestrationData = {
                sessionId: sessionId,
                vmInstanceId: vmInstance.vmInstanceId,
                participantId: participant.id,
                facilitatorId: facilitatorId,
                startTime: new Date(),
                status: 'active',
                milestones: [],
                realTimeEvents: [],
                installationStages: {
                    vm_started: new Date(),
                    installer_launched: null,
                    download_started: null,
                    download_completed: null,
                    installation_started: null,
                    installation_completed: null,
                    first_task_attempted: null,
                    session_completed: null
                },
                metrics: {
                    totalEvents: 0,
                    userInteractions: 0,
                    errors: 0,
                    screenshots: 0,
                    installationDuration: null,
                    taskCompletionTime: null
                }
            };

            this.orchestratedSessions.set(sessionId, orchestrationData);

            // Set up event forwarding and processing
            await this.setupEventProcessing(sessionId);

            // Record session start in analytics
            await this.analytics.trackEvent(sessionId, 'integrated_session_started', {
                vmInstanceId: vmInstance.vmInstanceId,
                participantSegment: participant.segment,
                facilitatorId: facilitatorId,
                sessionConfig: vmInstance.config
            });

            return {
                session: startedSession,
                vmInstance: vmInstance,
                orchestrationId: sessionId,
                status: 'active',
                websocketEndpoint: `ws://localhost:${process.env.LUNA_WS_PORT || 8080}`,
                monitoringUrl: `http://localhost:3001/api/sessions/${sessionId}/monitor`
            };

        } catch (error) {
            console.error('Error starting integrated session:', error);
            
            // Cleanup on failure
            try {
                await this.installer.stopLunaInstaller(sessionId, 'startup_failed');
            } catch (cleanupError) {
                console.error('Error during cleanup:', cleanupError);
            }
            
            throw error;
        }
    }

    async setupEventProcessing(sessionId) {
        const orchestrationData = this.orchestratedSessions.get(sessionId);
        if (!orchestrationData) {
            return;
        }

        // This would be called by the main session service when events are received
        orchestrationData.eventProcessor = async (eventType, eventData) => {
            await this.processSessionEvent(sessionId, eventType, eventData);
        };
    }

    async processSessionEvent(sessionId, eventType, eventData) {
        const orchestrationData = this.orchestratedSessions.get(sessionId);
        if (!orchestrationData) {
            return;
        }

        // Update metrics
        orchestrationData.metrics.totalEvents++;
        orchestrationData.realTimeEvents.push({
            type: eventType,
            data: eventData,
            timestamp: new Date()
        });

        // Process installer-specific events
        if (eventData.source === 'luna_installer') {
            await this.processInstallerEvent(sessionId, eventType, eventData, orchestrationData);
        }

        // Process user interaction events
        if (eventType.includes('user_') || eventType.includes('click') || eventType.includes('input')) {
            orchestrationData.metrics.userInteractions++;
        }

        // Process error events
        if (eventType.includes('error') || eventType.includes('fail')) {
            orchestrationData.metrics.errors++;
            
            // Check if critical error requiring intervention
            if (this.isCriticalError(eventType, eventData)) {
                await this.handleCriticalError(sessionId, eventType, eventData);
            }
        }

        // Process screenshot events
        if (eventType === 'screenshot_captured') {
            orchestrationData.metrics.screenshots++;
        }

        // Update orchestration data
        this.orchestratedSessions.set(sessionId, orchestrationData);

        // Check for milestone completion
        await this.checkMilestones(sessionId, eventType, eventData);
    }

    async processInstallerEvent(sessionId, eventType, eventData, orchestrationData) {
        const stages = orchestrationData.installationStages;
        const now = new Date();

        switch (eventType) {
            case 'installer_launched':
                stages.installer_launched = now;
                break;

            case 'download_started':
                stages.download_started = now;
                break;

            case 'download_completed':
                stages.download_completed = now;
                break;

            case 'installation_started':
                stages.installation_started = now;
                break;

            case 'installation_completed':
                stages.installation_completed = now;
                if (stages.installation_started) {
                    orchestrationData.metrics.installationDuration = 
                        now.getTime() - stages.installation_started.getTime();
                }
                break;

            case 'first_task_attempted':
                stages.first_task_attempted = now;
                if (stages.vm_started) {
                    orchestrationData.metrics.taskCompletionTime = 
                        now.getTime() - stages.vm_started.getTime();
                }
                break;

            case 'installation_failed':
            case 'vm_error':
                orchestrationData.status = 'failed';
                orchestrationData.error = eventData.error;
                break;
        }
    }

    isCriticalError(eventType, eventData) {
        const criticalEvents = [
            'vm_error',
            'installation_failed',
            'network_error',
            'permission_denied',
            'out_of_memory',
            'disk_full'
        ];

        return criticalEvents.includes(eventType) || 
               (eventData.severity && eventData.severity === 'critical');
    }

    async handleCriticalError(sessionId, eventType, eventData) {
        try {
            const orchestrationData = this.orchestratedSessions.get(sessionId);
            
            // Log critical error
            console.error(`Critical error in session ${sessionId}: ${eventType}`, eventData);

            // Record in analytics
            await this.analytics.trackEvent(sessionId, 'critical_error_detected', {
                originalEventType: eventType,
                errorData: eventData,
                sessionDuration: Date.now() - orchestrationData.startTime.getTime(),
                autoHandled: true
            });

            // Attempt automatic recovery
            const recoveryAction = await this.attemptRecovery(sessionId, eventType, eventData);

            // Notify facilitator/monitoring system
            // This would integrate with the WebSocket system to send real-time alerts

        } catch (error) {
            console.error('Error handling critical error:', error);
        }
    }

    async attemptRecovery(sessionId, errorType, errorData) {
        const recoveryActions = {
            'vm_error': async () => {
                // Try restarting the VM
                await this.installer.stopLunaInstaller(sessionId, 'error_recovery');
                // Would need participant data to restart
                return 'vm_restart_attempted';
            },
            'installation_failed': async () => {
                // Clear installer cache and retry
                await this.analytics.trackEvent(sessionId, 'installation_retry_initiated', {
                    originalError: errorData
                });
                return 'installation_retry_initiated';
            },
            'network_error': async () => {
                // Wait and retry network operations
                await new Promise(resolve => setTimeout(resolve, 5000));
                return 'network_retry_attempted';
            }
        };

        const recoveryAction = recoveryActions[errorType];
        if (recoveryAction) {
            try {
                return await recoveryAction();
            } catch (error) {
                console.error('Recovery action failed:', error);
                return 'recovery_failed';
            }
        }

        return 'no_recovery_available';
    }

    async checkMilestones(sessionId, eventType, eventData) {
        const orchestrationData = this.orchestratedSessions.get(sessionId);
        const milestones = [
            { name: 'vm_ready', event: 'vm_status_running' },
            { name: 'installer_started', event: 'installer_launched' },
            { name: 'download_complete', event: 'download_completed' },
            { name: 'installation_complete', event: 'installation_completed' },
            { name: 'first_interaction', event: 'user_interaction' },
            { name: 'task_attempted', event: 'task_started' },
            { name: 'task_completed', event: 'task_completed' }
        ];

        for (const milestone of milestones) {
            if (eventType === milestone.event) {
                const existingMilestone = orchestrationData.milestones.find(m => m.name === milestone.name);
                if (!existingMilestone) {
                    orchestrationData.milestones.push({
                        name: milestone.name,
                        timestamp: new Date(),
                        eventType: eventType,
                        eventData: eventData
                    });

                    // Record milestone in analytics
                    await this.analytics.trackEvent(sessionId, 'milestone_reached', {
                        milestone: milestone.name,
                        sessionDuration: Date.now() - orchestrationData.startTime.getTime()
                    });
                }
            }
        }
    }

    async completeIntegratedSession(sessionId, reason = 'completed') {
        try {
            const orchestrationData = this.orchestratedSessions.get(sessionId);
            if (!orchestrationData) {
                throw new Error('No orchestrated session found');
            }

            orchestrationData.status = 'completing';
            orchestrationData.endTime = new Date();
            orchestrationData.installationStages.session_completed = new Date();

            // Stop the Luna installer
            await this.installer.stopLunaInstaller(sessionId, reason);

            // Complete the testing session
            const completedSession = await this.sessions.complete(sessionId, reason);

            // Calculate final metrics
            const finalMetrics = await this.calculateFinalMetrics(orchestrationData);

            // Save comprehensive session report
            await this.saveSessionReport(sessionId, orchestrationData, finalMetrics);

            // Record completion in analytics
            await this.analytics.trackEvent(sessionId, 'integrated_session_completed', {
                reason: reason,
                totalDuration: orchestrationData.endTime.getTime() - orchestrationData.startTime.getTime(),
                finalMetrics: finalMetrics,
                milestonesReached: orchestrationData.milestones.length
            });

            // Cleanup orchestration data
            this.orchestratedSessions.delete(sessionId);

            return {
                session: completedSession,
                orchestrationData: orchestrationData,
                finalMetrics: finalMetrics,
                status: 'completed'
            };

        } catch (error) {
            console.error('Error completing integrated session:', error);
            throw error;
        }
    }

    async calculateFinalMetrics(orchestrationData) {
        const stages = orchestrationData.installationStages;
        const metrics = orchestrationData.metrics;

        return {
            // Duration metrics
            totalSessionDuration: orchestrationData.endTime.getTime() - orchestrationData.startTime.getTime(),
            installationDuration: metrics.installationDuration,
            taskCompletionTime: metrics.taskCompletionTime,
            
            // Interaction metrics
            totalEvents: metrics.totalEvents,
            userInteractions: metrics.userInteractions,
            errorsEncountered: metrics.errors,
            screenshotsCaptured: metrics.screenshots,
            
            // Success metrics
            milestonesReached: orchestrationData.milestones.length,
            installationCompleted: !!stages.installation_completed,
            tasksAttempted: !!stages.first_task_attempted,
            
            // Performance metrics
            eventsPerMinute: metrics.totalEvents / ((orchestrationData.endTime.getTime() - orchestrationData.startTime.getTime()) / 60000),
            errorRate: (metrics.errors / metrics.totalEvents) * 100,
            
            // Stage timings
            timeToInstaller: stages.installer_launched ? 
                stages.installer_launched.getTime() - stages.vm_started.getTime() : null,
            timeToDownload: stages.download_started ? 
                stages.download_started.getTime() - stages.vm_started.getTime() : null,
            downloadDuration: (stages.download_started && stages.download_completed) ? 
                stages.download_completed.getTime() - stages.download_started.getTime() : null,
            timeToFirstTask: stages.first_task_attempted ? 
                stages.first_task_attempted.getTime() - stages.vm_started.getTime() : null
        };
    }

    async saveSessionReport(sessionId, orchestrationData, finalMetrics) {
        try {
            const reportData = {
                sessionId: sessionId,
                orchestrationSummary: {
                    vmInstanceId: orchestrationData.vmInstanceId,
                    participantId: orchestrationData.participantId,
                    facilitatorId: orchestrationData.facilitatorId,
                    startTime: orchestrationData.startTime,
                    endTime: orchestrationData.endTime,
                    status: orchestrationData.status
                },
                installationStages: orchestrationData.installationStages,
                milestones: orchestrationData.milestones,
                finalMetrics: finalMetrics,
                eventSummary: {
                    totalEvents: orchestrationData.realTimeEvents.length,
                    eventTypes: this.getEventTypeSummary(orchestrationData.realTimeEvents),
                    timelineHighlights: this.getTimelineHighlights(orchestrationData.realTimeEvents)
                },
                generatedAt: new Date()
            };

            const reportJson = JSON.stringify(reportData, null, 2);
            
            // Save report via file service
            await this.files.saveFile(Buffer.from(reportJson), {
                originalName: `integrated-session-report-${sessionId}.json`,
                mimeType: 'application/json',
                category: 'logs',
                sessionId: sessionId,
                source: 'session_orchestrator'
            });

            return reportData;

        } catch (error) {
            console.error('Error saving session report:', error);
        }
    }

    getEventTypeSummary(events) {
        const summary = {};
        events.forEach(event => {
            summary[event.type] = (summary[event.type] || 0) + 1;
        });
        return summary;
    }

    getTimelineHighlights(events) {
        const highlights = [];
        const importantEvents = [
            'installer_launched',
            'download_started',
            'download_completed',
            'installation_started',
            'installation_completed',
            'first_task_attempted',
            'task_completed',
            'error',
            'critical_error'
        ];

        events.forEach(event => {
            if (importantEvents.some(important => event.type.includes(important))) {
                highlights.push({
                    timestamp: event.timestamp,
                    eventType: event.type,
                    significance: this.getEventSignificance(event.type)
                });
            }
        });

        return highlights.sort((a, b) => new Date(a.timestamp) - new Date(b.timestamp));
    }

    getEventSignificance(eventType) {
        if (eventType.includes('error') || eventType.includes('fail')) return 'critical';
        if (eventType.includes('completed') || eventType.includes('success')) return 'success';
        if (eventType.includes('started') || eventType.includes('launched')) return 'milestone';
        return 'normal';
    }

    async getSessionStatus(sessionId) {
        const orchestrationData = this.orchestratedSessions.get(sessionId);
        if (!orchestrationData) {
            return { status: 'not_found' };
        }

        const vmStatus = await this.installer.getVMStatus(sessionId);
        
        return {
            sessionId: sessionId,
            status: orchestrationData.status,
            startTime: orchestrationData.startTime,
            duration: Date.now() - orchestrationData.startTime.getTime(),
            vmStatus: vmStatus,
            milestones: orchestrationData.milestones,
            metrics: orchestrationData.metrics,
            installationStages: orchestrationData.installationStages,
            lastEventTime: orchestrationData.realTimeEvents.length > 0 ? 
                orchestrationData.realTimeEvents[orchestrationData.realTimeEvents.length - 1].timestamp : null
        };
    }

    async listActiveSessions() {
        const activeSessions = [];
        for (const [sessionId, orchestrationData] of this.orchestratedSessions) {
            const sessionStatus = await this.getSessionStatus(sessionId);
            activeSessions.push(sessionStatus);
        }
        return activeSessions;
    }

    async getOrchestrationStatistics() {
        const stats = {
            activeSessions: this.orchestratedSessions.size,
            totalSessionsToday: 0,
            avgSessionDuration: 0,
            avgInstallationTime: 0,
            successRate: 0,
            commonIssues: []
        };

        // Get installer statistics
        const installerStats = await this.installer.getInstallerStatistics();
        
        return {
            ...stats,
            installer: installerStats,
            generatedAt: new Date()
        };
    }
}