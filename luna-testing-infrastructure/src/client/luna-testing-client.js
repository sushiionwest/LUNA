/**
 * Luna Installer Client Integration
 * This script is embedded in the Luna one-click installer to communicate with the testing infrastructure
 */

class LunaTestingClient {
    constructor(config = {}) {
        this.config = {
            websocketUrl: config.websocketUrl || 'ws://localhost:8080',
            sessionId: config.sessionId || null,
            participantId: config.participantId || null,
            testingApiUrl: config.testingApiUrl || 'http://localhost:3001/api',
            reconnectInterval: config.reconnectInterval || 5000,
            maxReconnectAttempts: config.maxReconnectAttempts || 10,
            screenshotInterval: config.screenshotInterval || 30000,
            enableAutoScreenshots: config.enableAutoScreenshots !== false,
            enableEventTracking: config.enableEventTracking !== false,
            enablePerformanceMonitoring: config.enablePerformanceMonitoring !== false,
            ...config
        };

        this.websocket = null;
        this.isConnected = false;
        this.reconnectAttempts = 0;
        this.eventQueue = [];
        this.performanceMetrics = {};
        this.screenshotTimer = null;
        this.startTime = Date.now();

        this.initialize();
    }

    async initialize() {
        console.log('🌙 Luna Testing Client initializing...');
        
        // Connect to testing infrastructure
        await this.connectWebSocket();
        
        // Set up event listeners
        this.setupEventListeners();
        
        // Start performance monitoring
        if (this.config.enablePerformanceMonitoring) {
            this.startPerformanceMonitoring();
        }
        
        // Start automatic screenshots
        if (this.config.enableAutoScreenshots) {
            this.startScreenshotCapture();
        }
        
        // Send initial event
        this.trackEvent('luna_client_initialized', {
            config: this.config,
            userAgent: navigator.userAgent,
            timestamp: new Date()
        });
        
        console.log('🌙 Luna Testing Client ready');
    }

    async connectWebSocket() {
        return new Promise((resolve, reject) => {
            try {
                this.websocket = new WebSocket(this.config.websocketUrl);
                
                this.websocket.onopen = () => {
                    console.log('Luna Testing WebSocket connected');
                    this.isConnected = true;
                    this.reconnectAttempts = 0;
                    
                    // Send queued events
                    this.flushEventQueue();
                    
                    resolve();
                };
                
                this.websocket.onmessage = (event) => {
                    this.handleServerMessage(JSON.parse(event.data));
                };
                
                this.websocket.onclose = () => {
                    console.log('Luna Testing WebSocket disconnected');
                    this.isConnected = false;
                    this.attemptReconnect();
                };
                
                this.websocket.onerror = (error) => {
                    console.error('Luna Testing WebSocket error:', error);
                    reject(error);
                };
                
            } catch (error) {
                console.error('Failed to create WebSocket connection:', error);
                reject(error);
            }
        });
    }

    attemptReconnect() {
        if (this.reconnectAttempts >= this.config.maxReconnectAttempts) {
            console.error('Max reconnection attempts reached');
            return;
        }
        
        this.reconnectAttempts++;
        console.log(`Attempting to reconnect (${this.reconnectAttempts}/${this.config.maxReconnectAttempts})...`);
        
        setTimeout(() => {
            this.connectWebSocket().catch(console.error);
        }, this.config.reconnectInterval);
    }

    handleServerMessage(message) {
        const { type, sessionId, data } = message;
        
        // Only process messages for our session
        if (sessionId && sessionId !== this.config.sessionId) {
            return;
        }
        
        switch (type) {
            case 'capture_screenshot':
                this.captureScreenshot();
                break;
                
            case 'request_performance_data':
                this.sendPerformanceData();
                break;
                
            case 'request_status':
                this.sendStatus();
                break;
                
            case 'force_event':
                this.trackEvent(data.eventType, data.eventData);
                break;
                
            default:
                console.log('Unknown server message type:', type);
        }
    }

    setupEventListeners() {
        if (!this.config.enableEventTracking) {
            return;
        }
        
        // DOM click tracking
        document.addEventListener('click', (event) => {
            this.trackEvent('user_click', {
                element: event.target.tagName,
                elementId: event.target.id,
                elementClass: event.target.className,
                x: event.clientX,
                y: event.clientY,
                timestamp: new Date()
            });
        });
        
        // Form submissions
        document.addEventListener('submit', (event) => {
            this.trackEvent('form_submit', {
                formId: event.target.id,
                formAction: event.target.action,
                timestamp: new Date()
            });
        });
        
        // Page visibility changes
        document.addEventListener('visibilitychange', () => {
            this.trackEvent('visibility_change', {
                hidden: document.hidden,
                timestamp: new Date()
            });
        });
        
        // Error tracking
        window.addEventListener('error', (event) => {
            this.trackEvent('javascript_error', {
                message: event.message,
                filename: event.filename,
                lineno: event.lineno,
                colno: event.colno,
                error: event.error?.toString(),
                timestamp: new Date()
            });
        });
        
        // Unhandled promise rejections
        window.addEventListener('unhandledrejection', (event) => {
            this.trackEvent('unhandled_promise_rejection', {
                reason: event.reason?.toString(),
                timestamp: new Date()
            });
        });
        
        // Before unload (user leaving page)
        window.addEventListener('beforeunload', () => {
            this.trackEvent('page_unload', {
                sessionDuration: Date.now() - this.startTime,
                timestamp: new Date()
            });
        });
    }

    startPerformanceMonitoring() {
        // Monitor performance every 10 seconds
        setInterval(() => {
            this.collectPerformanceMetrics();
        }, 10000);
        
        // Monitor network events
        if ('connection' in navigator) {
            navigator.connection.addEventListener('change', () => {
                this.trackEvent('network_change', {
                    effectiveType: navigator.connection.effectiveType,
                    downlink: navigator.connection.downlink,
                    rtt: navigator.connection.rtt,
                    timestamp: new Date()
                });
            });
        }
    }

    collectPerformanceMetrics() {
        const perf = performance;
        
        this.performanceMetrics = {
            memory: perf.memory ? {
                usedJSHeapSize: perf.memory.usedJSHeapSize,
                totalJSHeapSize: perf.memory.totalJSHeapSize,
                jsHeapSizeLimit: perf.memory.jsHeapSizeLimit
            } : null,
            timing: perf.timing ? {
                domContentLoaded: perf.timing.domContentLoadedEventEnd - perf.timing.domContentLoadedEventStart,
                pageLoad: perf.timing.loadEventEnd - perf.timing.loadEventStart
            } : null,
            navigation: perf.navigation ? {
                type: perf.navigation.type,
                redirectCount: perf.navigation.redirectCount
            } : null,
            timestamp: new Date()
        };
    }

    sendPerformanceData() {
        this.collectPerformanceMetrics();
        this.trackEvent('performance_data', this.performanceMetrics);
    }

    startScreenshotCapture() {
        if (!this.config.enableAutoScreenshots) {
            return;
        }
        
        this.screenshotTimer = setInterval(() => {
            this.captureScreenshot();
        }, this.config.screenshotInterval);
    }

    async captureScreenshot() {
        try {
            // Use HTML5 Canvas to capture current page
            const canvas = document.createElement('canvas');
            const ctx = canvas.getContext('2d');
            
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;
            
            // Create a screenshot using html2canvas (would need to be included)
            if (typeof html2canvas !== 'undefined') {
                const canvasData = await html2canvas(document.body);
                const screenshotData = canvasData.toDataURL('image/png').split(',')[1];
                
                this.trackEvent('screenshot_captured', {
                    screenshot: screenshotData,
                    metadata: {
                        width: canvas.width,
                        height: canvas.height,
                        timestamp: new Date(),
                        url: window.location.href
                    }
                });
            } else {
                // Fallback: just capture basic page info
                this.trackEvent('screenshot_attempted', {
                    error: 'html2canvas not available',
                    pageInfo: {
                        url: window.location.href,
                        title: document.title,
                        timestamp: new Date()
                    }
                });
            }
        } catch (error) {
            console.error('Screenshot capture failed:', error);
            this.trackEvent('screenshot_error', {
                error: error.message,
                timestamp: new Date()
            });
        }
    }

    // Public API methods for Luna installer
    
    trackInstallationProgress(progress, stage) {
        this.trackEvent('installation_progress', {
            progress: progress,
            stage: stage,
            timestamp: new Date()
        });
    }
    
    trackInstallationStarted() {
        this.trackEvent('installation_started', {
            timestamp: new Date()
        });
    }
    
    trackInstallationCompleted() {
        this.trackEvent('installation_completed', {
            duration: Date.now() - this.startTime,
            timestamp: new Date()
        });
    }
    
    trackInstallationFailed(error) {
        this.trackEvent('installation_failed', {
            error: error,
            timestamp: new Date()
        });
    }
    
    trackDownloadStarted() {
        this.trackEvent('download_started', {
            timestamp: new Date()
        });
    }
    
    trackDownloadCompleted(size) {
        this.trackEvent('download_completed', {
            size: size,
            timestamp: new Date()
        });
    }
    
    trackUserInteraction(type, details = {}) {
        this.trackEvent('user_interaction', {
            type: type,
            details: details,
            timestamp: new Date()
        });
    }
    
    trackError(errorType, errorMessage, details = {}) {
        this.trackEvent('error_occurred', {
            errorType: errorType,
            errorMessage: errorMessage,
            details: details,
            timestamp: new Date()
        });
    }
    
    trackTaskStarted(taskName) {
        this.trackEvent('task_started', {
            taskName: taskName,
            timestamp: new Date()
        });
    }
    
    trackTaskCompleted(taskName, duration) {
        this.trackEvent('task_completed', {
            taskName: taskName,
            duration: duration,
            timestamp: new Date()
        });
    }

    // Core event tracking method
    trackEvent(eventType, eventData = {}) {
        const event = {
            sessionId: this.config.sessionId,
            participantId: this.config.participantId,
            eventType: eventType,
            data: {
                ...eventData,
                userAgent: navigator.userAgent,
                url: window.location.href,
                timestamp: new Date()
            },
            timestamp: new Date().toISOString()
        };

        if (this.isConnected && this.websocket) {
            try {
                this.websocket.send(JSON.stringify(event));
            } catch (error) {
                console.error('Failed to send event via WebSocket:', error);
                this.eventQueue.push(event);
            }
        } else {
            // Queue event for later transmission
            this.eventQueue.push(event);
        }

        // Also try HTTP fallback for critical events
        if (this.isCriticalEvent(eventType)) {
            this.sendEventViaHTTP(event);
        }
    }

    isCriticalEvent(eventType) {
        const criticalEvents = [
            'installation_started',
            'installation_completed',
            'installation_failed',
            'error_occurred',
            'javascript_error'
        ];
        return criticalEvents.includes(eventType);
    }

    async sendEventViaHTTP(event) {
        try {
            await fetch(`${this.config.testingApiUrl}/sessions/${this.config.sessionId}/events`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    eventType: event.eventType,
                    data: event.data
                })
            });
        } catch (error) {
            console.error('Failed to send event via HTTP:', error);
        }
    }

    flushEventQueue() {
        while (this.eventQueue.length > 0 && this.isConnected) {
            const event = this.eventQueue.shift();
            try {
                this.websocket.send(JSON.stringify(event));
            } catch (error) {
                console.error('Failed to send queued event:', error);
                this.eventQueue.unshift(event); // Put it back
                break;
            }
        }
    }

    sendStatus() {
        this.trackEvent('client_status', {
            isConnected: this.isConnected,
            reconnectAttempts: this.reconnectAttempts,
            queuedEvents: this.eventQueue.length,
            sessionDuration: Date.now() - this.startTime,
            performanceMetrics: this.performanceMetrics,
            timestamp: new Date()
        });
    }

    disconnect() {
        if (this.screenshotTimer) {
            clearInterval(this.screenshotTimer);
        }
        
        if (this.websocket) {
            this.websocket.close();
        }
        
        console.log('Luna Testing Client disconnected');
    }
}

// Auto-initialize if configuration is provided
if (typeof window !== 'undefined' && window.lunaTestingConfig) {
    window.lunaTestingClient = new LunaTestingClient(window.lunaTestingConfig);
}

// Export for use in modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = LunaTestingClient;
}

// Browser global
if (typeof window !== 'undefined') {
    window.LunaTestingClient = LunaTestingClient;
}