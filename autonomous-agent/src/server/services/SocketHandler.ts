import { Server, Socket } from 'socket.io';
import { AgentService } from './AgentService.js';
import { ScreenCaptureService } from './ScreenCaptureService.js';

export interface SocketClient {
  id: string;
  socket: Socket;
  connectedAt: Date;
  subscriptions: Set<string>;
}

export class SocketHandler {
  private io: Server;
  private agentService: AgentService;
  private screenCaptureService: ScreenCaptureService;
  private clients: Map<string, SocketClient> = new Map();
  private liveScreenCapture: boolean = false;
  private screenCaptureInterval: NodeJS.Timeout | null = null;

  constructor(
    io: Server,
    agentService: AgentService,
    screenCaptureService: ScreenCaptureService
  ) {
    this.io = io;
    this.agentService = agentService;
    this.screenCaptureService = screenCaptureService;

    this.setupSocketHandlers();
    this.setupAgentEventListeners();
  }

  private setupSocketHandlers(): void {
    this.io.on('connection', (socket: Socket) => {
      console.log(`🔌 Client connected: ${socket.id}`);
      
      // Register client
      const client: SocketClient = {
        id: socket.id,
        socket,
        connectedAt: new Date(),
        subscriptions: new Set()
      };
      this.clients.set(socket.id, client);

      // Send initial data
      this.sendInitialData(socket);

      // Handle client events
      this.handleClientEvents(socket, client);

      // Handle disconnection
      socket.on('disconnect', () => {
        console.log(`🔌 Client disconnected: ${socket.id}`);
        this.clients.delete(socket.id);
        
        // Stop live screen capture if no clients are subscribed
        this.checkLiveScreenCapture();
      });
    });
  }

  private async sendInitialData(socket: Socket): Promise<void> {
    try {
      // Send agent status
      const agentStatus = this.agentService.getStatus();
      socket.emit('agent:status', agentStatus);

      // Send recent tasks
      const recentTasks = await this.agentService.getTasks({
        limit: 20,
        orderBy: 'created_at',
        orderDirection: 'DESC'
      });
      socket.emit('tasks:list', recentTasks);

      // Send automation rules
      const automationRules = this.agentService.getAutomationRules();
      socket.emit('automation:rules', automationRules);

      // Send screen capture service status
      const screenStatus = {
        isActive: this.screenCaptureService.isActive(),
        queueLength: this.screenCaptureService.getQueueLength(),
        storageStats: await this.screenCaptureService.getStorageStats()
      };
      socket.emit('screen:status', screenStatus);

    } catch (error) {
      console.error('❌ Failed to send initial data:', error);
      socket.emit('error', { message: 'Failed to load initial data' });
    }
  }

  private handleClientEvents(socket: Socket, client: SocketClient): void {
    // Agent control events
    socket.on('agent:start', async () => {
      try {
        await this.agentService.start();
        socket.emit('agent:started');
      } catch (error) {
        socket.emit('error', { message: `Failed to start agent: ${error instanceof Error ? error.message : 'Unknown error'}` });
      }
    });

    socket.on('agent:stop', async () => {
      try {
        await this.agentService.stop();
        socket.emit('agent:stopped');
      } catch (error) {
        socket.emit('error', { message: `Failed to stop agent: ${error instanceof Error ? error.message : 'Unknown error'}` });
      }
    });

    // Task management events
    socket.on('task:add', async (taskData) => {
      try {
        const taskId = await this.agentService.addTask(taskData);
        socket.emit('task:added', { id: taskId });
      } catch (error) {
        socket.emit('error', { message: `Failed to add task: ${error instanceof Error ? error.message : 'Unknown error'}` });
      }
    });

    socket.on('task:cancel', async (taskId: string) => {
      try {
        const cancelled = await this.agentService.cancelTask(taskId);
        socket.emit('task:cancelled', { id: taskId, success: cancelled });
      } catch (error) {
        socket.emit('error', { message: `Failed to cancel task: ${error instanceof Error ? error.message : 'Unknown error'}` });
      }
    });

    socket.on('tasks:get', async (options) => {
      try {
        const tasks = await this.agentService.getTasks(options);
        socket.emit('tasks:list', tasks);
      } catch (error) {
        socket.emit('error', { message: `Failed to get tasks: ${error instanceof Error ? error.message : 'Unknown error'}` });
      }
    });

    // Screen capture events
    socket.on('screen:capture', async (options) => {
      try {
        const result = await this.screenCaptureService.captureScreen(options);
        socket.emit('screen:captured', result);
      } catch (error) {
        socket.emit('error', { message: `Screen capture failed: ${error instanceof Error ? error.message : 'Unknown error'}` });
      }
    });

    socket.on('screen:live:start', () => {
      client.subscriptions.add('live_screen');
      this.startLiveScreenCapture();
      socket.emit('screen:live:started');
    });

    socket.on('screen:live:stop', () => {
      client.subscriptions.delete('live_screen');
      this.checkLiveScreenCapture();
      socket.emit('screen:live:stopped');
    });

    socket.on('screen:compare', async (data) => {
      try {
        const { image1, image2, threshold } = data;
        const result = await this.screenCaptureService.compareScreenshots(image1, image2, threshold);
        socket.emit('screen:compared', result);
      } catch (error) {
        socket.emit('error', { message: `Screen comparison failed: ${error instanceof Error ? error.message : 'Unknown error'}` });
      }
    });

    socket.on('screen:cleanup', async (maxAge) => {
      try {
        const deletedCount = await this.screenCaptureService.cleanupOldScreenshots(maxAge);
        socket.emit('screen:cleaned', { deletedCount });
      } catch (error) {
        socket.emit('error', { message: `Cleanup failed: ${error instanceof Error ? error.message : 'Unknown error'}` });
      }
    });

    // Automation events
    socket.on('automation:rule:add', (ruleData) => {
      try {
        const ruleId = this.agentService.addAutomationRule(ruleData);
        socket.emit('automation:rule:added', { id: ruleId });
      } catch (error) {
        socket.emit('error', { message: `Failed to add automation rule: ${error instanceof Error ? error.message : 'Unknown error'}` });
      }
    });

    socket.on('automation:rule:delete', async (ruleId: string) => {
      try {
        const deleted = await this.agentService.deleteAutomationRule(ruleId);
        socket.emit('automation:rule:deleted', { id: ruleId, success: deleted });
      } catch (error) {
        socket.emit('error', { message: `Failed to delete automation rule: ${error instanceof Error ? error.message : 'Unknown error'}` });
      }
    });

    socket.on('automation:rules:get', () => {
      try {
        const rules = this.agentService.getAutomationRules();
        socket.emit('automation:rules', rules);
      } catch (error) {
        socket.emit('error', { message: `Failed to get automation rules: ${error instanceof Error ? error.message : 'Unknown error'}` });
      }
    });

    // Configuration events
    socket.on('config:update', (configData) => {
      try {
        // Handle configuration updates
        if (configData.maxConcurrentTasks) {
          this.agentService.setMaxConcurrentTasks(configData.maxConcurrentTasks);
        }
        socket.emit('config:updated');
      } catch (error) {
        socket.emit('error', { message: `Failed to update configuration: ${error instanceof Error ? error.message : 'Unknown error'}` });
      }
    });

    // Status requests
    socket.on('status:get', () => {
      try {
        const status = {
          agent: this.agentService.getStatus(),
          screen: {
            isActive: this.screenCaptureService.isActive(),
            queueLength: this.screenCaptureService.getQueueLength()
          },
          clients: this.clients.size,
          liveScreenCapture: this.liveScreenCapture
        };
        socket.emit('status:data', status);
      } catch (error) {
        socket.emit('error', { message: `Failed to get status: ${error instanceof Error ? error.message : 'Unknown error'}` });
      }
    });

    // Ping/pong for connection health
    socket.on('ping', () => {
      socket.emit('pong', { timestamp: Date.now() });
    });
  }

  private setupAgentEventListeners(): void {
    // Agent events
    this.agentService.on('agent:started', () => {
      this.broadcast('agent:started');
    });

    this.agentService.on('agent:stopped', () => {
      this.broadcast('agent:stopped');
    });

    this.agentService.on('agent:error', (error) => {
      this.broadcast('agent:error', { error: error.message });
    });

    // Task events
    this.agentService.on('task:added', (task) => {
      this.broadcast('task:added', task);
    });

    this.agentService.on('task:started', (task) => {
      this.broadcast('task:started', task);
    });

    this.agentService.on('task:completed', (task) => {
      this.broadcast('task:completed', task);
    });

    this.agentService.on('task:failed', (task) => {
      this.broadcast('task:failed', task);
    });

    this.agentService.on('task:cancelled', (task) => {
      this.broadcast('task:cancelled', task);
    });

    this.agentService.on('task:retry', (task) => {
      this.broadcast('task:retry', task);
    });

    // Automation events
    this.agentService.on('automation:rule_added', (rule) => {
      this.broadcast('automation:rule_added', rule);
    });

    this.agentService.on('automation:rule_deleted', (rule) => {
      this.broadcast('automation:rule_deleted', rule);
    });

    // Status updates (periodic)
    setInterval(() => {
      const status = this.agentService.getStatus();
      this.broadcast('agent:status', status);
    }, 5000); // Update every 5 seconds
  }

  private startLiveScreenCapture(): void {
    if (this.liveScreenCapture) return;

    this.liveScreenCapture = true;
    console.log('📸 Starting live screen capture');

    const captureLoop = async () => {
      if (!this.liveScreenCapture) return;

      try {
        const result = await this.screenCaptureService.captureScreen({
          format: 'jpg',
          quality: 70,
          resize: { width: 800, height: 600 }
        });

        // Send to subscribed clients
        this.broadcastToSubscribers('live_screen', 'screen:live:frame', {
          image: result.filepath,
          timestamp: result.metadata.timestamp,
          metadata: result.metadata
        });

      } catch (error) {
        console.error('❌ Live screen capture error:', error);
        this.broadcastToSubscribers('live_screen', 'screen:live:error', {
          error: error instanceof Error ? error.message : 'Unknown error'
        });
      }

      // Schedule next capture
      if (this.liveScreenCapture) {
        this.screenCaptureInterval = setTimeout(captureLoop, 2000); // 2 second interval
      }
    };

    captureLoop();
  }

  private checkLiveScreenCapture(): void {
    const hasSubscribers = Array.from(this.clients.values())
      .some(client => client.subscriptions.has('live_screen'));

    if (!hasSubscribers && this.liveScreenCapture) {
      this.stopLiveScreenCapture();
    }
  }

  private stopLiveScreenCapture(): void {
    if (!this.liveScreenCapture) return;

    this.liveScreenCapture = false;
    
    if (this.screenCaptureInterval) {
      clearTimeout(this.screenCaptureInterval);
      this.screenCaptureInterval = null;
    }

    console.log('🛑 Stopped live screen capture');
  }

  private broadcast(event: string, data?: any): void {
    this.io.emit(event, data);
  }

  private broadcastToSubscribers(subscription: string, event: string, data?: any): void {
    for (const client of this.clients.values()) {
      if (client.subscriptions.has(subscription)) {
        client.socket.emit(event, data);
      }
    }
  }

  // Public methods for external use
  public getConnectedClients(): number {
    return this.clients.size;
  }

  public getClientSubscriptions(): Map<string, string[]> {
    const subscriptions = new Map<string, string[]>();
    
    for (const [clientId, client] of this.clients) {
      subscriptions.set(clientId, Array.from(client.subscriptions));
    }
    
    return subscriptions;
  }

  public sendToClient(clientId: string, event: string, data?: any): boolean {
    const client = this.clients.get(clientId);
    if (client) {
      client.socket.emit(event, data);
      return true;
    }
    return false;
  }

  public disconnectClient(clientId: string): boolean {
    const client = this.clients.get(clientId);
    if (client) {
      client.socket.disconnect();
      return true;
    }
    return false;
  }

  public async getSystemStats(): Promise<any> {
    return {
      connectedClients: this.clients.size,
      liveScreenCapture: this.liveScreenCapture,
      agentStatus: this.agentService.getStatus(),
      screenCaptureStats: await this.screenCaptureService.getStorageStats(),
      clientConnections: Array.from(this.clients.values()).map(client => ({
        id: client.id,
        connectedAt: client.connectedAt,
        subscriptions: Array.from(client.subscriptions)
      }))
    };
  }
}