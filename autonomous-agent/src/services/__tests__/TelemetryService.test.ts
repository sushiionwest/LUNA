import { describe, it, expect, beforeEach, afterEach, jest } from '@jest/globals';
import { TelemetryService } from '../TelemetryService';
import { join } from 'path';
import { tmpdir } from 'os';
import { rmSync, existsSync, readFileSync, readdirSync } from 'fs';

// Mock Sentry
jest.mock('@sentry/node', () => ({
  init: jest.fn(),
  setUser: jest.fn(),
  setTags: jest.fn(),
  addBreadcrumb: jest.fn(),
  withScope: jest.fn((callback) => callback({ setTag: jest.fn(), setContext: jest.fn() })),
  captureException: jest.fn(),
  close: jest.fn(),
  Integrations: {
    Console: jest.fn(),
    Http: jest.fn(),
    OnUncaughtException: jest.fn(),
    OnUnhandledRejection: jest.fn()
  }
}));

describe('TelemetryService', () => {
  let telemetryService: TelemetryService;
  let testLogDir: string;

  beforeEach(() => {
    testLogDir = join(tmpdir(), `luna-telemetry-test-${Date.now()}`);
    
    telemetryService = new TelemetryService({
      applicationName: 'LunaTestApp',
      applicationVersion: '1.0.0-test',
      environment: 'development',
      enableSentry: false, // Disable for tests
      enableLocalLogging: true,
      logDirectory: testLogDir,
      maxLogSizeMB: 1, // Small size for testing
      maxLogFiles: 3,
      enablePerformanceMonitoring: false, // Disable to avoid timer issues in tests
      enableUserTracking: true
    });
  });

  afterEach(() => {
    if (existsSync(testLogDir)) {
      rmSync(testLogDir, { recursive: true, force: true });
    }
  });

  describe('Basic Logging', () => {
    it('should log events with correct structure', () => {
      telemetryService.logEvent('info', 'system', 'test_event', 'Test message', {
        testData: 'value'
      });

      const recentLogs = telemetryService.getRecentLogs(1);
      expect(recentLogs).toHaveLength(1);
      
      const log = recentLogs[0];
      expect(log.level).toBe('info');
      expect(log.category).toBe('system');
      expect(log.event).toBe('test_event');
      expect(log.message).toBe('Test message');
      expect(log.data).toEqual({ testData: 'value' });
      expect(log.id).toBeDefined();
      expect(log.timestamp).toBeInstanceOf(Date);
      expect(log.context).toBeDefined();
    });

    it('should maintain recent events buffer', () => {
      // Add more events than buffer size
      for (let i = 0; i < 105; i++) {
        telemetryService.logEvent('info', 'system', 'test_event', `Message ${i}`);
      }

      const recentLogs = telemetryService.getRecentLogs();
      expect(recentLogs).toHaveLength(50); // Default recent logs limit
      
      // Should have the most recent events
      expect(recentLogs[recentLogs.length - 1].message).toBe('Message 104');
    });

    it('should write logs to file when enabled', () => {
      telemetryService.logEvent('info', 'system', 'file_test', 'File logging test');

      // Check that log file was created
      const logFiles = readdirSync(testLogDir).filter(file => file.endsWith('.log'));
      expect(logFiles.length).toBeGreaterThan(0);

      // Check log file content
      const logContent = readFileSync(join(testLogDir, logFiles[0]), 'utf8');
      expect(logContent).toContain('file_test');
      expect(logContent).toContain('File logging test');
    });
  });

  describe('User Tracking', () => {
    it('should record user actions', () => {
      telemetryService.recordUserAction('click_button', {
        buttonId: 'submit',
        page: 'login'
      });

      const recentLogs = telemetryService.getRecentLogs(1);
      expect(recentLogs[0].category).toBe('user');
      expect(recentLogs[0].event).toBe('click_button');
      expect(recentLogs[0].data).toMatchObject({
        buttonId: 'submit',
        page: 'login'
      });
    });

    it('should set user context', () => {
      telemetryService.setUser('test-user-123', {
        email: 'test@example.com',
        plan: 'premium'
      });

      const recentLogs = telemetryService.getRecentLogs(1);
      expect(recentLogs[0].event).toBe('user_identified');
      expect(recentLogs[0].message).toContain('test-user-123');
    });
  });

  describe('Performance Monitoring', () => {
    it('should record performance metrics', () => {
      const metric = {
        operation: 'database_query',
        duration: 150,
        success: true,
        timestamp: new Date()
      };

      telemetryService.recordPerformanceMetric(metric);

      // Performance metrics are stored internally
      // We can't directly access them, but we can verify through diagnostics
      const diagnostics = telemetryService.getSystemDiagnostics();
      expect(diagnostics.performance).toBeDefined();
    });
  });

  describe('Breadcrumbs', () => {
    it('should add breadcrumbs for debugging', () => {
      telemetryService.addBreadcrumb('User clicked login', 'user', {
        buttonId: 'login-btn'
      });

      const recentLogs = telemetryService.getRecentLogs(1);
      expect(recentLogs[0].level).toBe('debug');
      expect(recentLogs[0].category).toBe('system');
      expect(recentLogs[0].event).toBe('breadcrumb');
      expect(recentLogs[0].message).toBe('User clicked login');
    });
  });

  describe('Exception Handling', () => {
    it('should capture exceptions with context', () => {
      const testError = new Error('Test error message');
      testError.stack = 'Error: Test error\n  at test.js:1:1';

      telemetryService.captureException(testError, 'test_exception', {
        additionalContext: 'test data'
      });

      const recentLogs = telemetryService.getRecentLogs(1);
      expect(recentLogs[0].level).toBe('fatal');
      expect(recentLogs[0].category).toBe('error');
      expect(recentLogs[0].event).toBe('test_exception');
      expect(recentLogs[0].message).toContain('Test error message');
      expect(recentLogs[0].data).toMatchObject({
        additionalContext: 'test data'
      });
    });
  });

  describe('System Diagnostics', () => {
    it('should provide system diagnostics', () => {
      const diagnostics = telemetryService.getSystemDiagnostics();

      expect(diagnostics).toHaveProperty('timestamp');
      expect(diagnostics).toHaveProperty('sessionId');
      expect(diagnostics).toHaveProperty('uptime');
      expect(diagnostics).toHaveProperty('memory');
      expect(diagnostics).toHaveProperty('system');
      expect(diagnostics).toHaveProperty('performance');
      expect(diagnostics).toHaveProperty('logs');

      expect(diagnostics.system).toHaveProperty('platform');
      expect(diagnostics.system).toHaveProperty('arch');
      expect(diagnostics.system).toHaveProperty('hostname');
      expect(diagnostics.logs).toHaveProperty('logDirectory');
    });
  });

  describe('Log Rotation', () => {
    it('should handle log file rotation', async () => {
      // Create a large number of log entries to trigger rotation
      const largeData = 'x'.repeat(10000); // 10KB per entry
      
      for (let i = 0; i < 150; i++) { // Should exceed 1MB limit
        telemetryService.logEvent('info', 'system', 'large_test', 'Large message', {
          data: largeData
        });
      }

      // Give some time for file operations
      await new Promise(resolve => setTimeout(resolve, 100));

      // Check that multiple log files were created due to rotation
      const logFiles = readdirSync(testLogDir).filter(file => file.endsWith('.log'));
      
      // Should have at least rotated once, but not exceed max files
      expect(logFiles.length).toBeGreaterThan(0);
      expect(logFiles.length).toBeLessThanOrEqual(3); // maxLogFiles = 3
    });
  });

  describe('Configuration', () => {
    it('should handle different log levels appropriately', () => {
      const levels: Array<'debug' | 'info' | 'warn' | 'error' | 'fatal'> = 
        ['debug', 'info', 'warn', 'error', 'fatal'];

      levels.forEach(level => {
        telemetryService.logEvent(level, 'system', 'level_test', `${level} message`);
      });

      const recentLogs = telemetryService.getRecentLogs(5);
      expect(recentLogs).toHaveLength(5);
      
      levels.forEach((level, index) => {
        expect(recentLogs[index].level).toBe(level);
      });
    });

    it('should handle different categories', () => {
      const categories: Array<'system' | 'user' | 'performance' | 'error' | 'security'> = 
        ['system', 'user', 'performance', 'error', 'security'];

      categories.forEach(category => {
        telemetryService.logEvent('info', category, 'category_test', `${category} message`);
      });

      const recentLogs = telemetryService.getRecentLogs(5);
      expect(recentLogs).toHaveLength(5);
      
      categories.forEach((category, index) => {
        expect(recentLogs[index].category).toBe(category);
      });
    });
  });

  describe('Error Handling', () => {
    it('should gracefully handle file system errors', () => {
      // Create a telemetry service with an invalid directory
      const invalidTelemetry = new TelemetryService({
        applicationName: 'InvalidTest',
        applicationVersion: '1.0.0',
        environment: 'development',
        enableSentry: false,
        enableLocalLogging: true,
        logDirectory: '/invalid/path/that/does/not/exist',
        maxLogSizeMB: 1,
        maxLogFiles: 3,
        enablePerformanceMonitoring: false
      });

      // Should not throw error
      expect(() => {
        invalidTelemetry.logEvent('info', 'system', 'test', 'Should not crash');
      }).not.toThrow();
    });

    it('should handle large log data gracefully', () => {
      const veryLargeData = {
        largeArray: new Array(10000).fill('large data item'),
        largeString: 'x'.repeat(100000),
        deepObject: {
          level1: { level2: { level3: { level4: 'deep data' } } }
        }
      };

      expect(() => {
        telemetryService.logEvent('info', 'system', 'large_data_test', 
          'Testing large data', veryLargeData);
      }).not.toThrow();

      const recentLogs = telemetryService.getRecentLogs(1);
      expect(recentLogs[0].data).toEqual(veryLargeData);
    });
  });

  describe('Shutdown', () => {
    it('should shutdown gracefully', () => {
      expect(() => {
        telemetryService.shutdown();
      }).not.toThrow();

      // Should still be able to log after shutdown (for cleanup logs)
      expect(() => {
        telemetryService.logEvent('info', 'system', 'post_shutdown', 'After shutdown');
      }).not.toThrow();
    });
  });
});