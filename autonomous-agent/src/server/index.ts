import express from 'express';
import { createServer } from 'http';
import { Server } from 'socket.io';
import cors from 'cors';
import dotenv from 'dotenv';
import path from 'path';
import { fileURLToPath } from 'url';

import { DatabaseService } from './services/DatabaseService.js';
import { AgentService } from './services/AgentService.js';
import { ScreenCaptureService } from './services/ScreenCaptureService.js';
import { SocialMediaService } from './services/SocialMediaService.js';
import { ConfigService } from './config/ConfigService.js';
import { SocketHandler } from './services/SocketHandler.js';

// Routes
import agentRoutes from './routes/agent.js';
import systemRoutes from './routes/system.js';
import socialRoutes from './routes/social.js';
import installerRoutes, { initializeInstallerRoutes } from './routes/installer.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Load environment variables
dotenv.config();

class Server {
  private app: express.Application;
  private server: any;
  private io: Server;
  private port: number;
  
  // Services
  private databaseService: DatabaseService;
  private agentService: AgentService;
  private screenCaptureService: ScreenCaptureService;
  private socialMediaService: SocialMediaService;
  private configService: ConfigService;
  private socketHandler: SocketHandler;

  constructor() {
    this.app = express();
    this.server = createServer(this.app);
    this.io = new Server(this.server, {
      cors: {
        origin: "*",
        methods: ["GET", "POST"]
      }
    });
    this.port = parseInt(process.env.PORT || '3001');

    this.initializeServices();
    this.configureMiddleware();
    this.configureRoutes();
    this.configureSocketHandlers();
  }

  private async initializeServices(): Promise<void> {
    try {
      console.log('🚀 Initializing services...');
      
      // Initialize core services
      this.configService = new ConfigService();
      this.databaseService = new DatabaseService();
      await this.databaseService.initialize();
      
      this.screenCaptureService = new ScreenCaptureService();
      this.socialMediaService = new SocialMediaService(this.configService);
      this.agentService = new AgentService(
        this.databaseService,
        this.screenCaptureService,
        this.socialMediaService
      );
      
      this.socketHandler = new SocketHandler(
        this.io,
        this.agentService,
        this.screenCaptureService
      );

      console.log('✅ All services initialized successfully');
    } catch (error) {
      console.error('❌ Service initialization failed:', error);
      process.exit(1);
    }
  }

  private configureMiddleware(): void {
    // CORS configuration
    this.app.use(cors({
      origin: process.env.NODE_ENV === 'production' 
        ? process.env.FRONTEND_URL 
        : '*',
      credentials: true
    }));

    // Body parsing
    this.app.use(express.json({ limit: '50mb' }));
    this.app.use(express.urlencoded({ extended: true, limit: '50mb' }));

    // Static files
    this.app.use('/uploads', express.static(path.join(__dirname, '../uploads')));
    
    // Logging middleware
    this.app.use((req, res, next) => {
      console.log(`${new Date().toISOString()} - ${req.method} ${req.path}`);
      next();
    });
  }

  private configureRoutes(): void {
    // Health check
    this.app.get('/health', (req, res) => {
      res.json({ 
        status: 'healthy', 
        timestamp: new Date().toISOString(),
        uptime: process.uptime(),
        services: {
          database: this.databaseService.isConnected(),
          agent: this.agentService.getStatus(),
          screenCapture: this.screenCaptureService.isActive()
        }
      });
    });

    // API routes
    this.app.use('/api/agent', agentRoutes);
    this.app.use('/api/system', systemRoutes);
    this.app.use('/api/social', socialRoutes);
    this.app.use('/api/installer', initializeInstallerRoutes(this.databaseService, this.screenCaptureService));

    // 404 handler
    this.app.use('*', (req, res) => {
      res.status(404).json({ error: 'Route not found' });
    });

    // Global error handler
    this.app.use((error: any, req: express.Request, res: express.Response, next: express.NextFunction) => {
      console.error('Global error handler:', error);
      res.status(500).json({ 
        error: 'Internal server error',
        message: process.env.NODE_ENV === 'development' ? error.message : undefined
      });
    });
  }

  private configureSocketHandlers(): void {
    this.io.on('connection', (socket) => {
      console.log(`🔌 Client connected: ${socket.id}`);
      
      socket.on('disconnect', () => {
        console.log(`🔌 Client disconnected: ${socket.id}`);
      });
    });
  }

  public start(): void {
    this.server.listen(this.port, () => {
      console.log(`
╔══════════════════════════════════════════════════════════════╗
║                 🤖 Autonomous Agent Server                   ║
║                                                              ║
║  Server running on port ${this.port}                                   ║
║  Environment: ${process.env.NODE_ENV || 'development'}                            ║
║  Database: Connected                                         ║
║  Socket.io: Active                                           ║
║                                                              ║
║  Health Check: http://localhost:${this.port}/health                ║
╚══════════════════════════════════════════════════════════════╝
      `);
    });

    // Graceful shutdown
    process.on('SIGTERM', () => this.shutdown());
    process.on('SIGINT', () => this.shutdown());
  }

  private async shutdown(): Promise<void> {
    console.log('🛑 Shutting down server...');
    
    // Stop agent operations
    if (this.agentService) {
      await this.agentService.stop();
    }

    // Close database connection
    if (this.databaseService) {
      await this.databaseService.close();
    }

    // Close server
    this.server.close(() => {
      console.log('✅ Server shutdown complete');
      process.exit(0);
    });

    // Force exit after 10 seconds
    setTimeout(() => {
      console.log('⚠️ Forced shutdown');
      process.exit(1);
    }, 10000);
  }
}

// Start the server
const server = new Server();
server.start();

export default Server;