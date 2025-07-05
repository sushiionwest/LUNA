import * as Sentry from '@sentry/node';
import { writeFileSync, readFileSync, existsSync, mkdirSync, readdirSync, statSync, unlinkSync } from 'fs';
import { join } from 'path';
import { hostname, userInfo, platform, arch, release, totalmem, freemem, cpus } from 'os';
import { promisify } from 'util';

export interface TelemetryConfig {
  sentryDsn?: string;
  environment: 'development' | 'production' | 'staging';
  applicationName: string;
  applicationVersion: string;
  enableSentry: boolean;
  enableLocalLogging: boolean;
  logDirectory?: string;
  maxLogSizeMB: number;
  maxLogFiles: number;
  enablePerformanceMonitoring: boolean;
  enableUserTracking: boolean;
  userId?: string;
}

export interface TelemetryEvent {
  id: string;
  timestamp: Date;
  level: 'debug' | 'info' | 'warn' | 'error' | 'fatal';
  category: 'system' | 'user' | 'performance' | 'error' | 'security';
  event: string;
  message: string;
  data?: Record<string, any>;
  stack?: string;
  context?: {
    user?: string;
    session?: string;
    version?: string;
    os?: string;
    memory?: number;
    cpu?: number;
  };
}

export interface CrashReport {
  id: string;
  timestamp: Date;
  error: Error;
  stack: string;
  context: {
    version: string;
    platform: string;
    arch: string;
    nodeVersion: string;
    memoryUsage: NodeJS.MemoryUsage;
    systemInfo: {
      hostname: string;
      totalMemory: number;
      freeMemory: number;
      cpus: number;
    };
    recentEvents: TelemetryEvent[];
    recentPerformance: any[];
  };
  breadcrumbs: any[];
  userAgent?: string;
  userId?: string;
}

/**
 * Telemetry and crash reporting service for Luna Agent
 * Integrates with Sentry for remote crash reporting and maintains local structured logs
 */
export class TelemetryService {
  private config: TelemetryConfig;
  private logDirectory: string;
  private currentLogFile: string;
  private recentEvents: TelemetryEvent[] = [];
  private performanceMetrics: any[] = [];
  private sessionId: string;
  private isInitialized = false;

  constructor(config: TelemetryConfig) {
    this.config = {
      maxLogSizeMB: 20,
      maxLogFiles: 5,
      enablePerformanceMonitoring: true,
      enableUserTracking: false,
      ...config
    };

    this.sessionId = this.generateSessionId();
    this.logDirectory = config.logDirectory || this.getDefaultLogDirectory();
    this.currentLogFile = join(this.logDirectory, `luna-${new Date().toISOString().split('T')[0]}.log`);
    
    this.initialize();
  }

  /**
   * Initialize the telemetry service
   */
  private initialize(): void {
    try {
      // Ensure log directory exists
      this.ensureLogDirectory();

      // Initialize Sentry if enabled
      if (this.config.enableSentry && this.config.sentryDsn) {
        this.initializeSentry();
      }

      // Set up global error handlers
      this.setupErrorHandlers();

      // Set up performance monitoring
      if (this.config.enablePerformanceMonitoring) {
        this.setupPerformanceMonitoring();
      }

      // Log initialization
      this.logEvent('info', 'system', 'telemetry_initialized', 'Telemetry service initialized', {
        sessionId: this.sessionId,
        config: {
          enableSentry: this.config.enableSentry,
          enableLocalLogging: this.config.enableLocalLogging,
          environment: this.config.environment
        }
      });

      this.isInitialized = true;

    } catch (error) {
      console.error('Failed to initialize telemetry service:', error);
    }
  }

  /**
   * Initialize Sentry for crash reporting
   */
  private initializeSentry(): void {
    Sentry.init({
      dsn: this.config.sentryDsn,
      environment: this.config.environment,
      release: `${this.config.applicationName}@${this.config.applicationVersion}`,
      integrations: [
        new Sentry.Integrations.Console(),
        new Sentry.Integrations.Http(),
        new Sentry.Integrations.OnUncaughtException(),
        new Sentry.Integrations.OnUnhandledRejection(),
      ],
      tracesSampleRate: this.config.environment === 'production' ? 0.1 : 1.0,
      beforeSend: (event) => {
        // Filter sensitive data
        return this.filterSentryEvent(event);
      }
    });

    // Set user context if available
    if (this.config.userId) {
      Sentry.setUser({ id: this.config.userId });
    }

    // Set tags
    Sentry.setTags({
      platform: platform(),
      arch: arch(),
      nodeVersion: process.version,
      sessionId: this.sessionId
    });
  }

  /**
   * Set up global error handlers
   */
  private setupErrorHandlers(): void {
    // Uncaught exceptions
    process.on('uncaughtException', (error) => {
      this.captureException(error, 'uncaught_exception');
      
      // Give time for logging/reporting before exit
      setTimeout(() => {
        process.exit(1);
      }, 1000);
    });

    // Unhandled promise rejections
    process.on('unhandledRejection', (reason, promise) => {
      const error = reason instanceof Error ? reason : new Error(String(reason));
      this.captureException(error, 'unhandled_rejection', { promise });
    });

    // SIGINT and SIGTERM handlers
    process.on('SIGINT', () => {
      this.logEvent('info', 'system', 'shutdown', 'Application shutdown requested (SIGINT)');
      this.shutdown();
    });

    process.on('SIGTERM', () => {
      this.logEvent('info', 'system', 'shutdown', 'Application shutdown requested (SIGTERM)');
      this.shutdown();
    });
  }

  /**
   * Set up performance monitoring
   */
  private setupPerformanceMonitoring(): void {
    // Monitor memory usage every 30 seconds
    setInterval(() => {
      const memUsage = process.memoryUsage();
      const systemMem = {
        total: totalmem(),
        free: freemem()
      };

      const performanceData = {
        timestamp: new Date(),
        type: 'memory',
        process: memUsage,
        system: systemMem,
        usage: {
          heapPercent: (memUsage.heapUsed / memUsage.heapTotal) * 100,
          systemPercent: ((systemMem.total - systemMem.free) / systemMem.total) * 100
        }
      };

      this.recordPerformanceMetric(performanceData);

      // Alert on high memory usage
      if (performanceData.usage.heapPercent > 90) {
        this.logEvent('warn', 'performance', 'high_memory_usage', 
          `High heap usage: ${performanceData.usage.heapPercent.toFixed(1)}%`, 
          performanceData
        );
      }

    }, 30000);

    // Monitor event loop lag
    let start = process.hrtime.bigint();
    setInterval(() => {
      const delta = process.hrtime.bigint() - start;
      const nanosec = Number(delta);
      const millisec = nanosec / 1e6;
      
      if (millisec > 100) {
        this.logEvent('warn', 'performance', 'event_loop_lag', 
          `Event loop lag: ${millisec.toFixed(2)}ms`, 
          { lagMs: millisec }
        );
      }
      
      start = process.hrtime.bigint();
    }, 1000);
  }

  /**
   * Log a telemetry event
   */
  logEvent(
    level: TelemetryEvent['level'], 
    category: TelemetryEvent['category'], 
    event: string, 
    message: string, 
    data?: Record<string, any>
  ): void {
    const telemetryEvent: TelemetryEvent = {
      id: this.generateEventId(),
      timestamp: new Date(),
      level,
      category,
      event,
      message,
      data,
      context: {
        session: this.sessionId,
        version: this.config.applicationVersion,
        os: `${platform()} ${release()}`,
        memory: process.memoryUsage().heapUsed,
        cpu: process.cpuUsage().user
      }
    };

    // Add to recent events buffer
    this.recentEvents.push(telemetryEvent);
    if (this.recentEvents.length > 100) {
      this.recentEvents = this.recentEvents.slice(-100);
    }

    // Log to console in development
    if (this.config.environment === 'development') {
      console.log(`[${level.toUpperCase()}] ${category}:${event} - ${message}`, data || '');
    }

    // Write to local log file
    if (this.config.enableLocalLogging) {
      this.writeToLogFile(telemetryEvent);
    }

    // Send to Sentry if appropriate level
    if (this.config.enableSentry && (level === 'error' || level === 'fatal')) {
      Sentry.addBreadcrumb({
        message,
        category,
        level: level as any,
        data
      });
    }
  }

  /**
   * Capture an exception with full context
   */
  captureException(error: Error, type: string, additionalData?: Record<string, any>): void {
    const crashReport: CrashReport = {
      id: this.generateEventId(),
      timestamp: new Date(),
      error,
      stack: error.stack || '',
      context: {
        version: this.config.applicationVersion,
        platform: platform(),
        arch: arch(),
        nodeVersion: process.version,
        memoryUsage: process.memoryUsage(),
        systemInfo: {
          hostname: hostname(),
          totalMemory: totalmem(),
          freeMemory: freemem(),
          cpus: cpus().length
        },
        recentEvents: this.recentEvents.slice(-20), // Last 20 events
        recentPerformance: this.performanceMetrics.slice(-10) // Last 10 performance metrics
      },
      breadcrumbs: [],
      userId: this.config.userId
    };

    // Log the crash
    this.logEvent('fatal', 'error', type, `Unhandled exception: ${error.message}`, {
      ...additionalData,
      stack: error.stack,
      crashReportId: crashReport.id
    });

    // Save crash report to file
    this.saveCrashReport(crashReport);

    // Send to Sentry
    if (this.config.enableSentry) {
      Sentry.withScope((scope) => {
        scope.setTag('crashType', type);
        scope.setContext('crashReport', crashReport.context);
        if (additionalData) {
          scope.setContext('additionalData', additionalData);
        }
        Sentry.captureException(error);
      });
    }
  }

  /**
   * Record user action for analytics
   */
  recordUserAction(action: string, details?: Record<string, any>): void {
    if (!this.config.enableUserTracking) return;

    this.logEvent('info', 'user', action, `User action: ${action}`, details);
  }

  /**
   * Record performance metric
   */
  recordPerformanceMetric(metric: any): void {
    this.performanceMetrics.push(metric);
    if (this.performanceMetrics.length > 1000) {
      this.performanceMetrics = this.performanceMetrics.slice(-1000);
    }
  }

  /**
   * Set user context
   */
  setUser(userId: string, additionalData?: Record<string, any>): void {
    this.config.userId = userId;
    
    if (this.config.enableSentry) {
      Sentry.setUser({ id: userId, ...additionalData });
    }

    this.logEvent('info', 'user', 'user_identified', `User identified: ${userId}`, additionalData);
  }

  /**
   * Add breadcrumb for debugging
   */
  addBreadcrumb(message: string, category: string, data?: Record<string, any>): void {
    if (this.config.enableSentry) {
      Sentry.addBreadcrumb({
        message,
        category,
        data,
        timestamp: Date.now() / 1000
      });
    }

    this.logEvent('debug', 'system', 'breadcrumb', message, { category, ...data });
  }

  /**
   * Get recent logs for debugging
   */
  getRecentLogs(count = 50): TelemetryEvent[] {
    return this.recentEvents.slice(-count);
  }

  /**
   * Get system diagnostics
   */
  getSystemDiagnostics(): Record<string, any> {
    return {
      timestamp: new Date(),
      sessionId: this.sessionId,
      uptime: process.uptime(),
      memory: process.memoryUsage(),
      system: {
        platform: platform(),
        arch: arch(),
        release: release(),
        hostname: hostname(),
        totalMemory: totalmem(),
        freeMemory: freemem(),
        cpus: cpus().length
      },
      performance: {
        recentMetrics: this.performanceMetrics.slice(-5)
      },
      logs: {
        recentEvents: this.recentEvents.length,
        logDirectory: this.logDirectory,
        currentLogFile: this.currentLogFile
      }
    };
  }

  /**
   * Shutdown telemetry service gracefully
   */
  shutdown(): void {
    this.logEvent('info', 'system', 'telemetry_shutdown', 'Telemetry service shutting down');
    
    if (this.config.enableSentry) {
      Sentry.close(2000);
    }
  }

  /**
   * Write telemetry event to log file
   */
  private writeToLogFile(event: TelemetryEvent): void {
    try {
      const logLine = JSON.stringify(event) + '\n';
      
      // Check if log rotation is needed
      if (this.shouldRotateLog()) {
        this.rotateLogFile();
      }

      // Append to current log file
      writeFileSync(this.currentLogFile, logLine, { flag: 'a' });

    } catch (error) {
      console.error('Failed to write to log file:', error);
    }
  }

  /**
   * Check if log file should be rotated
   */
  private shouldRotateLog(): boolean {
    try {
      if (!existsSync(this.currentLogFile)) {
        return false;
      }

      const stats = statSync(this.currentLogFile);
      const sizeMB = stats.size / (1024 * 1024);
      
      return sizeMB >= this.config.maxLogSizeMB;
    } catch {
      return false;
    }
  }

  /**
   * Rotate log file
   */
  private rotateLogFile(): void {
    try {
      // Clean up old log files
      this.cleanupOldLogs();

      // Create new log file name
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      this.currentLogFile = join(this.logDirectory, `luna-${timestamp}.log`);

    } catch (error) {
      console.error('Failed to rotate log file:', error);
    }
  }

  /**
   * Clean up old log files
   */
  private cleanupOldLogs(): void {
    try {
      const files = readdirSync(this.logDirectory)
        .filter(file => file.startsWith('luna-') && file.endsWith('.log'))
        .map(file => ({
          name: file,
          path: join(this.logDirectory, file),
          stats: statSync(join(this.logDirectory, file))
        }))
        .sort((a, b) => b.stats.mtime.getTime() - a.stats.mtime.getTime());

      // Keep only the most recent files
      const filesToDelete = files.slice(this.config.maxLogFiles);
      
      for (const file of filesToDelete) {
        unlinkSync(file.path);
      }

    } catch (error) {
      console.error('Failed to cleanup old logs:', error);
    }
  }

  /**
   * Save crash report to file
   */
  private saveCrashReport(report: CrashReport): void {
    try {
      const crashDir = join(this.logDirectory, 'crashes');
      if (!existsSync(crashDir)) {
        mkdirSync(crashDir, { recursive: true });
      }

      const crashFile = join(crashDir, `crash-${report.id}.json`);
      writeFileSync(crashFile, JSON.stringify(report, null, 2));

    } catch (error) {
      console.error('Failed to save crash report:', error);
    }
  }

  /**
   * Filter sensitive data from Sentry events
   */
  private filterSentryEvent(event: any): any {
    // Remove sensitive data from event
    if (event.extra) {
      delete event.extra.password;
      delete event.extra.token;
      delete event.extra.apiKey;
      delete event.extra.secret;
    }

    // Filter breadcrumbs
    if (event.breadcrumbs) {
      event.breadcrumbs = event.breadcrumbs.filter((crumb: any) => {
        return !crumb.message?.includes('password') && 
               !crumb.message?.includes('token') &&
               !crumb.message?.includes('secret');
      });
    }

    return event;
  }

  /**
   * Get default log directory
   */
  private getDefaultLogDirectory(): string {
    const appDataDir = process.env.LOCALAPPDATA || 
                      process.env.APPDATA || 
                      join(require('os').homedir(), '.local', 'share');
    
    return join(appDataDir, this.config.applicationName, 'logs');
  }

  /**
   * Ensure log directory exists
   */
  private ensureLogDirectory(): void {
    if (!existsSync(this.logDirectory)) {
      mkdirSync(this.logDirectory, { recursive: true });
    }
  }

  /**
   * Generate session ID
   */
  private generateSessionId(): string {
    return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Generate event ID
   */
  private generateEventId(): string {
    return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }
}