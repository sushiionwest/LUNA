import express from 'express';
import { ScreenCaptureService } from '../services/ScreenCaptureService.js';
import { DatabaseService } from '../services/DatabaseService.js';
import { SocketHandler } from '../services/SocketHandler.js';
import path from 'path';
import fs from 'fs/promises';

const router = express.Router();

// GET /api/system/status
router.get('/status', async (req, res) => {
  try {
    const socketHandler = req.app.get('socketHandler') as SocketHandler;
    const screenCaptureService = req.app.get('screenCaptureService') as ScreenCaptureService;
    
    const systemStats = await socketHandler.getSystemStats();
    const screenStats = await screenCaptureService.getStorageStats();
    
    const status = {
      server: {
        uptime: process.uptime(),
        memory: process.memoryUsage(),
        cpu: process.cpuUsage(),
        platform: process.platform,
        nodeVersion: process.version,
        environment: process.env.NODE_ENV || 'development'
      },
      services: {
        database: true, // Would check actual database connection
        screenCapture: screenCaptureService.isActive(),
        socketConnections: systemStats.connectedClients
      },
      storage: screenStats,
      performance: {
        totalTasks: systemStats.agentStatus.totalTasksProcessed,
        successfulTasks: systemStats.agentStatus.successfulTasks,
        failedTasks: systemStats.agentStatus.failedTasks,
        currentTasks: systemStats.agentStatus.currentTasks
      }
    };

    res.json(status);
  } catch (error) {
    console.error('Failed to get system status:', error);
    res.status(500).json({ 
      error: 'Failed to get system status',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/system/logs
router.get('/logs', async (req, res) => {
  try {
    const databaseService = req.app.get('databaseService') as DatabaseService;
    
    const options = {
      type: req.query.type as string,
      category: req.query.category as string,
      limit: req.query.limit ? parseInt(req.query.limit as string) : 100,
      offset: req.query.offset ? parseInt(req.query.offset as string) : 0
    };

    const logs = await databaseService.getActivityLogs(options);
    res.json(logs);
  } catch (error) {
    console.error('Failed to get logs:', error);
    res.status(500).json({ 
      error: 'Failed to get logs',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/system/metrics
router.get('/metrics', async (req, res) => {
  try {
    const databaseService = req.app.get('databaseService') as DatabaseService;
    
    const options = {
      metric: req.query.metric as string,
      category: req.query.category as string,
      startTime: req.query.startTime ? new Date(req.query.startTime as string) : undefined,
      endTime: req.query.endTime ? new Date(req.query.endTime as string) : undefined,
      limit: req.query.limit ? parseInt(req.query.limit as string) : 1000
    };

    const metrics = await databaseService.getMetrics(options);
    res.json(metrics);
  } catch (error) {
    console.error('Failed to get metrics:', error);
    res.status(500).json({ 
      error: 'Failed to get metrics',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/system/metrics
router.post('/metrics', async (req, res) => {
  try {
    const databaseService = req.app.get('databaseService') as DatabaseService;
    
    const { metric, value, unit, category = 'usage' } = req.body;

    if (!metric || value === undefined || !unit) {
      return res.status(400).json({ 
        error: 'Missing required fields',
        message: 'metric, value, and unit are required'
      });
    }

    const metricId = await databaseService.recordMetric({
      metric,
      value: parseFloat(value),
      unit,
      category
    });

    res.status(201).json({ 
      success: true, 
      metricId,
      message: 'Metric recorded successfully' 
    });
  } catch (error) {
    console.error('Failed to record metric:', error);
    res.status(500).json({ 
      error: 'Failed to record metric',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/system/screenshots
router.get('/screenshots', async (req, res) => {
  try {
    const screenCaptureService = req.app.get('screenCaptureService') as ScreenCaptureService;
    
    const captureDir = screenCaptureService.getCaptureDirectory();
    const files = await fs.readdir(captureDir);
    
    const screenshots = await Promise.all(
      files
        .filter(file => /\.(png|jpg|jpeg|webp)$/i.test(file))
        .slice(0, 50) // Limit to last 50 screenshots
        .map(async (filename) => {
          const filepath = path.join(captureDir, filename);
          const stats = await fs.stat(filepath);
          
          return {
            filename,
            filepath: `/uploads/${filename}`, // Public URL path
            size: stats.size,
            createdAt: stats.birthtime,
            modifiedAt: stats.mtime
          };
        })
    );

    // Sort by creation date, newest first
    screenshots.sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime());

    res.json(screenshots);
  } catch (error) {
    console.error('Failed to get screenshots:', error);
    res.status(500).json({ 
      error: 'Failed to get screenshots',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/system/screenshots/capture
router.post('/screenshots/capture', async (req, res) => {
  try {
    const screenCaptureService = req.app.get('screenCaptureService') as ScreenCaptureService;
    
    const options = req.body || {};
    
    const result = await screenCaptureService.captureScreen(options);
    
    res.status(201).json({ 
      success: true, 
      capture: {
        id: result.id,
        filename: result.filename,
        filepath: `/uploads/${result.filename}`,
        metadata: result.metadata
      },
      message: 'Screenshot captured successfully' 
    });
  } catch (error) {
    console.error('Failed to capture screenshot:', error);
    res.status(500).json({ 
      error: 'Failed to capture screenshot',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/system/screenshots/compare
router.post('/screenshots/compare', async (req, res) => {
  try {
    const screenCaptureService = req.app.get('screenCaptureService') as ScreenCaptureService;
    
    const { image1, image2, threshold = 0.1 } = req.body;

    if (!image1 || !image2) {
      return res.status(400).json({ 
        error: 'Missing required fields',
        message: 'image1 and image2 paths are required'
      });
    }

    const result = await screenCaptureService.compareScreenshots(image1, image2, threshold);
    
    res.json({
      success: true,
      comparison: result
    });
  } catch (error) {
    console.error('Failed to compare screenshots:', error);
    res.status(500).json({ 
      error: 'Failed to compare screenshots',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// DELETE /api/system/screenshots/cleanup
router.delete('/screenshots/cleanup', async (req, res) => {
  try {
    const screenCaptureService = req.app.get('screenCaptureService') as ScreenCaptureService;
    
    const maxAge = req.query.maxAge ? parseInt(req.query.maxAge as string) : 24 * 60 * 60 * 1000; // 24 hours default
    
    const deletedCount = await screenCaptureService.cleanupOldScreenshots(maxAge);
    
    res.json({
      success: true,
      deletedCount,
      message: `Cleaned up ${deletedCount} old screenshots`
    });
  } catch (error) {
    console.error('Failed to cleanup screenshots:', error);
    res.status(500).json({ 
      error: 'Failed to cleanup screenshots',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/system/screenshots/stats
router.get('/screenshots/stats', async (req, res) => {
  try {
    const screenCaptureService = req.app.get('screenCaptureService') as ScreenCaptureService;
    
    const stats = await screenCaptureService.getStorageStats();
    
    res.json({
      success: true,
      stats
    });
  } catch (error) {
    console.error('Failed to get screenshot stats:', error);
    res.status(500).json({ 
      error: 'Failed to get screenshot stats',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/system/screen/info
router.get('/screen/info', async (req, res) => {
  try {
    const screenCaptureService = req.app.get('screenCaptureService') as ScreenCaptureService;
    
    const screenInfo = await screenCaptureService.getScreenInfo();
    
    res.json({
      success: true,
      screenInfo
    });
  } catch (error) {
    console.error('Failed to get screen info:', error);
    res.status(500).json({ 
      error: 'Failed to get screen info',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/system/connections
router.get('/connections', async (req, res) => {
  try {
    const socketHandler = req.app.get('socketHandler') as SocketHandler;
    
    const clientCount = socketHandler.getConnectedClients();
    const subscriptions = socketHandler.getClientSubscriptions();
    
    res.json({
      success: true,
      connections: {
        totalClients: clientCount,
        clientSubscriptions: Object.fromEntries(subscriptions)
      }
    });
  } catch (error) {
    console.error('Failed to get connections:', error);
    res.status(500).json({ 
      error: 'Failed to get connections',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/system/connections/:clientId/disconnect
router.post('/connections/:clientId/disconnect', async (req, res) => {
  try {
    const socketHandler = req.app.get('socketHandler') as SocketHandler;
    const { clientId } = req.params;
    
    const disconnected = socketHandler.disconnectClient(clientId);
    
    if (disconnected) {
      res.json({ 
        success: true, 
        message: 'Client disconnected successfully' 
      });
    } else {
      res.status(404).json({ 
        error: 'Client not found',
        message: 'Client not found or already disconnected'
      });
    }
  } catch (error) {
    console.error('Failed to disconnect client:', error);
    res.status(500).json({ 
      error: 'Failed to disconnect client',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/system/health
router.get('/health', async (req, res) => {
  try {
    const databaseService = req.app.get('databaseService') as DatabaseService;
    const screenCaptureService = req.app.get('screenCaptureService') as ScreenCaptureService;
    
    const health = {
      status: 'healthy',
      timestamp: new Date().toISOString(),
      uptime: process.uptime(),
      services: {
        database: databaseService.isConnected(),
        screenCapture: screenCaptureService.isActive(),
        memory: {
          used: process.memoryUsage().heapUsed,
          total: process.memoryUsage().heapTotal,
          external: process.memoryUsage().external,
          rss: process.memoryUsage().rss
        }
      }
    };

    res.json(health);
  } catch (error) {
    console.error('Health check failed:', error);
    res.status(500).json({ 
      status: 'unhealthy',
      error: 'Health check failed',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// POST /api/system/restart (placeholder - would require special handling)
router.post('/restart', async (req, res) => {
  try {
    console.log('🔄 System restart requested');
    
    // In a real implementation, this would:
    // 1. Stop all services gracefully
    // 2. Save current state
    // 3. Restart the process
    
    res.json({ 
      success: true,
      message: 'System restart initiated (placeholder - not implemented)',
      warning: 'Actual restart functionality requires process management setup'
    });
  } catch (error) {
    console.error('Failed to restart system:', error);
    res.status(500).json({ 
      error: 'Failed to restart system',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

// GET /api/system/config
router.get('/config', async (req, res) => {
  try {
    // Return safe configuration (no secrets)
    const config = {
      environment: process.env.NODE_ENV || 'development',
      version: process.env.npm_package_version || '1.0.0',
      features: {
        screenCapture: true,
        socialMedia: true,
        automation: true,
        realTimeMonitoring: true
      },
      limits: {
        maxConcurrentTasks: 10,
        maxScreenshotAge: 24 * 60 * 60 * 1000, // 24 hours
        maxLogEntries: 10000
      }
    };

    res.json(config);
  } catch (error) {
    console.error('Failed to get system config:', error);
    res.status(500).json({ 
      error: 'Failed to get system config',
      message: error instanceof Error ? error.message : 'Unknown error'
    });
  }
});

export default router;