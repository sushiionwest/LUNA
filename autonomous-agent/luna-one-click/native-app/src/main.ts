// main.ts - Luna Agent Main Process
import { app, BrowserWindow, ipcMain, dialog } from 'electron';
import { autoUpdater } from 'electron-updater';
import path from 'path';
import { LunaVMEngine } from './vm-engine';
import { LunaUI } from './ui-manager';
import { ErrorRecovery } from './error-recovery';

class LunaAgent {
  private vmEngine: LunaVMEngine;
  private ui: LunaUI;
  private recovery: ErrorRecovery;
  private isReady = false;

  constructor() {
    this.vmEngine = new LunaVMEngine();
    this.ui = new LunaUI();
    this.recovery = new ErrorRecovery(this.vmEngine);
  }

  async initialize() {
    try {
      // Show splash screen immediately
      await this.ui.showSplash();
      
      // Initialize VM in background
      await this.initializeVM();
      
      // Show main interface
      await this.ui.showMain();
      
      this.isReady = true;
      console.log('🌙 Luna Agent ready');
      
    } catch (error) {
      console.error('Failed to initialize Luna:', error);
      await this.recovery.handleInitializationError(error);
    }
  }

  private async initializeVM() {
    // Update splash with progress
    this.ui.updateSplash('Initializing Luna environment...');
    
    // Check system requirements
    if (!await this.vmEngine.checkSystemRequirements()) {
      throw new Error('Insufficient system resources');
    }
    
    this.ui.updateSplash('Setting up automation tools...');
    
    // Ensure VM is ready
    await this.vmEngine.ensureReady();
    
    this.ui.updateSplash('Starting Luna services...');
    
    // Wait for Luna to be responsive
    await this.vmEngine.waitForLunaReady();
  }

  // Handle app events
  setupEventHandlers() {
    // App ready
    app.whenReady().then(() => {
      this.initialize();
    });

    // All windows closed
    app.on('window-all-closed', () => {
      if (process.platform !== 'darwin') {
        this.shutdown();
      }
    });

    // App reactivated (macOS)
    app.on('activate', () => {
      if (BrowserWindow.getAllWindows().length === 0) {
        this.ui.showMain();
      }
    });

    // Before quit
    app.on('before-quit', () => {
      this.shutdown();
    });

    // IPC handlers
    this.setupIPC();
  }

  private setupIPC() {
    // Get Luna status
    ipcMain.handle('luna:getStatus', async () => {
      return {
        ready: this.isReady,
        vmRunning: await this.vmEngine.isRunning(),
        endpoint: this.vmEngine.getEndpoint()
      };
    });

    // Restart Luna
    ipcMain.handle('luna:restart', async () => {
      try {
        await this.vmEngine.restart();
        return { success: true };
      } catch (error) {
        return { success: false, error: error.message };
      }
    });

    // Get system info
    ipcMain.handle('system:getInfo', async () => {
      return await this.vmEngine.getSystemInfo();
    });

    // Handle errors
    ipcMain.handle('luna:reportError', async (event, error) => {
      await this.recovery.handleUserReportedError(error);
    });
  }

  private async shutdown() {
    console.log('🌙 Shutting down Luna Agent...');
    
    try {
      // Graceful VM shutdown
      await this.vmEngine.shutdown();
    } catch (error) {
      console.error('Error during shutdown:', error);
    }
    
    app.quit();
  }
}

// Auto-updater setup
autoUpdater.checkForUpdatesAndNotify();

// Create and start Luna Agent
const lunaAgent = new LunaAgent();
lunaAgent.setupEventHandlers();