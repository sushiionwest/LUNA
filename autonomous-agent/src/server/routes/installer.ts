import { Router } from 'express';
import { WindowInstallerService } from '../services/WindowInstallerService';
import { DatabaseService } from '../services/DatabaseService';
import { ScreenCaptureService } from '../services/ScreenCaptureService';

const router = Router();

let windowInstallerService: WindowInstallerService;

// Initialize service
export const initializeInstallerRoutes = (
  databaseService: DatabaseService,
  screenCaptureService: ScreenCaptureService
) => {
  windowInstallerService = new WindowInstallerService(databaseService, screenCaptureService);
  return router;
};

// Get available packages
router.get('/packages', async (req, res) => {
  try {
    const { category, platform } = req.query;
    const packages = await windowInstallerService.getAvailablePackages(
      category as string,
      platform as string
    );
    res.json({ success: true, packages });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Search packages
router.get('/packages/search', async (req, res) => {
  try {
    const { query } = req.query;
    if (!query) {
      return res.status(400).json({ success: false, error: 'Query parameter required' });
    }
    
    const packages = await windowInstallerService.searchPackages(query as string);
    res.json({ success: true, packages });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Add custom package
router.post('/packages', async (req, res) => {
  try {
    const packageData = req.body;
    await windowInstallerService.addCustomPackage(packageData);
    res.json({ success: true, message: 'Package added successfully' });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Remove package
router.delete('/packages/:packageId', async (req, res) => {
  try {
    const { packageId } = req.params;
    const removed = await windowInstallerService.removePackage(packageId);
    res.json({ success: removed, message: removed ? 'Package removed' : 'Package not found' });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Install package
router.post('/install', async (req, res) => {
  try {
    const { packageId, options = {} } = req.body;
    if (!packageId) {
      return res.status(400).json({ success: false, error: 'Package ID required' });
    }

    const task = await windowInstallerService.installPackage(packageId, options);
    res.json({ success: true, task });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Get installation status
router.get('/install/:taskId', async (req, res) => {
  try {
    const { taskId } = req.params;
    const task = windowInstallerService.getInstallationStatus(taskId);
    
    if (!task) {
      return res.status(404).json({ success: false, error: 'Installation task not found' });
    }
    
    res.json({ success: true, task });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Get all installations
router.get('/installations', async (req, res) => {
  try {
    const installations = windowInstallerService.getAllInstallations();
    res.json({ success: true, installations });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Cancel installation
router.post('/install/:taskId/cancel', async (req, res) => {
  try {
    const { taskId } = req.params;
    const cancelled = await windowInstallerService.cancelInstallation(taskId);
    res.json({ 
      success: cancelled, 
      message: cancelled ? 'Installation cancelled' : 'Cannot cancel installation' 
    });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Get installed applications
router.get('/installed', async (req, res) => {
  try {
    const applications = await windowInstallerService.getInstalledApplications();
    res.json({ success: true, applications });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Uninstall application
router.delete('/installed/:appId', async (req, res) => {
  try {
    const { appId } = req.params;
    const uninstalled = await windowInstallerService.uninstallApplication(appId);
    res.json({ 
      success: uninstalled, 
      message: uninstalled ? 'Application uninstalled' : 'Failed to uninstall application' 
    });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Window management
router.get('/windows', async (req, res) => {
  try {
    const windows = await windowInstallerService.getWindowList();
    res.json({ success: true, windows });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Manage window
router.post('/windows/:windowId/:action', async (req, res) => {
  try {
    const { windowId, action } = req.params;
    
    if (!['minimize', 'maximize', 'close', 'focus'].includes(action)) {
      return res.status(400).json({ success: false, error: 'Invalid action' });
    }
    
    const result = await windowInstallerService.manageWindow(
      windowId, 
      action as 'minimize' | 'maximize' | 'close' | 'focus'
    );
    
    res.json({ 
      success: result, 
      message: result ? `Window ${action} successful` : `Failed to ${action} window` 
    });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Bulk operations
router.post('/bulk/install', async (req, res) => {
  try {
    const { packageIds, options = {} } = req.body;
    if (!Array.isArray(packageIds)) {
      return res.status(400).json({ success: false, error: 'Package IDs array required' });
    }

    const tasks = [];
    for (const packageId of packageIds) {
      try {
        const task = await windowInstallerService.installPackage(packageId, options);
        tasks.push(task);
      } catch (error) {
        tasks.push({
          packageId,
          error: error instanceof Error ? error.message : 'Unknown error'
        });
      }
    }

    res.json({ success: true, tasks });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

router.post('/bulk/uninstall', async (req, res) => {
  try {
    const { appIds } = req.body;
    if (!Array.isArray(appIds)) {
      return res.status(400).json({ success: false, error: 'Application IDs array required' });
    }

    const results = [];
    for (const appId of appIds) {
      try {
        const result = await windowInstallerService.uninstallApplication(appId);
        results.push({ appId, success: result });
      } catch (error) {
        results.push({
          appId,
          success: false,
          error: error instanceof Error ? error.message : 'Unknown error'
        });
      }
    }

    res.json({ success: true, results });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// System integration
router.get('/system/info', async (req, res) => {
  try {
    const { execSync } = require('child_process');
    
    const systemInfo = {
      platform: process.platform,
      arch: process.arch,
      os: execSync('lsb_release -d -s 2>/dev/null || echo "Unknown"', { encoding: 'utf8' }).trim(),
      kernel: execSync('uname -r 2>/dev/null || echo "Unknown"', { encoding: 'utf8' }).trim(),
      memory: process.memoryUsage(),
      uptime: process.uptime(),
      nodeVersion: process.version,
      availableSpace: execSync('df -h / | tail -1 | awk "{print $4}"', { encoding: 'utf8' }).trim(),
      packageManagers: {
        apt: execSync('which apt 2>/dev/null || echo ""', { encoding: 'utf8' }).trim() !== '',
        snap: execSync('which snap 2>/dev/null || echo ""', { encoding: 'utf8' }).trim() !== '',
        flatpak: execSync('which flatpak 2>/dev/null || echo ""', { encoding: 'utf8' }).trim() !== '',
        rpm: execSync('which rpm 2>/dev/null || echo ""', { encoding: 'utf8' }).trim() !== ''
      }
    };

    res.json({ success: true, systemInfo });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

export default router;