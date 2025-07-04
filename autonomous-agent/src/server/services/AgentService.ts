import { EventEmitter } from 'events';
import { v4 as uuidv4 } from 'uuid';
import cron from 'node-cron';

import { DatabaseService, Task } from './DatabaseService.js';
import { ScreenCaptureService, CaptureOptions } from './ScreenCaptureService.js';
import { SocialMediaService, SocialMediaPost, ContentGenerationOptions } from './SocialMediaService.js';

export interface AgentTask {
  id: string;
  name: string;
  type: 'screen_capture' | 'social_media' | 'automation' | 'installation' | 'analysis' | 'custom';
  priority: number;
  parameters: any;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  retryCount: number;
  maxRetries: number;
  result?: any;
  error?: string;
  startTime?: Date;
  endTime?: Date;
  duration?: number;
}

export interface AgentStatus {
  isRunning: boolean;
  currentTasks: number;
  maxConcurrentTasks: number;
  totalTasksProcessed: number;
  successfulTasks: number;
  failedTasks: number;
  queuedTasks: number;
  uptime: number;
  lastActivity: Date | null;
}

export interface AutomationRule {
  id: string;
  name: string;
  description: string;
  trigger: {
    type: 'schedule' | 'screen_change' | 'file_change' | 'api_call' | 'manual';
    schedule?: string; // Cron expression
    parameters?: any;
  };
  conditions: Array<{
    type: 'screen_content' | 'file_exists' | 'time_range' | 'system_metric';
    operator: 'equals' | 'contains' | 'greater_than' | 'less_than' | 'exists';
    value: any;
  }>;
  actions: Array<{
    type: 'screen_capture' | 'social_media_post' | 'file_operation' | 'notification' | 'api_call';
    parameters: any;
  }>;
  isActive: boolean;
  lastExecuted?: Date;
  executionCount: number;
}

export class AgentService extends EventEmitter {
  private databaseService: DatabaseService;
  private screenCaptureService: ScreenCaptureService;
  private socialMediaService: SocialMediaService;
  
  private isRunning: boolean = false;
  private taskQueue: AgentTask[] = [];
  private currentTasks: Map<string, AgentTask> = new Map();
  private maxConcurrentTasks: number = 3;
  private automationRules: Map<string, AutomationRule> = new Map();
  private scheduledJobs: Map<string, any> = new Map();
  
  // Statistics
  private totalTasksProcessed: number = 0;
  private successfulTasks: number = 0;
  private failedTasks: number = 0;
  private startTime: Date = new Date();
  private lastActivity: Date | null = null;

  constructor(
    databaseService: DatabaseService,
    screenCaptureService: ScreenCaptureService,
    socialMediaService: SocialMediaService
  ) {
    super();
    this.databaseService = databaseService;
    this.screenCaptureService = screenCaptureService;
    this.socialMediaService = socialMediaService;
    
    this.initializeAgent();
  }

  private async initializeAgent(): Promise<void> {
    try {
      // Load existing tasks from database
      await this.loadPendingTasks();
      
      // Load automation rules
      await this.loadAutomationRules();
      
      console.log('✅ Agent service initialized');
      this.emit('agent:initialized');
    } catch (error) {
      console.error('❌ Agent initialization failed:', error);
      this.emit('agent:error', error);
    }
  }

  /**
   * Start the agent
   */
  public async start(): Promise<void> {
    if (this.isRunning) {
      console.warn('⚠️ Agent is already running');
      return;
    }

    this.isRunning = true;
    this.startTime = new Date();
    
    // Start processing queue
    this.processTaskQueue();
    
    // Start automation rules
    this.activateAutomationRules();
    
    console.log('🚀 Agent started');
    this.emit('agent:started');
    
    // Log activity
    await this.databaseService.logActivity({
      type: 'info',
      category: 'agent',
      message: 'Agent service started',
      source: 'AgentService'
    });
  }

  /**
   * Stop the agent
   */
  public async stop(): Promise<void> {
    if (!this.isRunning) {
      console.warn('⚠️ Agent is not running');
      return;
    }

    this.isRunning = false;
    
    // Cancel all running tasks
    for (const [taskId, task] of this.currentTasks) {
      task.status = 'cancelled';
      await this.updateTaskInDatabase(task);
    }
    this.currentTasks.clear();
    
    // Stop all scheduled jobs
    for (const [ruleId, job] of this.scheduledJobs) {
      job.stop();
    }
    this.scheduledJobs.clear();
    
    console.log('🛑 Agent stopped');
    this.emit('agent:stopped');
    
    // Log activity
    await this.databaseService.logActivity({
      type: 'info',
      category: 'agent',
      message: 'Agent service stopped',
      source: 'AgentService'
    });
  }

  /**
   * Add a new task to the queue
   */
  public async addTask(task: Omit<AgentTask, 'id' | 'status' | 'retryCount' | 'startTime' | 'endTime' | 'duration'>): Promise<string> {
    const agentTask: AgentTask = {
      ...task,
      id: uuidv4(),
      status: 'pending',
      retryCount: 0,
      maxRetries: task.maxRetries || 3
    };

    // Add to queue
    this.taskQueue.push(agentTask);
    this.taskQueue.sort((a, b) => b.priority - a.priority); // Higher priority first

    // Save to database
    await this.saveTaskToDatabase(agentTask);

    console.log(`📋 Task added: ${agentTask.name} (ID: ${agentTask.id})`);
    this.emit('task:added', agentTask);

    // Start processing if agent is running
    if (this.isRunning) {
      this.processTaskQueue();
    }

    return agentTask.id;
  }

  /**
   * Process the task queue
   */
  private async processTaskQueue(): Promise<void> {
    if (!this.isRunning) return;

    // Process tasks while we have capacity and queued tasks
    while (this.currentTasks.size < this.maxConcurrentTasks && this.taskQueue.length > 0) {
      const task = this.taskQueue.shift()!;
      
      if (task.status === 'pending') {
        this.executeTask(task);
      }
    }

    // Schedule next processing cycle
    if (this.isRunning) {
      setTimeout(() => this.processTaskQueue(), 1000);
    }
  }

  /**
   * Execute a specific task
   */
  private async executeTask(task: AgentTask): Promise<void> {
    task.status = 'running';
    task.startTime = new Date();
    this.currentTasks.set(task.id, task);
    this.lastActivity = new Date();

    console.log(`🔄 Executing task: ${task.name} (${task.type})`);
    this.emit('task:started', task);

    // Update database
    await this.updateTaskInDatabase(task);

    try {
      let result: any;

      switch (task.type) {
        case 'screen_capture':
          result = await this.executeScreenCaptureTask(task);
          break;
        case 'social_media':
          result = await this.executeSocialMediaTask(task);
          break;
        case 'automation':
          result = await this.executeAutomationTask(task);
          break;
        case 'analysis':
          result = await this.executeAnalysisTask(task);
          break;
        case 'installation':
          result = await this.executeInstallationTask(task);
          break;
        case 'custom':
          result = await this.executeCustomTask(task);
          break;
        default:
          throw new Error(`Unknown task type: ${task.type}`);
      }

      // Task completed successfully
      task.status = 'completed';
      task.result = result;
      task.endTime = new Date();
      task.duration = task.endTime.getTime() - task.startTime!.getTime();
      
      this.successfulTasks++;
      console.log(`✅ Task completed: ${task.name} (${task.duration}ms)`);
      this.emit('task:completed', task);

    } catch (error) {
      console.error(`❌ Task failed: ${task.name}`, error);
      
      task.error = error instanceof Error ? error.message : 'Unknown error';
      task.retryCount++;

      // Retry if under limit
      if (task.retryCount < task.maxRetries) {
        console.log(`🔄 Retrying task: ${task.name} (attempt ${task.retryCount + 1}/${task.maxRetries})`);
        task.status = 'pending';
        task.startTime = undefined;
        this.taskQueue.unshift(task); // Add back to front of queue
        this.emit('task:retry', task);
      } else {
        task.status = 'failed';
        task.endTime = new Date();
        task.duration = task.endTime.getTime() - task.startTime!.getTime();
        this.failedTasks++;
        this.emit('task:failed', task);
      }
    } finally {
      this.totalTasksProcessed++;
      this.currentTasks.delete(task.id);
      await this.updateTaskInDatabase(task);
      
      // Log activity
      await this.databaseService.logActivity({
        taskId: task.id,
        type: task.status === 'completed' ? 'success' : task.status === 'failed' ? 'error' : 'info',
        category: 'agent',
        message: `Task ${task.status}: ${task.name}`,
        details: JSON.stringify({
          taskType: task.type,
          duration: task.duration,
          retryCount: task.retryCount,
          error: task.error
        }),
        source: 'AgentService'
      });
    }
  }

  /**
   * Execute screen capture task
   */
  private async executeScreenCaptureTask(task: AgentTask): Promise<any> {
    const options: CaptureOptions = task.parameters || {};
    
    const result = await this.screenCaptureService.captureScreen(options);
    
    // Save to database
    await this.databaseService.saveScreenCapture({
      taskId: task.id,
      imagePath: result.filepath,
      metadata: JSON.stringify(result.metadata)
    });

    return {
      captureId: result.id,
      filepath: result.filepath,
      metadata: result.metadata
    };
  }

  /**
   * Execute social media task
   */
  private async executeSocialMediaTask(task: AgentTask): Promise<any> {
    const { action, platform, content, mediaUrls, contentOptions, imagePath } = task.parameters;

    switch (action) {
      case 'post':
        const posts: SocialMediaPost[] = [{
          platform,
          content,
          mediaUrls
        }];
        
        const results = await this.socialMediaService.postToMultiplePlatforms(posts);
        
        // Save posts to database
        for (const result of results) {
          await this.databaseService.saveSocialMediaPost({
            platform: result.platform as any,
            content,
            mediaUrls: mediaUrls ? JSON.stringify(mediaUrls) : undefined,
            status: result.success ? 'posted' : 'failed',
            postId: result.postId,
            postedAt: result.success ? new Date() : undefined
          });
        }
        
        return results;

      case 'analyze_and_post':
        if (!imagePath) throw new Error('Image path required for analyze_and_post action');
        
        const { analysis, generatedContent } = await this.socialMediaService.analyzeAndGenerateContent(
          imagePath,
          contentOptions
        );
        
        const postResult = await this.socialMediaService.postToTwitter(generatedContent, [imagePath]);
        
        return {
          analysis,
          content: generatedContent,
          postResult
        };

      case 'analyze_image':
        if (!imagePath) throw new Error('Image path required for analyze_image action');
        
        const analysisResult = await this.socialMediaService.analyzeImage(imagePath);
        return analysisResult;

      default:
        throw new Error(`Unknown social media action: ${action}`);
    }
  }

  /**
   * Execute automation task
   */
  private async executeAutomationTask(task: AgentTask): Promise<any> {
    const { ruleId, action } = task.parameters;

    switch (action) {
      case 'execute_rule':
        const rule = this.automationRules.get(ruleId);
        if (!rule) throw new Error(`Automation rule not found: ${ruleId}`);
        
        return await this.executeAutomationRule(rule);

      default:
        throw new Error(`Unknown automation action: ${action}`);
    }
  }

  /**
   * Execute analysis task
   */
  private async executeAnalysisTask(task: AgentTask): Promise<any> {
    const { type, parameters } = task.parameters;

    switch (type) {
      case 'screen_comparison':
        const { image1, image2, threshold } = parameters;
        return await this.screenCaptureService.compareScreenshots(image1, image2, threshold);

      case 'storage_analysis':
        return await this.screenCaptureService.getStorageStats();

      default:
        throw new Error(`Unknown analysis type: ${type}`);
    }
  }

  /**
   * Execute installation task (placeholder)
   */
  private async executeInstallationTask(task: AgentTask): Promise<any> {
    // This would contain actual installation logic
    console.log('🔧 Installation task execution not yet implemented');
    return { message: 'Installation task completed (placeholder)' };
  }

  /**
   * Execute custom task
   */
  private async executeCustomTask(task: AgentTask): Promise<any> {
    // Custom task execution based on parameters
    const { script, command, parameters } = task.parameters;
    
    if (script) {
      // Execute custom script (would need security considerations)
      console.log('📜 Custom script execution not implemented for security');
      return { message: 'Custom script execution disabled for security' };
    }
    
    if (command) {
      // Execute system command (would need security considerations)
      console.log('💻 System command execution not implemented for security');
      return { message: 'System command execution disabled for security' };
    }

    return { message: 'Custom task completed' };
  }

  /**
   * Execute automation rule
   */
  private async executeAutomationRule(rule: AutomationRule): Promise<any> {
    console.log(`🔧 Executing automation rule: ${rule.name}`);

    // Check conditions
    const conditionsMet = await this.checkAutomationConditions(rule.conditions);
    if (!conditionsMet) {
      console.log(`⏭️ Skipping rule execution - conditions not met: ${rule.name}`);
      return { skipped: true, reason: 'conditions_not_met' };
    }

    // Execute actions
    const actionResults = [];
    for (const action of rule.actions) {
      try {
        const result = await this.executeAutomationAction(action);
        actionResults.push({ action: action.type, success: true, result });
      } catch (error) {
        console.error(`❌ Automation action failed: ${action.type}`, error);
        actionResults.push({ 
          action: action.type, 
          success: false, 
          error: error instanceof Error ? error.message : 'Unknown error' 
        });
      }
    }

    // Update rule execution stats
    rule.lastExecuted = new Date();
    rule.executionCount++;

    return {
      ruleId: rule.id,
      executedAt: rule.lastExecuted,
      actionResults
    };
  }

  /**
   * Check automation rule conditions
   */
  private async checkAutomationConditions(conditions: AutomationRule['conditions']): Promise<boolean> {
    for (const condition of conditions) {
      const result = await this.evaluateCondition(condition);
      if (!result) return false;
    }
    return true;
  }

  /**
   * Evaluate a single condition
   */
  private async evaluateCondition(condition: AutomationRule['conditions'][0]): Promise<boolean> {
    // Implementation would depend on condition type
    switch (condition.type) {
      case 'time_range':
        // Check if current time is within range
        const now = new Date();
        const { start, end } = condition.value;
        const currentTime = now.getHours() * 60 + now.getMinutes();
        const startTime = parseInt(start.split(':')[0]) * 60 + parseInt(start.split(':')[1]);
        const endTime = parseInt(end.split(':')[0]) * 60 + parseInt(end.split(':')[1]);
        return currentTime >= startTime && currentTime <= endTime;

      case 'file_exists':
        // Check if file exists (would need file system access)
        return true; // Placeholder

      default:
        console.warn(`Unknown condition type: ${condition.type}`);
        return true;
    }
  }

  /**
   * Execute automation action
   */
  private async executeAutomationAction(action: AutomationRule['actions'][0]): Promise<any> {
    switch (action.type) {
      case 'screen_capture':
        return await this.addTask({
          name: 'Automated Screen Capture',
          type: 'screen_capture',
          priority: 1,
          parameters: action.parameters
        });

      case 'social_media_post':
        return await this.addTask({
          name: 'Automated Social Media Post',
          type: 'social_media',
          priority: 1,
          parameters: action.parameters
        });

      default:
        throw new Error(`Unknown automation action: ${action.type}`);
    }
  }

  /**
   * Add automation rule
   */
  public addAutomationRule(rule: Omit<AutomationRule, 'id' | 'lastExecuted' | 'executionCount'>): string {
    const automationRule: AutomationRule = {
      ...rule,
      id: uuidv4(),
      executionCount: 0
    };

    this.automationRules.set(automationRule.id, automationRule);

    if (automationRule.isActive && automationRule.trigger.type === 'schedule') {
      this.scheduleAutomationRule(automationRule);
    }

    console.log(`📋 Automation rule added: ${automationRule.name}`);
    this.emit('automation:rule_added', automationRule);

    return automationRule.id;
  }

  /**
   * Schedule automation rule
   */
  private scheduleAutomationRule(rule: AutomationRule): void {
    if (rule.trigger.type !== 'schedule' || !rule.trigger.schedule) return;

    const job = cron.schedule(rule.trigger.schedule, async () => {
      console.log(`⏰ Executing scheduled rule: ${rule.name}`);
      
      await this.addTask({
        name: `Scheduled: ${rule.name}`,
        type: 'automation',
        priority: 2,
        parameters: {
          ruleId: rule.id,
          action: 'execute_rule'
        }
      });
    }, {
      scheduled: false
    });

    job.start();
    this.scheduledJobs.set(rule.id, job);
    console.log(`⏰ Scheduled automation rule: ${rule.name} (${rule.trigger.schedule})`);
  }

  /**
   * Activate all automation rules
   */
  private activateAutomationRules(): void {
    for (const rule of this.automationRules.values()) {
      if (rule.isActive && rule.trigger.type === 'schedule') {
        this.scheduleAutomationRule(rule);
      }
    }
  }

  // Database operations
  private async saveTaskToDatabase(task: AgentTask): Promise<void> {
    await this.databaseService.createTask({
      name: task.name,
      description: `${task.type} task`,
      status: task.status,
      type: task.type,
      priority: task.priority,
      parameters: JSON.stringify(task.parameters),
      result: task.result ? JSON.stringify(task.result) : undefined,
      error: task.error,
      startTime: task.startTime,
      endTime: task.endTime,
      duration: task.duration,
      retryCount: task.retryCount,
      maxRetries: task.maxRetries
    });
  }

  private async updateTaskInDatabase(task: AgentTask): Promise<void> {
    await this.databaseService.updateTask(task.id, {
      status: task.status,
      result: task.result ? JSON.stringify(task.result) : undefined,
      error: task.error,
      startTime: task.startTime,
      endTime: task.endTime,
      duration: task.duration,
      retryCount: task.retryCount
    });
  }

  private async loadPendingTasks(): Promise<void> {
    const pendingTasks = await this.databaseService.getTasks({
      status: 'pending',
      limit: 100
    });

    for (const dbTask of pendingTasks) {
      const agentTask: AgentTask = {
        id: dbTask.id,
        name: dbTask.name,
        type: dbTask.type,
        priority: dbTask.priority,
        parameters: dbTask.parameters ? JSON.parse(dbTask.parameters) : {},
        status: dbTask.status,
        retryCount: dbTask.retryCount,
        maxRetries: dbTask.maxRetries,
        result: dbTask.result ? JSON.parse(dbTask.result) : undefined,
        error: dbTask.error,
        startTime: dbTask.startTime,
        endTime: dbTask.endTime,
        duration: dbTask.duration
      };

      this.taskQueue.push(agentTask);
    }

    this.taskQueue.sort((a, b) => b.priority - a.priority);
    console.log(`📋 Loaded ${pendingTasks.length} pending tasks from database`);
  }

  private async loadAutomationRules(): Promise<void> {
    // In a real implementation, this would load from database
    // For now, we'll start with empty rules
    console.log('📋 Automation rules loaded (placeholder)');
  }

  // Public API methods
  public getStatus(): AgentStatus {
    return {
      isRunning: this.isRunning,
      currentTasks: this.currentTasks.size,
      maxConcurrentTasks: this.maxConcurrentTasks,
      totalTasksProcessed: this.totalTasksProcessed,
      successfulTasks: this.successfulTasks,
      failedTasks: this.failedTasks,
      queuedTasks: this.taskQueue.length,
      uptime: Date.now() - this.startTime.getTime(),
      lastActivity: this.lastActivity
    };
  }

  public async getTasks(options?: any): Promise<AgentTask[]> {
    // Convert database tasks to agent tasks
    const dbTasks = await this.databaseService.getTasks(options);
    return dbTasks.map(dbTask => ({
      id: dbTask.id,
      name: dbTask.name,
      type: dbTask.type,
      priority: dbTask.priority,
      parameters: dbTask.parameters ? JSON.parse(dbTask.parameters) : {},
      status: dbTask.status,
      retryCount: dbTask.retryCount,
      maxRetries: dbTask.maxRetries,
      result: dbTask.result ? JSON.parse(dbTask.result) : undefined,
      error: dbTask.error,
      startTime: dbTask.startTime,
      endTime: dbTask.endTime,
      duration: dbTask.duration
    }));
  }

  public async cancelTask(taskId: string): Promise<boolean> {
    // Check if task is currently running
    const runningTask = this.currentTasks.get(taskId);
    if (runningTask) {
      runningTask.status = 'cancelled';
      this.currentTasks.delete(taskId);
      await this.updateTaskInDatabase(runningTask);
      this.emit('task:cancelled', runningTask);
      return true;
    }

    // Check if task is in queue
    const queueIndex = this.taskQueue.findIndex(task => task.id === taskId);
    if (queueIndex !== -1) {
      const task = this.taskQueue[queueIndex];
      task.status = 'cancelled';
      this.taskQueue.splice(queueIndex, 1);
      await this.updateTaskInDatabase(task);
      this.emit('task:cancelled', task);
      return true;
    }

    return false;
  }

  public setMaxConcurrentTasks(max: number): void {
    this.maxConcurrentTasks = Math.max(1, Math.min(max, 10)); // Limit between 1-10
    console.log(`📊 Max concurrent tasks set to: ${this.maxConcurrentTasks}`);
  }

  public getAutomationRules(): AutomationRule[] {
    return Array.from(this.automationRules.values());
  }

  public getAutomationRule(id: string): AutomationRule | undefined {
    return this.automationRules.get(id);
  }

  public async deleteAutomationRule(id: string): Promise<boolean> {
    const rule = this.automationRules.get(id);
    if (!rule) return false;

    // Stop scheduled job if exists
    const job = this.scheduledJobs.get(id);
    if (job) {
      job.stop();
      this.scheduledJobs.delete(id);
    }

    this.automationRules.delete(id);
    console.log(`🗑️ Automation rule deleted: ${rule.name}`);
    this.emit('automation:rule_deleted', rule);

    return true;
  }
}