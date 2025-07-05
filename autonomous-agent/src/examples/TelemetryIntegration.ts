/**
 * Example integration of Telemetry and Crash Reporting in Luna Agent
 * This shows how to set up comprehensive monitoring and error handling
 */

import { TelemetryService } from '../services/TelemetryService';
import { CrashReporter } from '../services/CrashReporter';
import { PerformanceMonitor } from '../services/PerformanceMonitor';

// Configuration for telemetry
const telemetryConfig = {
  sentryDsn: process.env.SENTRY_DSN || 'https://your-sentry-dsn@sentry.io/project',
  environment: (process.env.NODE_ENV as any) || 'development',
  applicationName: 'LunaAgent',
  applicationVersion: process.env.npm_package_version || '1.0.0',
  enableSentry: process.env.NODE_ENV === 'production',
  enableLocalLogging: true,
  maxLogSizeMB: 20,
  maxLogFiles: 5,
  enablePerformanceMonitoring: true,
  enableUserTracking: process.env.ENABLE_ANALYTICS === 'true',
  userId: process.env.USER_ID
};

// Initialize services
export const telemetryService = new TelemetryService(telemetryConfig);
export const crashReporter = new CrashReporter(telemetryService);

/**
 * Initialize telemetry for the Luna Agent
 */
export function initializeTelemetry(): void {
  console.log('🔍 Initializing telemetry and crash reporting...');

  // Set user context if available
  const userId = process.env.USER_ID || generateAnonymousUserId();
  telemetryService.setUser(userId, {
    platform: process.platform,
    arch: process.arch,
    nodeVersion: process.version
  });

  // Log application startup
  telemetryService.logEvent('info', 'system', 'application_start', 'Luna Agent started', {
    version: telemetryConfig.applicationVersion,
    environment: telemetryConfig.environment,
    processId: process.pid
  });

  console.log('✅ Telemetry initialized');
}

/**
 * Example: Wrapping Express.js application with telemetry
 */
export function createMonitoredExpressApp() {
  const express = require('express');
  const app = express();

  // Request logging middleware
  app.use((req: any, res: any, next: any) => {
    const startTime = Date.now();
    
    telemetryService.addBreadcrumb(`${req.method} ${req.path}`, 'http', {
      method: req.method,
      path: req.path,
      userAgent: req.get('User-Agent'),
      ip: req.ip
    });

    res.on('finish', () => {
      const duration = Date.now() - startTime;
      const level = res.statusCode >= 400 ? 'error' : 'info';
      
      telemetryService.logEvent(level, 'system', 'http_request', 
        `${req.method} ${req.path} - ${res.statusCode}`, {
          method: req.method,
          path: req.path,
          statusCode: res.statusCode,
          duration,
          userAgent: req.get('User-Agent')
        }
      );

      // Track slow requests
      if (duration > 1000) {
        telemetryService.logEvent('warn', 'performance', 'slow_request', 
          `Slow request: ${req.method} ${req.path}`, {
            duration,
            statusCode: res.statusCode
          }
        );
      }
    });

    next();
  });

  // Error handling middleware
  app.use((error: Error, req: any, res: any, next: any) => {
    telemetryService.captureException(error, 'express_error', {
      method: req.method,
      path: req.path,
      userAgent: req.get('User-Agent'),
      body: req.body
    });

    res.status(500).json({ 
      error: 'Internal server error',
      id: telemetryService.generateEventId?.() || 'unknown'
    });
  });

  return app;
}

/**
 * Example: Monitoring WebSocket connections
 */
export function monitorWebSocketConnection(socket: any): void {
  telemetryService.logEvent('info', 'user', 'websocket_connect', 'Client connected', {
    socketId: socket.id,
    remoteAddress: socket.handshake?.address
  });

  socket.on('disconnect', (reason: string) => {
    telemetryService.logEvent('info', 'user', 'websocket_disconnect', 'Client disconnected', {
      socketId: socket.id,
      reason
    });
  });

  socket.on('error', (error: Error) => {
    telemetryService.logEvent('error', 'system', 'websocket_error', 'WebSocket error', {
      socketId: socket.id,
      error: error.message,
      stack: error.stack
    });
  });
}

/**
 * Example: Monitoring file operations
 */
export async function monitoredFileOperation<T>(
  operation: () => Promise<T>,
  operationType: string,
  filePath?: string
): Promise<T> {
  const startTime = Date.now();
  
  telemetryService.addBreadcrumb(`File operation: ${operationType}`, 'file', {
    operation: operationType,
    path: filePath
  });

  try {
    const result = await operation();
    const duration = Date.now() - startTime;
    
    telemetryService.logEvent('info', 'system', 'file_operation', 
      `File operation completed: ${operationType}`, {
        operation: operationType,
        path: filePath,
        duration,
        success: true
      }
    );

    return result;

  } catch (error) {
    const duration = Date.now() - startTime;
    
    telemetryService.logEvent('error', 'system', 'file_operation_error', 
      `File operation failed: ${operationType}`, {
        operation: operationType,
        path: filePath,
        duration,
        error: error.message,
        success: false
      }
    );

    throw error;
  }
}

/**
 * Example: User action tracking
 */
export function trackUserAction(action: string, details?: Record<string, any>): void {
  telemetryService.recordUserAction(action, {
    timestamp: new Date(),
    ...details
  });
  
  // Add breadcrumb for debugging
  telemetryService.addBreadcrumb(`User: ${action}`, 'user', details);
}

/**
 * Example: Performance tracking decorator
 */
export function withPerformanceTracking(
  target: any,
  propertyName: string,
  descriptor: PropertyDescriptor
): PropertyDescriptor {
  const method = descriptor.value;

  descriptor.value = async function (...args: any[]) {
    const startTime = Date.now();
    const operationName = `${target.constructor.name}.${propertyName}`;
    
    try {
      const result = await method.apply(this, args);
      const duration = Date.now() - startTime;
      
      telemetryService.recordPerformanceMetric({
        operation: operationName,
        duration,
        success: true,
        timestamp: new Date()
      });

      // Warn about slow operations
      if (duration > 5000) {
        telemetryService.logEvent('warn', 'performance', 'slow_operation', 
          `Slow operation: ${operationName}`, {
            duration,
            args: args.map(arg => typeof arg)
          }
        );
      }

      return result;

    } catch (error) {
      const duration = Date.now() - startTime;
      
      telemetryService.recordPerformanceMetric({
        operation: operationName,
        duration,
        success: false,
        error: error.message,
        timestamp: new Date()
      });

      telemetryService.logEvent('error', 'system', 'operation_error', 
        `Operation failed: ${operationName}`, {
          duration,
          error: error.message,
          args: args.map(arg => typeof arg)
        }
      );

      throw error;
    }
  };

  return descriptor;
}

/**
 * Example: AI operation monitoring
 */
export class MonitoredAIService {
  @withPerformanceTracking
  async generateResponse(prompt: string): Promise<string> {
    telemetryService.logEvent('info', 'user', 'ai_request', 'AI generation requested', {
      promptLength: prompt.length,
      model: 'gpt-4'
    });

    try {
      // Simulate AI API call
      const response = await this.callAIAPI(prompt);
      
      telemetryService.logEvent('info', 'system', 'ai_response', 'AI response generated', {
        promptLength: prompt.length,
        responseLength: response.length,
        model: 'gpt-4'
      });

      return response;

    } catch (error) {
      telemetryService.logEvent('error', 'system', 'ai_error', 'AI generation failed', {
        promptLength: prompt.length,
        error: error.message,
        model: 'gpt-4'
      });
      throw error;
    }
  }

  private async callAIAPI(prompt: string): Promise<string> {
    // Placeholder for actual AI API call
    return 'Generated response';
  }
}

/**
 * Example: Screen capture monitoring
 */
export class MonitoredScreenCapture {
  @withPerformanceTracking
  async captureScreen(): Promise<Buffer> {
    telemetryService.addBreadcrumb('Screen capture started', 'system');
    
    try {
      const screenshot = require('screenshot-desktop');
      const image = await screenshot();
      
      telemetryService.logEvent('info', 'system', 'screen_capture', 'Screen captured successfully', {
        imageSize: image.length,
        format: 'png'
      });

      return image;

    } catch (error) {
      telemetryService.logEvent('error', 'system', 'screen_capture_error', 'Screen capture failed', {
        error: error.message
      });
      throw error;
    }
  }
}

/**
 * Example: Database operation monitoring
 */
export class MonitoredDatabase {
  @withPerformanceTracking
  async query(sql: string, params?: any[]): Promise<any[]> {
    telemetryService.addBreadcrumb(`Database query: ${sql.substring(0, 50)}...`, 'database', {
      paramCount: params?.length || 0
    });

    try {
      // Simulate database query
      const results = await this.executeQuery(sql, params);
      
      telemetryService.logEvent('info', 'system', 'database_query', 'Database query executed', {
        queryLength: sql.length,
        resultCount: results.length,
        paramCount: params?.length || 0
      });

      return results;

    } catch (error) {
      telemetryService.logEvent('error', 'system', 'database_error', 'Database query failed', {
        query: sql.substring(0, 100),
        error: error.message,
        paramCount: params?.length || 0
      });
      throw error;
    }
  }

  private async executeQuery(sql: string, params?: any[]): Promise<any[]> {
    // Placeholder for actual database query
    return [];
  }
}

/**
 * Health check endpoint with telemetry
 */
export function createHealthCheckEndpoint(app: any): void {
  app.get('/health', async (req: any, res: any) => {
    try {
      const diagnostics = telemetryService.getSystemDiagnostics();
      const recentCrashes = crashReporter.getRecentCrashes(1);
      
      const health = {
        status: 'healthy',
        timestamp: new Date(),
        uptime: process.uptime(),
        memory: process.memoryUsage(),
        system: diagnostics.system,
        recentCrashes: recentCrashes.length,
        telemetry: {
          recentEvents: diagnostics.logs.recentEvents,
          logDirectory: diagnostics.logs.logDirectory
        }
      };

      // Check for concerning metrics
      if (health.memory.heapUsed / health.memory.heapTotal > 0.9) {
        health.status = 'warning';
        telemetryService.logEvent('warn', 'system', 'health_check_warning', 
          'High memory usage detected in health check', health
        );
      }

      if (recentCrashes.length > 0) {
        health.status = 'degraded';
        telemetryService.logEvent('warn', 'system', 'health_check_crashes', 
          'Recent crashes detected in health check', { crashes: recentCrashes }
        );
      }

      res.json(health);

    } catch (error) {
      telemetryService.logEvent('error', 'system', 'health_check_error', 
        'Health check failed', { error: error.message }
      );
      
      res.status(500).json({
        status: 'error',
        message: 'Health check failed'
      });
    }
  });
}

/**
 * Graceful shutdown with telemetry
 */
export function setupGracefulShutdown(): void {
  const shutdown = (signal: string) => {
    telemetryService.logEvent('info', 'system', 'shutdown_initiated', 
      `Shutdown initiated by ${signal}`
    );

    // Give time for telemetry to be sent
    setTimeout(() => {
      telemetryService.shutdown();
      process.exit(0);
    }, 2000);
  };

  process.on('SIGTERM', () => shutdown('SIGTERM'));
  process.on('SIGINT', () => shutdown('SIGINT'));
}

/**
 * Utility functions
 */
function generateAnonymousUserId(): string {
  return `anon-${Date.now()}-${Math.random().toString(36).substr(2, 8)}`;
}

/**
 * Example main application setup
 */
export async function setupLunaWithTelemetry(): Promise<void> {
  try {
    // Initialize telemetry
    initializeTelemetry();

    // Create monitored Express app
    const app = createMonitoredExpressApp();
    
    // Add health check endpoint
    createHealthCheckEndpoint(app);

    // Setup graceful shutdown
    setupGracefulShutdown();

    // Start server
    const port = process.env.PORT || 3000;
    app.listen(port, () => {
      telemetryService.logEvent('info', 'system', 'server_started', 
        `Luna Agent server started on port ${port}`, { port }
      );
      console.log(`🚀 Luna Agent running on port ${port}`);
    });

    // Example usage of monitored services
    const aiService = new MonitoredAIService();
    const screenCapture = new MonitoredScreenCapture();
    const database = new MonitoredDatabase();

    // Test operations
    await screenCapture.captureScreen();
    await database.query('SELECT * FROM users');
    await aiService.generateResponse('Hello, world!');

    console.log('✅ Luna Agent with telemetry is ready');

  } catch (error) {
    crashReporter.reportCrash(error, 'critical');
    console.error('❌ Failed to start Luna Agent:', error);
    process.exit(1);
  }
}

// Export for use in main application
export { telemetryService, crashReporter };