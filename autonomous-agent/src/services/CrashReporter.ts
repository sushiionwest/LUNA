import { TelemetryService } from './TelemetryService';
import { writeFileSync, readFileSync, existsSync, mkdirSync } from 'fs';
import { join } from 'path';
import { execSync } from 'child_process';

export interface CrashContext {
  timestamp: Date;
  processId: number;
  parentProcessId?: number;
  workingDirectory: string;
  commandLineArgs: string[];
  environmentVariables: Record<string, string>;
  openFileDescriptors?: number;
  memoryMaps?: string[];
  stackTrace?: string[];
  modules?: Array<{
    name: string;
    version?: string;
    path: string;
  }>;
}

export interface SystemSnapshot {
  timestamp: Date;
  cpu: {
    model: string;
    cores: number;
    usage: number;
  };
  memory: {
    total: number;
    available: number;
    used: number;
    percentage: number;
  };
  disk: Array<{
    filesystem: string;
    size: number;
    used: number;
    available: number;
    percentage: number;
    mountpoint: string;
  }>;
  network: Array<{
    interface: string;
    ip: string;
    status: string;
  }>;
  processes: Array<{
    pid: number;
    name: string;
    cpu: number;
    memory: number;
  }>;
}

export interface CrashDump {
  id: string;
  timestamp: Date;
  severity: 'low' | 'medium' | 'high' | 'critical';
  error: {
    message: string;
    stack: string;
    name: string;
    code?: string;
  };
  context: CrashContext;
  systemSnapshot: SystemSnapshot;
  applicationState: {
    version: string;
    uptime: number;
    activeConnections: number;
    lastUserAction?: string;
    recentLogs: any[];
    configuration: Record<string, any>;
  };
  recovery: {
    canRestart: boolean;
    restartAttempts: number;
    safeMode: boolean;
    dataIntegrity: 'intact' | 'corrupted' | 'unknown';
  };
}

/**
 * Advanced crash reporter for Luna Agent
 * Provides detailed crash analysis and recovery recommendations
 */
export class CrashReporter {
  private telemetryService: TelemetryService;
  private crashDirectory: string;
  private isEnabled: boolean;
  private maxCrashDumps: number;
  private restartAttempts: number = 0;
  private lastCrashTime?: Date;

  constructor(telemetryService: TelemetryService, crashDirectory?: string) {
    this.telemetryService = telemetryService;
    this.crashDirectory = crashDirectory || this.getDefaultCrashDirectory();
    this.isEnabled = true;
    this.maxCrashDumps = 50;
    
    this.ensureCrashDirectory();
    this.setupCrashHandlers();
  }

  /**
   * Report a crash with full context
   */
  async reportCrash(error: Error, severity: CrashDump['severity'] = 'high'): Promise<string> {
    try {
      const crashId = this.generateCrashId();
      
      const crashDump: CrashDump = {
        id: crashId,
        timestamp: new Date(),
        severity,
        error: {
          message: error.message,
          stack: error.stack || '',
          name: error.name,
          code: (error as any).code
        },
        context: await this.collectCrashContext(),
        systemSnapshot: await this.captureSystemSnapshot(),
        applicationState: await this.collectApplicationState(),
        recovery: this.assessRecoveryOptions(error)
      };

      // Save crash dump
      await this.saveCrashDump(crashDump);

      // Log to telemetry
      this.telemetryService.logEvent('fatal', 'error', 'crash_reported', 
        `Crash reported: ${error.message}`, {
          crashId,
          severity,
          canRestart: crashDump.recovery.canRestart
        }
      );

      // Send to Sentry through telemetry service
      this.telemetryService.captureException(error, 'crash_report', {
        crashId,
        severity,
        recovery: crashDump.recovery
      });

      // Cleanup old crash dumps
      this.cleanupOldCrashDumps();

      // Update restart attempts
      this.updateRestartAttempts();

      return crashId;

    } catch (reportError) {
      console.error('Failed to report crash:', reportError);
      // Fallback: at least log the original error
      console.error('Original crash:', error);
      return 'crash-report-failed';
    }
  }

  /**
   * Check if the application should attempt to restart
   */
  shouldAttemptRestart(error: Error): boolean {
    // Don't restart if too many recent attempts
    if (this.restartAttempts >= 3) {
      return false;
    }

    // Don't restart if last crash was very recent (< 60 seconds)
    if (this.lastCrashTime && (Date.now() - this.lastCrashTime.getTime()) < 60000) {
      return false;
    }

    // Don't restart for certain types of errors
    const nonRestartableErrors = [
      'EADDRINUSE',
      'EACCES',
      'ENOENT',
      'MODULE_NOT_FOUND'
    ];

    if (nonRestartableErrors.includes((error as any).code)) {
      return false;
    }

    return true;
  }

  /**
   * Generate crash report summary for user
   */
  generateCrashReport(crashId: string): string {
    try {
      const crashDumpPath = join(this.crashDirectory, `${crashId}.json`);
      
      if (!existsSync(crashDumpPath)) {
        return `Crash report ${crashId} not found.`;
      }

      const crashDump: CrashDump = JSON.parse(readFileSync(crashDumpPath, 'utf8'));
      
      return this.formatCrashReport(crashDump);

    } catch (error) {
      return `Failed to generate crash report for ${crashId}: ${error.message}`;
    }
  }

  /**
   * Get recent crash history
   */
  getRecentCrashes(days = 7): Array<{ id: string; timestamp: Date; severity: string; message: string }> {
    try {
      const fs = require('fs');
      const cutoffDate = new Date(Date.now() - days * 24 * 60 * 60 * 1000);
      
      return fs.readdirSync(this.crashDirectory)
        .filter((file: string) => file.endsWith('.json'))
        .map((file: string) => {
          try {
            const crashDump = JSON.parse(fs.readFileSync(join(this.crashDirectory, file), 'utf8'));
            return {
              id: crashDump.id,
              timestamp: new Date(crashDump.timestamp),
              severity: crashDump.severity,
              message: crashDump.error.message
            };
          } catch {
            return null;
          }
        })
        .filter((crash: any) => crash && crash.timestamp > cutoffDate)
        .sort((a: any, b: any) => b.timestamp.getTime() - a.timestamp.getTime());

    } catch {
      return [];
    }
  }

  /**
   * Setup crash handlers
   */
  private setupCrashHandlers(): void {
    // Handle uncaught exceptions
    process.on('uncaughtException', async (error) => {
      const crashId = await this.reportCrash(error, 'critical');
      console.error(`\n💥 FATAL ERROR [${crashId}]: ${error.message}`);
      console.error('Stack trace:', error.stack);
      
      if (this.shouldAttemptRestart(error)) {
        console.log('🔄 Attempting to restart...');
        // Give time for crash report to be saved
        setTimeout(() => {
          process.exit(1);
        }, 2000);
      } else {
        console.log('❌ Too many restart attempts or non-recoverable error. Exiting.');
        process.exit(1);
      }
    });

    // Handle unhandled promise rejections
    process.on('unhandledRejection', async (reason, promise) => {
      const error = reason instanceof Error ? reason : new Error(String(reason));
      const crashId = await this.reportCrash(error, 'high');
      console.error(`\n⚠️  UNHANDLED REJECTION [${crashId}]: ${error.message}`);
      console.error('Promise:', promise);
    });

    // Handle warnings
    process.on('warning', (warning) => {
      this.telemetryService.logEvent('warn', 'system', 'node_warning', warning.message, {
        name: warning.name,
        stack: warning.stack
      });
    });

    // Handle memory warnings
    process.on('beforeExit', () => {
      this.telemetryService.logEvent('info', 'system', 'before_exit', 'Process is about to exit');
    });
  }

  /**
   * Collect crash context information
   */
  private async collectCrashContext(): Promise<CrashContext> {
    const context: CrashContext = {
      timestamp: new Date(),
      processId: process.pid,
      parentProcessId: process.ppid,
      workingDirectory: process.cwd(),
      commandLineArgs: process.argv,
      environmentVariables: this.filterEnvironmentVariables(process.env)
    };

    // Try to get additional system info on Windows
    if (process.platform === 'win32') {
      try {
        context.openFileDescriptors = this.getOpenFileDescriptors();
        context.stackTrace = this.getStackTrace();
        context.modules = this.getLoadedModules();
      } catch {
        // Silently fail if we can't get extended info
      }
    }

    return context;
  }

  /**
   * Capture system snapshot
   */
  private async captureSystemSnapshot(): Promise<SystemSnapshot> {
    const snapshot: SystemSnapshot = {
      timestamp: new Date(),
      cpu: await this.getCpuInfo(),
      memory: await this.getMemoryInfo(),
      disk: await this.getDiskInfo(),
      network: await this.getNetworkInfo(),
      processes: await this.getTopProcesses()
    };

    return snapshot;
  }

  /**
   * Collect application state
   */
  private async collectApplicationState(): Promise<CrashDump['applicationState']> {
    const recentLogs = this.telemetryService.getRecentLogs(20);
    
    return {
      version: process.env.npm_package_version || 'unknown',
      uptime: process.uptime(),
      activeConnections: this.getActiveConnections(),
      lastUserAction: this.getLastUserAction(),
      recentLogs,
      configuration: this.getSafeConfiguration()
    };
  }

  /**
   * Assess recovery options
   */
  private assessRecoveryOptions(error: Error): CrashDump['recovery'] {
    const canRestart = this.shouldAttemptRestart(error);
    const safeMode = this.restartAttempts > 0;
    
    return {
      canRestart,
      restartAttempts: this.restartAttempts,
      safeMode,
      dataIntegrity: this.assessDataIntegrity(error)
    };
  }

  /**
   * Save crash dump to file
   */
  private async saveCrashDump(crashDump: CrashDump): Promise<void> {
    const crashFilePath = join(this.crashDirectory, `${crashDump.id}.json`);
    writeFileSync(crashFilePath, JSON.stringify(crashDump, null, 2));

    // Also save a human-readable summary
    const summaryPath = join(this.crashDirectory, `${crashDump.id}.txt`);
    writeFileSync(summaryPath, this.formatCrashReport(crashDump));
  }

  /**
   * Format crash report for human reading
   */
  private formatCrashReport(crashDump: CrashDump): string {
    return `
LUNA CRASH REPORT
================

Crash ID: ${crashDump.id}
Time: ${crashDump.timestamp.toISOString()}
Severity: ${crashDump.severity.toUpperCase()}

ERROR DETAILS
-------------
Message: ${crashDump.error.message}
Type: ${crashDump.error.name}
Code: ${crashDump.error.code || 'N/A'}

Stack Trace:
${crashDump.error.stack}

SYSTEM INFORMATION
------------------
Platform: ${process.platform} ${process.arch}
Node.js: ${process.version}
Process ID: ${crashDump.context.processId}
Working Directory: ${crashDump.context.workingDirectory}
Uptime: ${Math.floor(crashDump.applicationState.uptime / 60)} minutes

MEMORY USAGE
------------
Total System: ${Math.round(crashDump.systemSnapshot.memory.total / 1024 / 1024)} MB
Available: ${Math.round(crashDump.systemSnapshot.memory.available / 1024 / 1024)} MB
Used: ${crashDump.systemSnapshot.memory.percentage.toFixed(1)}%

RECOVERY OPTIONS
----------------
Can Restart: ${crashDump.recovery.canRestart ? 'Yes' : 'No'}
Restart Attempts: ${crashDump.recovery.restartAttempts}
Safe Mode: ${crashDump.recovery.safeMode ? 'Yes' : 'No'}
Data Integrity: ${crashDump.recovery.dataIntegrity}

RECENT ACTIVITY
---------------
${crashDump.applicationState.recentLogs.slice(-5).map(log => 
  `${log.timestamp}: [${log.level.toUpperCase()}] ${log.message}`
).join('\n')}

For technical support, please provide this crash ID: ${crashDump.id}
`;
  }

  /**
   * Get CPU information
   */
  private async getCpuInfo(): Promise<SystemSnapshot['cpu']> {
    const cpus = require('os').cpus();
    return {
      model: cpus[0]?.model || 'Unknown',
      cores: cpus.length,
      usage: await this.getCpuUsage()
    };
  }

  /**
   * Get CPU usage percentage
   */
  private async getCpuUsage(): Promise<number> {
    return new Promise((resolve) => {
      const startUsage = process.cpuUsage();
      setTimeout(() => {
        const endUsage = process.cpuUsage(startUsage);
        const totalUsage = endUsage.user + endUsage.system;
        const percentage = (totalUsage / 1000000) * 100; // Convert to percentage
        resolve(Math.min(percentage, 100));
      }, 100);
    });
  }

  /**
   * Get memory information
   */
  private async getMemoryInfo(): Promise<SystemSnapshot['memory']> {
    const totalMem = require('os').totalmem();
    const freeMem = require('os').freemem();
    const usedMem = totalMem - freeMem;
    
    return {
      total: totalMem,
      available: freeMem,
      used: usedMem,
      percentage: (usedMem / totalMem) * 100
    };
  }

  /**
   * Get disk information (Windows-specific)
   */
  private async getDiskInfo(): Promise<SystemSnapshot['disk']> {
    if (process.platform !== 'win32') {
      return [];
    }

    try {
      const output = execSync('wmic logicaldisk get size,freespace,caption', { encoding: 'utf8' });
      const lines = output.split('\n').filter(line => line.trim() && !line.includes('Caption'));
      
      return lines.map(line => {
        const parts = line.trim().split(/\s+/);
        if (parts.length >= 3) {
          const size = parseInt(parts[1]) || 0;
          const free = parseInt(parts[0]) || 0;
          const used = size - free;
          
          return {
            filesystem: parts[2],
            size,
            used,
            available: free,
            percentage: size > 0 ? (used / size) * 100 : 0,
            mountpoint: parts[2]
          };
        }
        return null;
      }).filter(Boolean) as SystemSnapshot['disk'];
    } catch {
      return [];
    }
  }

  /**
   * Get network information
   */
  private async getNetworkInfo(): Promise<SystemSnapshot['network']> {
    const interfaces = require('os').networkInterfaces();
    const result: SystemSnapshot['network'] = [];
    
    for (const [name, addrs] of Object.entries(interfaces)) {
      if (addrs) {
        for (const addr of addrs as any[]) {
          if (addr.family === 'IPv4' && !addr.internal) {
            result.push({
              interface: name,
              ip: addr.address,
              status: 'active'
            });
          }
        }
      }
    }
    
    return result;
  }

  /**
   * Get top processes (Windows-specific)
   */
  private async getTopProcesses(): Promise<SystemSnapshot['processes']> {
    if (process.platform !== 'win32') {
      return [];
    }

    try {
      const output = execSync('tasklist /fo csv | findstr /v "Image Name"', { encoding: 'utf8' });
      const lines = output.split('\n').filter(line => line.trim());
      
      return lines.slice(0, 10).map(line => {
        const parts = line.split(',').map(part => part.replace(/"/g, ''));
        return {
          pid: parseInt(parts[1]) || 0,
          name: parts[0] || 'Unknown',
          cpu: 0, // Would need additional command to get CPU usage
          memory: this.parseMemoryString(parts[4]) || 0
        };
      });
    } catch {
      return [];
    }
  }

  /**
   * Helper methods
   */
  private getOpenFileDescriptors(): number {
    // Placeholder - would need platform-specific implementation
    return 0;
  }

  private getStackTrace(): string[] {
    return (new Error()).stack?.split('\n') || [];
  }

  private getLoadedModules(): CrashContext['modules'] {
    return Object.keys(require.cache).map(path => ({
      name: require('path').basename(path),
      path
    }));
  }

  private filterEnvironmentVariables(env: NodeJS.ProcessEnv): Record<string, string> {
    const filtered: Record<string, string> = {};
    const sensitiveKeys = ['password', 'secret', 'key', 'token', 'auth'];
    
    for (const [key, value] of Object.entries(env)) {
      if (value && !sensitiveKeys.some(sensitive => key.toLowerCase().includes(sensitive))) {
        filtered[key] = value;
      }
    }
    
    return filtered;
  }

  private getActiveConnections(): number {
    // Placeholder - would need to track active WebSocket/HTTP connections
    return 0;
  }

  private getLastUserAction(): string | undefined {
    // Placeholder - would need to track user actions
    return undefined;
  }

  private getSafeConfiguration(): Record<string, any> {
    // Return non-sensitive configuration
    return {
      nodeEnv: process.env.NODE_ENV,
      platform: process.platform,
      arch: process.arch
    };
  }

  private assessDataIntegrity(error: Error): 'intact' | 'corrupted' | 'unknown' {
    // Simple heuristic - could be more sophisticated
    if (error.message.includes('corrupt') || error.message.includes('invalid')) {
      return 'corrupted';
    }
    return 'unknown';
  }

  private parseMemoryString(memStr: string): number {
    if (!memStr) return 0;
    const num = parseFloat(memStr.replace(/[^\d.]/g, ''));
    if (memStr.includes('K')) return num * 1024;
    if (memStr.includes('M')) return num * 1024 * 1024;
    if (memStr.includes('G')) return num * 1024 * 1024 * 1024;
    return num;
  }

  private updateRestartAttempts(): void {
    this.lastCrashTime = new Date();
    this.restartAttempts++;
    
    // Reset restart attempts after 24 hours
    setTimeout(() => {
      this.restartAttempts = 0;
    }, 24 * 60 * 60 * 1000);
  }

  private cleanupOldCrashDumps(): void {
    try {
      const fs = require('fs');
      const files = fs.readdirSync(this.crashDirectory)
        .filter((file: string) => file.endsWith('.json'))
        .map((file: string) => ({
          name: file,
          path: join(this.crashDirectory, file),
          stats: fs.statSync(join(this.crashDirectory, file))
        }))
        .sort((a: any, b: any) => b.stats.mtime.getTime() - a.stats.mtime.getTime());

      const filesToDelete = files.slice(this.maxCrashDumps);
      
      for (const file of filesToDelete) {
        fs.unlinkSync(file.path);
        // Also delete corresponding text file
        const txtFile = file.path.replace('.json', '.txt');
        if (fs.existsSync(txtFile)) {
          fs.unlinkSync(txtFile);
        }
      }
    } catch (error) {
      console.error('Failed to cleanup old crash dumps:', error);
    }
  }

  private getDefaultCrashDirectory(): string {
    const appDataDir = process.env.LOCALAPPDATA || 
                      process.env.APPDATA || 
                      join(require('os').homedir(), '.local', 'share');
    
    return join(appDataDir, 'Luna', 'crashes');
  }

  private ensureCrashDirectory(): void {
    if (!existsSync(this.crashDirectory)) {
      mkdirSync(this.crashDirectory, { recursive: true });
    }
  }

  private generateCrashId(): string {
    return `crash-${Date.now()}-${Math.random().toString(36).substr(2, 8)}`;
  }
}