/**
 * Luna Installer Service - Manages VM instances and installer sessions
 */

import { spawn, exec } from 'child_process';
import { v4 as uuidv4 } from 'uuid';
import fs from 'fs/promises';
import path from 'path';
import WebSocket from 'ws';

export class LunaInstallerService {
    constructor(database, config = {}) {
        this.db = database;
        this.config = {
            vmBasePath: config.vmBasePath || path.join(process.cwd(), '../../luna-one-click'),
            vmExecutable: config.vmExecutable || 'build-one-click-luna.sh',
            maxConcurrentVMs: config.maxConcurrentVMs || 10,
            vmTimeout: config.vmTimeout || 60 * 60 * 1000, // 1 hour
            screenshotInterval: config.screenshotInterval || 30000, // 30 seconds
            ...config
        };

        this.activeVMs = new Map(); // Track running VM instances
        this.vmSessions = new Map(); // Map session IDs to VM instances
        this.websocketServer = null;
        this.clientConnections = new Map(); // Track WebSocket connections

        this.initializeWebSocketServer();
    }

    initializeWebSocketServer() {
        // Create WebSocket server for real-time communication with VM instances
        this.websocketServer = new WebSocket.Server({ 
            port: process.env.LUNA_WS_PORT || 8080,
            verifyClient: (info) => {
                // Basic verification - could be enhanced with authentication
                return true;
            }
        });

        this.websocketServer.on('connection', (ws, req) => {
            const connectionId = uuidv4();
            this.clientConnections.set(connectionId, ws);

            console.log(`Luna installer client connected: ${connectionId}`);

            ws.on('message', async (data) => {
                try {
                    const message = JSON.parse(data.toString());
                    await this.handleInstallerMessage(connectionId, message);
                } catch (error) {
                    console.error('Error processing installer message:', error);
                }
            });

            ws.on('close', () => {
                this.clientConnections.delete(connectionId);
                console.log(`Luna installer client disconnected: ${connectionId}`);
            });

            ws.on('error', (error) => {
                console.error('WebSocket error:', error);
                this.clientConnections.delete(connectionId);
            });
        });

        console.log(`Luna installer WebSocket server running on port ${process.env.LUNA_WS_PORT || 8080}`);
    }

    async handleInstallerMessage(connectionId, message) {
        const { sessionId, eventType, data, timestamp } = message;

        if (!sessionId) {
            console.warn('Received message without session ID');
            return;
        }

        try {
            // Forward events to the main testing infrastructure
            await this.forwardEventToTestingInfrastructure(sessionId, eventType, {
                ...data,
                source: 'luna_installer',
                connectionId: connectionId,
                installerTimestamp: timestamp
            });

            // Handle installer-specific events
            await this.processInstallerEvent(sessionId, eventType, data);

        } catch (error) {
            console.error('Error handling installer message:', error);
        }
    }

    async forwardEventToTestingInfrastructure(sessionId, eventType, data) {
        try {
            // Make HTTP request to testing infrastructure
            const response = await fetch(`http://localhost:3001/api/sessions/${sessionId}/events`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    eventType: eventType,
                    data: data
                })
            });

            if (!response.ok) {
                throw new Error(`Failed to forward event: ${response.statusText}`);
            }

        } catch (error) {
            console.error('Error forwarding event to testing infrastructure:', error);
        }
    }

    async processInstallerEvent(sessionId, eventType, data) {
        const vmInstance = this.vmSessions.get(sessionId);
        if (!vmInstance) {
            console.warn(`No VM instance found for session ${sessionId}`);
            return;
        }

        switch (eventType) {
            case 'installation_started':
                vmInstance.status = 'installing';
                vmInstance.installationStartTime = new Date();
                break;

            case 'installation_progress':
                vmInstance.installationProgress = data.progress || 0;
                break;

            case 'installation_completed':
                vmInstance.status = 'completed';
                vmInstance.installationEndTime = new Date();
                break;

            case 'installation_failed':
                vmInstance.status = 'failed';
                vmInstance.error = data.error;
                break;

            case 'user_interaction':
                if (!vmInstance.userInteractions) {
                    vmInstance.userInteractions = [];
                }
                vmInstance.userInteractions.push({
                    type: data.type,
                    timestamp: new Date(),
                    details: data
                });
                break;

            case 'screenshot_captured':
                await this.saveScreenshot(sessionId, data.screenshot, data.metadata);
                break;

            case 'vm_error':
                console.error(`VM error in session ${sessionId}:`, data.error);
                vmInstance.status = 'error';
                vmInstance.error = data.error;
                break;
        }

        // Update VM instance in our tracking
        this.vmSessions.set(sessionId, vmInstance);
    }

    async startLunaInstaller(sessionId, participantData, options = {}) {
        try {
            if (this.activeVMs.size >= this.config.maxConcurrentVMs) {
                throw new Error('Maximum concurrent VM limit reached');
            }

            const vmInstanceId = uuidv4();
            const vmInstance = {
                id: vmInstanceId,
                sessionId: sessionId,
                participantId: participantData.id,
                status: 'starting',
                startTime: new Date(),
                config: {
                    operatingSystem: participantData.operating_system,
                    techLevel: participantData.tech_level,
                    segment: participantData.segment,
                    ...options
                },
                process: null,
                websocketConnection: null,
                screenshots: [],
                logs: [],
                userInteractions: [],
                installationProgress: 0
            };

            // Generate VM configuration
            const vmConfig = await this.generateVMConfig(vmInstance);
            
            // Save VM configuration
            const configPath = path.join(this.config.vmBasePath, 'sessions', `session-${sessionId}-config.json`);
            await fs.mkdir(path.dirname(configPath), { recursive: true });
            await fs.writeFile(configPath, JSON.stringify(vmConfig, null, 2));

            // Start VM process
            const vmProcess = await this.launchVM(vmInstance, configPath);
            vmInstance.process = vmProcess;

            // Track the VM instance
            this.activeVMs.set(vmInstanceId, vmInstance);
            this.vmSessions.set(sessionId, vmInstance);

            // Set up automatic cleanup timeout
            setTimeout(() => {
                this.cleanupVMInstance(vmInstanceId, 'timeout');
            }, this.config.vmTimeout);

            // Start screenshot capture
            this.startScreenshotCapture(vmInstance);

            await this.logInstallerEvent('vm_started', sessionId, {
                vmInstanceId: vmInstanceId,
                participantId: participantData.id,
                config: vmConfig
            });

            return {
                vmInstanceId: vmInstanceId,
                sessionId: sessionId,
                status: 'starting',
                config: vmConfig,
                websocketPort: process.env.LUNA_WS_PORT || 8080
            };

        } catch (error) {
            console.error('Error starting Luna installer:', error);
            throw error;
        }
    }

    async generateVMConfig(vmInstance) {
        return {
            sessionId: vmInstance.sessionId,
            participantId: vmInstance.participantId,
            vmInstanceId: vmInstance.id,
            operatingSystem: vmInstance.config.operatingSystem,
            techLevel: vmInstance.config.techLevel,
            segment: vmInstance.config.segment,
            
            // VM settings
            memory: this.getMemoryForSegment(vmInstance.config.segment),
            cpus: this.getCPUsForSegment(vmInstance.config.segment),
            
            // Network settings
            websocketEndpoint: `ws://localhost:${process.env.LUNA_WS_PORT || 8080}`,
            testingApiEndpoint: 'http://localhost:3001/api',
            
            // Installer settings
            installerMode: 'testing',
            debugMode: vmInstance.config.techLevel === 'advanced',
            guidedMode: vmInstance.config.techLevel === 'non-technical',
            autoScreenshots: true,
            screenshotInterval: this.config.screenshotInterval,
            
            // Session settings
            maxSessionTime: this.config.vmTimeout,
            autoCleanup: true,
            
            // Monitoring
            enableEventTracking: true,
            enablePerformanceMonitoring: true,
            enableErrorReporting: true
        };
    }

    getMemoryForSegment(segment) {
        const memoryMap = {
            'non-technical': 2048,    // 2GB - lighter workload
            'semi-technical': 4096,   // 4GB - moderate workload
            'technical': 6144         // 6GB - heavier testing
        };
        return memoryMap[segment] || 4096;
    }

    getCPUsForSegment(segment) {
        const cpuMap = {
            'non-technical': 2,       // 2 CPUs
            'semi-technical': 3,      // 3 CPUs
            'technical': 4            // 4 CPUs
        };
        return cpuMap[segment] || 2;
    }

    async launchVM(vmInstance, configPath) {
        return new Promise((resolve, reject) => {
            const vmScript = path.join(this.config.vmBasePath, this.config.vmExecutable);
            
            const vmProcess = spawn('bash', [vmScript, configPath], {
                cwd: this.config.vmBasePath,
                stdio: ['pipe', 'pipe', 'pipe'],
                env: {
                    ...process.env,
                    LUNA_SESSION_ID: vmInstance.sessionId,
                    LUNA_VM_ID: vmInstance.id,
                    LUNA_CONFIG_PATH: configPath
                }
            });

            vmProcess.stdout.on('data', (data) => {
                const logEntry = {
                    timestamp: new Date(),
                    level: 'info',
                    source: 'vm_stdout',
                    message: data.toString()
                };
                vmInstance.logs.push(logEntry);
                console.log(`VM ${vmInstance.id} stdout:`, data.toString());
            });

            vmProcess.stderr.on('data', (data) => {
                const logEntry = {
                    timestamp: new Date(),
                    level: 'error',
                    source: 'vm_stderr',
                    message: data.toString()
                };
                vmInstance.logs.push(logEntry);
                console.error(`VM ${vmInstance.id} stderr:`, data.toString());
            });

            vmProcess.on('error', (error) => {
                console.error(`VM process error for ${vmInstance.id}:`, error);
                reject(error);
            });

            vmProcess.on('spawn', () => {
                console.log(`VM process spawned for session ${vmInstance.sessionId}`);
                vmInstance.status = 'running';
                resolve(vmProcess);
            });

            vmProcess.on('exit', (code, signal) => {
                console.log(`VM process exited for session ${vmInstance.sessionId} with code ${code}, signal ${signal}`);
                vmInstance.status = code === 0 ? 'completed' : 'failed';
                vmInstance.exitCode = code;
                vmInstance.exitSignal = signal;
                this.cleanupVMInstance(vmInstance.id, 'exit');
            });
        });
    }

    async startScreenshotCapture(vmInstance) {
        const captureScreenshot = async () => {
            if (vmInstance.status === 'running' || vmInstance.status === 'installing') {
                try {
                    // Send screenshot request to VM via WebSocket
                    await this.sendMessageToVM(vmInstance.sessionId, {
                        type: 'capture_screenshot',
                        timestamp: new Date()
                    });
                } catch (error) {
                    console.error('Error requesting screenshot:', error);
                }
            }
        };

        // Initial screenshot
        setTimeout(captureScreenshot, 5000); // Wait 5 seconds for VM to start

        // Regular screenshots
        vmInstance.screenshotInterval = setInterval(captureScreenshot, this.config.screenshotInterval);
    }

    async sendMessageToVM(sessionId, message) {
        // Find WebSocket connection for this session
        for (const [connectionId, ws] of this.clientConnections) {
            // We'd need to track which connection belongs to which session
            // For now, broadcast to all connections
            if (ws.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify({
                    sessionId: sessionId,
                    ...message
                }));
            }
        }
    }

    async saveScreenshot(sessionId, screenshotData, metadata = {}) {
        try {
            // Decode base64 screenshot data
            const imageBuffer = Buffer.from(screenshotData, 'base64');
            
            // Save via file service
            const response = await fetch('http://localhost:3001/api/files/upload', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    fileData: screenshotData,
                    metadata: {
                        originalName: `session-${sessionId}-${Date.now()}.png`,
                        mimeType: 'image/png',
                        category: 'screenshots',
                        sessionId: sessionId,
                        source: 'luna_installer',
                        ...metadata
                    }
                })
            });

            if (response.ok) {
                const result = await response.json();
                console.log(`Screenshot saved for session ${sessionId}: ${result.id}`);
                return result;
            }

        } catch (error) {
            console.error('Error saving screenshot:', error);
        }
    }

    async stopLunaInstaller(sessionId, reason = 'manual') {
        try {
            const vmInstance = this.vmSessions.get(sessionId);
            if (!vmInstance) {
                throw new Error(`No VM instance found for session ${sessionId}`);
            }

            await this.cleanupVMInstance(vmInstance.id, reason);

            await this.logInstallerEvent('vm_stopped', sessionId, {
                vmInstanceId: vmInstance.id,
                reason: reason,
                duration: Date.now() - vmInstance.startTime.getTime()
            });

            return {
                success: true,
                vmInstanceId: vmInstance.id,
                reason: reason
            };

        } catch (error) {
            console.error('Error stopping Luna installer:', error);
            throw error;
        }
    }

    async cleanupVMInstance(vmInstanceId, reason) {
        const vmInstance = this.activeVMs.get(vmInstanceId);
        if (!vmInstance) {
            return;
        }

        try {
            // Stop screenshot capture
            if (vmInstance.screenshotInterval) {
                clearInterval(vmInstance.screenshotInterval);
            }

            // Terminate VM process
            if (vmInstance.process && !vmInstance.process.killed) {
                vmInstance.process.kill('SIGTERM');
                
                // Force kill after 10 seconds
                setTimeout(() => {
                    if (!vmInstance.process.killed) {
                        vmInstance.process.kill('SIGKILL');
                    }
                }, 10000);
            }

            // Save final logs and data
            await this.saveFinalVMData(vmInstance);

            // Remove from tracking
            this.activeVMs.delete(vmInstanceId);
            this.vmSessions.delete(vmInstance.sessionId);

            console.log(`VM instance ${vmInstanceId} cleaned up (reason: ${reason})`);

        } catch (error) {
            console.error('Error during VM cleanup:', error);
        }
    }

    async saveFinalVMData(vmInstance) {
        try {
            const finalData = {
                vmInstanceId: vmInstance.id,
                sessionId: vmInstance.sessionId,
                participantId: vmInstance.participantId,
                startTime: vmInstance.startTime,
                endTime: new Date(),
                status: vmInstance.status,
                exitCode: vmInstance.exitCode,
                exitSignal: vmInstance.exitSignal,
                error: vmInstance.error,
                installationProgress: vmInstance.installationProgress,
                userInteractions: vmInstance.userInteractions,
                logs: vmInstance.logs,
                screenshots: vmInstance.screenshots.length,
                config: vmInstance.config
            };

            // Save session data as JSON file
            const dataPath = path.join(this.config.vmBasePath, 'sessions', `session-${vmInstance.sessionId}-final.json`);
            await fs.writeFile(dataPath, JSON.stringify(finalData, null, 2));

            // Also save via file service
            await fetch('http://localhost:3001/api/files/upload', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    fileData: Buffer.from(JSON.stringify(finalData, null, 2)).toString('base64'),
                    metadata: {
                        originalName: `session-${vmInstance.sessionId}-vm-data.json`,
                        mimeType: 'application/json',
                        category: 'logs',
                        sessionId: vmInstance.sessionId,
                        source: 'luna_installer'
                    }
                })
            });

        } catch (error) {
            console.error('Error saving final VM data:', error);
        }
    }

    async getVMStatus(sessionId) {
        const vmInstance = this.vmSessions.get(sessionId);
        if (!vmInstance) {
            return { status: 'not_found' };
        }

        return {
            vmInstanceId: vmInstance.id,
            sessionId: vmInstance.sessionId,
            status: vmInstance.status,
            startTime: vmInstance.startTime,
            installationProgress: vmInstance.installationProgress,
            userInteractions: vmInstance.userInteractions.length,
            screenshots: vmInstance.screenshots.length,
            logs: vmInstance.logs.length,
            error: vmInstance.error
        };
    }

    async listActiveVMs() {
        const activeVMs = [];
        for (const [vmId, vmInstance] of this.activeVMs) {
            activeVMs.push({
                vmInstanceId: vmId,
                sessionId: vmInstance.sessionId,
                participantId: vmInstance.participantId,
                status: vmInstance.status,
                startTime: vmInstance.startTime,
                installationProgress: vmInstance.installationProgress,
                duration: Date.now() - vmInstance.startTime.getTime()
            });
        }
        return activeVMs;
    }

    async getInstallerStatistics() {
        try {
            const stats = {
                activeVMs: this.activeVMs.size,
                maxConcurrentVMs: this.config.maxConcurrentVMs,
                websocketConnections: this.clientConnections.size,
                totalSessionsToday: 0,
                avgInstallationTime: 0,
                successRate: 0
            };

            // Get additional stats from database
            const dbStats = await this.db.db.all(`
                SELECT 
                    COUNT(*) as total_sessions,
                    AVG(CASE WHEN JSON_EXTRACT(data, '$.source') = 'luna_installer' 
                        AND message = 'installation_completed' THEN 1 ELSE 0 END) as success_rate,
                    AVG(CASE WHEN JSON_EXTRACT(data, '$.installationDuration') IS NOT NULL 
                        THEN JSON_EXTRACT(data, '$.installationDuration') ELSE NULL END) as avg_duration
                FROM system_logs 
                WHERE component = 'LunaInstallerService'
                AND created_at >= date('now')
            `);

            if (dbStats.length > 0) {
                stats.totalSessionsToday = dbStats[0].total_sessions || 0;
                stats.successRate = Math.round((dbStats[0].success_rate || 0) * 100);
                stats.avgInstallationTime = Math.round(dbStats[0].avg_duration || 0);
            }

            return stats;

        } catch (error) {
            console.error('Error getting installer statistics:', error);
            return {
                activeVMs: this.activeVMs.size,
                maxConcurrentVMs: this.config.maxConcurrentVMs,
                websocketConnections: this.clientConnections.size,
                error: error.message
            };
        }
    }

    async logInstallerEvent(eventType, sessionId, data) {
        try {
            await this.db.db.run(
                `INSERT INTO system_logs (level, component, message, data)
                 VALUES (?, ?, ?, ?)`,
                ['info', 'LunaInstallerService', eventType, JSON.stringify({
                    sessionId,
                    timestamp: new Date(),
                    ...data
                })]
            );
        } catch (error) {
            console.error('Error logging installer event:', error);
        }
    }

    async shutdown() {
        console.log('Shutting down Luna Installer Service...');

        // Stop all active VMs
        const cleanupPromises = Array.from(this.activeVMs.keys()).map(vmId => 
            this.cleanupVMInstance(vmId, 'shutdown')
        );
        await Promise.all(cleanupPromises);

        // Close WebSocket server
        if (this.websocketServer) {
            this.websocketServer.close();
        }

        console.log('Luna Installer Service shutdown complete');
    }
}