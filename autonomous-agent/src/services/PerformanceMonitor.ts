import { EventEmitter } from 'events';
import * as os from 'os';
import * as process from 'process';
import { performance, PerformanceObserver } from 'perf_hooks';

export interface PerformanceMetrics {
  timestamp: Date;
  cpu: {
    usage: number; // Percentage
    loadAverage: number[];
    cores: number;
  };
  memory: {
    used: number; // Bytes
    free: number; // Bytes
    total: number; // Bytes
    usage: number; // Percentage
    heapUsed: number; // Node.js heap
    heapTotal: number; // Node.js heap
  };
  process: {
    pid: number;
    uptime: number; // Seconds
    cpuUsage: NodeJS.CpuUsage;
    memoryUsage: NodeJS.MemoryUsage;
  };
  operations: {
    screenshotCount: number;
    screenshotRate: number; // Per minute
    clickCount: number;
    clickRate: number; // Per minute
    totalOperations: number;
  };
  performance: {
    avgResponseTime: number; // Milliseconds
    maxResponseTime: number; // Milliseconds
    minResponseTime: number; // Milliseconds
    errorRate: number; // Percentage
  };
}

export interface PerformanceAlert {
  level: 'warning' | 'critical';
  type: 'cpu' | 'memory' | 'operations' | 'performance';
  message: string;
  value: number;
  threshold: number;
  timestamp: Date;
}

export interface PerformanceThresholds {
  cpu: {
    warning: number;
    critical: number;
  };
  memory: {
    warning: number;
    critical: number;
  };
  operations: {
    maxScreenshotsPerMinute: number;
    maxClicksPerMinute: number;
  };
  performance: {
    maxResponseTime: number;
    maxErrorRate: number;
  };
}

export class PerformanceMonitor extends EventEmitter {
  private isRunning = false;
  private monitoringInterval: NodeJS.Timeout | null = null;
  private metricsHistory: PerformanceMetrics[] = [];
  private operationCounts = {
    screenshots: 0,
    clicks: 0,
    total: 0
  };
  private operationTimes: number[] = [];
  private responseTimes: number[] = [];
  private errors = 0;
  private totalRequests = 0;
  private lastCpuUsage: NodeJS.CpuUsage | null = null;
  private startTime = Date.now();

  private readonly thresholds: PerformanceThresholds = {
    cpu: {
      warning: 70, // 70% CPU usage
      critical: 90  // 90% CPU usage
    },
    memory: {
      warning: 80, // 80% memory usage
      critical: 95  // 95% memory usage
    },
    operations: {
      maxScreenshotsPerMinute: 10,
      maxClicksPerMinute: 60
    },
    performance: {
      maxResponseTime: 5000, // 5 seconds
      maxErrorRate: 5 // 5% error rate
    }
  };

  private readonly maxHistorySize = 1000; // Keep last 1000 metrics
  private readonly monitoringIntervalMs = 5000; // 5 seconds

  constructor(customThresholds?: Partial<PerformanceThresholds>) {
    super();
    
    if (customThresholds) {
      this.thresholds = { ...this.thresholds, ...customThresholds };
    }

    this.setupPerformanceObserver();
  }

  private setupPerformanceObserver(): void {
    const perfObserver = new PerformanceObserver((list) => {
      const entries = list.getEntries();
      entries.forEach((entry) => {
        if (entry.entryType === 'measure') {
          this.responseTimes.push(entry.duration);
          
          // Keep only recent response times (last 100)
          if (this.responseTimes.length > 100) {
            this.responseTimes = this.responseTimes.slice(-100);
          }
        }
      });
    });

    perfObserver.observe({ entryTypes: ['measure'] });
  }

  public start(): void {
    if (this.isRunning) {
      return;
    }

    this.isRunning = true;
    this.startTime = Date.now();
    this.lastCpuUsage = process.cpuUsage();

    console.log('🔍 Performance monitor started');
    this.emit('started');

    this.monitoringInterval = setInterval(() => {
      this.collectMetrics();
    }, this.monitoringIntervalMs);
  }

  public stop(): void {
    if (!this.isRunning) {
      return;
    }

    this.isRunning = false;

    if (this.monitoringInterval) {
      clearInterval(this.monitoringInterval);
      this.monitoringInterval = null;
    }

    console.log('⏹️ Performance monitor stopped');
    this.emit('stopped');
  }

  private async collectMetrics(): Promise<void> {
    try {
      const currentTime = new Date();
      const uptimeSeconds = (Date.now() - this.startTime) / 1000;

      // CPU metrics
      const currentCpuUsage = process.cpuUsage(this.lastCpuUsage || undefined);
      const cpuPercent = this.calculateCpuPercentage(currentCpuUsage);
      this.lastCpuUsage = process.cpuUsage();

      // Memory metrics
      const memUsage = process.memoryUsage();
      const systemMem = {
        total: os.totalmem(),
        free: os.freemem()
      };
      const memoryUsage = ((systemMem.total - systemMem.free) / systemMem.total) * 100;

      // Operation metrics
      const screenshotRate = this.calculateOperationRate('screenshots', uptimeSeconds);
      const clickRate = this.calculateOperationRate('clicks', uptimeSeconds);

      // Performance metrics
      const perfMetrics = this.calculatePerformanceMetrics();

      const metrics: PerformanceMetrics = {
        timestamp: currentTime,
        cpu: {
          usage: cpuPercent,
          loadAverage: os.loadavg(),
          cores: os.cpus().length
        },
        memory: {
          used: systemMem.total - systemMem.free,
          free: systemMem.free,
          total: systemMem.total,
          usage: memoryUsage,
          heapUsed: memUsage.heapUsed,
          heapTotal: memUsage.heapTotal
        },
        process: {
          pid: process.pid,
          uptime: uptimeSeconds,
          cpuUsage: currentCpuUsage,
          memoryUsage: memUsage
        },
        operations: {
          screenshotCount: this.operationCounts.screenshots,
          screenshotRate,
          clickCount: this.operationCounts.clicks,
          clickRate,
          totalOperations: this.operationCounts.total
        },
        performance: perfMetrics
      };

      // Store metrics
      this.metricsHistory.push(metrics);
      if (this.metricsHistory.length > this.maxHistorySize) {
        this.metricsHistory = this.metricsHistory.slice(-this.maxHistorySize);
      }

      // Check for alerts
      this.checkThresholds(metrics);

      // Emit metrics event
      this.emit('metrics', metrics);

    } catch (error) {
      console.error('Error collecting performance metrics:', error);
      this.emit('error', error);
    }
  }

  private calculateCpuPercentage(cpuUsage: NodeJS.CpuUsage): number {
    const totalCpuTime = cpuUsage.user + cpuUsage.system;
    const totalTime = this.monitoringIntervalMs * 1000; // Convert to microseconds
    
    return Math.min(100, (totalCpuTime / totalTime) * 100);
  }

  private calculateOperationRate(operation: keyof typeof this.operationCounts, uptimeSeconds: number): number {
    const count = this.operationCounts[operation];
    return uptimeSeconds > 0 ? (count / uptimeSeconds) * 60 : 0; // Operations per minute
  }

  private calculatePerformanceMetrics(): PerformanceMetrics['performance'] {
    if (this.responseTimes.length === 0) {
      return {
        avgResponseTime: 0,
        maxResponseTime: 0,
        minResponseTime: 0,
        errorRate: 0
      };
    }

    const avg = this.responseTimes.reduce((sum, time) => sum + time, 0) / this.responseTimes.length;
    const max = Math.max(...this.responseTimes);
    const min = Math.min(...this.responseTimes);
    const errorRate = this.totalRequests > 0 ? (this.errors / this.totalRequests) * 100 : 0;

    return {
      avgResponseTime: avg,
      maxResponseTime: max,
      minResponseTime: min,
      errorRate
    };
  }

  private checkThresholds(metrics: PerformanceMetrics): void {
    const alerts: PerformanceAlert[] = [];

    // CPU alerts
    if (metrics.cpu.usage >= this.thresholds.cpu.critical) {
      alerts.push({
        level: 'critical',
        type: 'cpu',
        message: `Critical CPU usage: ${metrics.cpu.usage.toFixed(1)}%`,
        value: metrics.cpu.usage,
        threshold: this.thresholds.cpu.critical,
        timestamp: metrics.timestamp
      });
    } else if (metrics.cpu.usage >= this.thresholds.cpu.warning) {
      alerts.push({
        level: 'warning',
        type: 'cpu',
        message: `High CPU usage: ${metrics.cpu.usage.toFixed(1)}%`,
        value: metrics.cpu.usage,
        threshold: this.thresholds.cpu.warning,
        timestamp: metrics.timestamp
      });
    }

    // Memory alerts
    if (metrics.memory.usage >= this.thresholds.memory.critical) {
      alerts.push({
        level: 'critical',
        type: 'memory',
        message: `Critical memory usage: ${metrics.memory.usage.toFixed(1)}%`,
        value: metrics.memory.usage,
        threshold: this.thresholds.memory.critical,
        timestamp: metrics.timestamp
      });
    } else if (metrics.memory.usage >= this.thresholds.memory.warning) {
      alerts.push({
        level: 'warning',
        type: 'memory',
        message: `High memory usage: ${metrics.memory.usage.toFixed(1)}%`,
        value: metrics.memory.usage,
        threshold: this.thresholds.memory.warning,
        timestamp: metrics.timestamp
      });
    }

    // Operation rate alerts
    if (metrics.operations.screenshotRate > this.thresholds.operations.maxScreenshotsPerMinute) {
      alerts.push({
        level: 'warning',
        type: 'operations',
        message: `High screenshot rate: ${metrics.operations.screenshotRate.toFixed(1)}/min`,
        value: metrics.operations.screenshotRate,
        threshold: this.thresholds.operations.maxScreenshotsPerMinute,
        timestamp: metrics.timestamp
      });
    }

    if (metrics.operations.clickRate > this.thresholds.operations.maxClicksPerMinute) {
      alerts.push({
        level: 'warning',
        type: 'operations',
        message: `High click rate: ${metrics.operations.clickRate.toFixed(1)}/min`,
        value: metrics.operations.clickRate,
        threshold: this.thresholds.operations.maxClicksPerMinute,
        timestamp: metrics.timestamp
      });
    }

    // Performance alerts
    if (metrics.performance.avgResponseTime > this.thresholds.performance.maxResponseTime) {
      alerts.push({
        level: 'warning',
        type: 'performance',
        message: `Slow response time: ${metrics.performance.avgResponseTime.toFixed(0)}ms`,
        value: metrics.performance.avgResponseTime,
        threshold: this.thresholds.performance.maxResponseTime,
        timestamp: metrics.timestamp
      });
    }

    if (metrics.performance.errorRate > this.thresholds.performance.maxErrorRate) {
      alerts.push({
        level: 'critical',
        type: 'performance',
        message: `High error rate: ${metrics.performance.errorRate.toFixed(1)}%`,
        value: metrics.performance.errorRate,
        threshold: this.thresholds.performance.maxErrorRate,
        timestamp: metrics.timestamp
      });
    }

    // Emit alerts
    alerts.forEach(alert => {
      console.warn(`🚨 Performance Alert [${alert.level.toUpperCase()}]: ${alert.message}`);
      this.emit('alert', alert);
    });
  }

  // Public methods for recording operations
  public recordScreenshot(): void {
    this.operationCounts.screenshots++;
    this.operationCounts.total++;
    this.operationTimes.push(Date.now());
  }

  public recordClick(): void {
    this.operationCounts.clicks++;
    this.operationCounts.total++;
    this.operationTimes.push(Date.now());
  }

  public recordOperation(type: string): void {
    this.operationCounts.total++;
    this.operationTimes.push(Date.now());
  }

  public recordRequest(success: boolean, responseTime?: number): void {
    this.totalRequests++;
    if (!success) {
      this.errors++;
    }
    if (responseTime !== undefined) {
      this.responseTimes.push(responseTime);
    }
  }

  public startMeasure(name: string): void {
    performance.mark(`${name}-start`);
  }

  public endMeasure(name: string): number {
    performance.mark(`${name}-end`);
    performance.measure(name, `${name}-start`, `${name}-end`);
    
    const entries = performance.getEntriesByName(name, 'measure');
    return entries.length > 0 ? entries[entries.length - 1].duration : 0;
  }

  // Getters for current metrics
  public getCurrentMetrics(): PerformanceMetrics | null {
    return this.metricsHistory.length > 0 ? this.metricsHistory[this.metricsHistory.length - 1] : null;
  }

  public getMetricsHistory(count?: number): PerformanceMetrics[] {
    if (count) {
      return this.metricsHistory.slice(-count);
    }
    return [...this.metricsHistory];
  }

  public getAverageMetrics(timeWindowMinutes: number = 5): Partial<PerformanceMetrics> | null {
    const cutoffTime = new Date(Date.now() - timeWindowMinutes * 60 * 1000);
    const recentMetrics = this.metricsHistory.filter(m => m.timestamp >= cutoffTime);

    if (recentMetrics.length === 0) {
      return null;
    }

    const avgCpu = recentMetrics.reduce((sum, m) => sum + m.cpu.usage, 0) / recentMetrics.length;
    const avgMemory = recentMetrics.reduce((sum, m) => sum + m.memory.usage, 0) / recentMetrics.length;
    const avgResponseTime = recentMetrics.reduce((sum, m) => sum + m.performance.avgResponseTime, 0) / recentMetrics.length;

    return {
      cpu: { usage: avgCpu } as any,
      memory: { usage: avgMemory } as any,
      performance: { avgResponseTime } as any
    };
  }

  public getThresholds(): PerformanceThresholds {
    return { ...this.thresholds };
  }

  public updateThresholds(newThresholds: Partial<PerformanceThresholds>): void {
    Object.assign(this.thresholds, newThresholds);
    this.emit('thresholds-updated', this.thresholds);
  }

  public isMonitoring(): boolean {
    return this.isRunning;
  }

  public getUptime(): number {
    return (Date.now() - this.startTime) / 1000;
  }

  public reset(): void {
    this.metricsHistory = [];
    this.operationCounts = { screenshots: 0, clicks: 0, total: 0 };
    this.operationTimes = [];
    this.responseTimes = [];
    this.errors = 0;
    this.totalRequests = 0;
    this.startTime = Date.now();
    this.emit('reset');
  }
}