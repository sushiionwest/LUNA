// luna-app.ts - Perfect User Experience Main Application
import { app, BrowserWindow, ipcMain, dialog, Menu, Tray, nativeImage } from 'electron';
import { autoUpdater } from 'electron-updater';
import path from 'path';
import { SeamlessVMManager } from './seamless-vm-manager';

class LunaApp {
  private mainWindow: BrowserWindow | null = null;
  private splashWindow: BrowserWindow | null = null;
  private tray: Tray | null = null;
  private vmManager: SeamlessVMManager;
  private isQuitting = false;

  constructor() {
    this.vmManager = new SeamlessVMManager();
    this.setupAutoUpdater();
    this.setupEventHandlers();
  }

  // Main application initialization
  async initialize(): Promise<void> {
    try {
      // Show splash immediately for instant feedback
      await this.createSplashWindow();
      
      // Start Luna VM seamlessly in background
      const lunaEndpoint = await this.vmManager.startLunaSeamlessly(
        (message, progress) => this.updateSplash(message, progress)
      );
      
      // Create main window and connect to Luna
      await this.createMainWindow(lunaEndpoint);
      
      // Set up system tray for background operation
      this.setupSystemTray();
      
      // Hide splash, show main window
      this.transitionToMainWindow();
      
    } catch (error) {
      await this.handleStartupError(error);
    }
  }

  // Create splash screen for immediate user feedback
  private async createSplashWindow(): Promise<void> {
    this.splashWindow = new BrowserWindow({
      width: 400,
      height: 300,
      frame: false,
      alwaysOnTop: true,
      center: true,
      resizable: false,
      transparent: true,
      webPreferences: {
        nodeIntegration: false,
        contextIsolation: true,
        preload: path.join(__dirname, 'preload.js')
      }
    });

    await this.splashWindow.loadFile(path.join(__dirname, 'ui', 'splash.html'));
    this.splashWindow.show();
  }

  // Create main Luna interface window
  private async createMainWindow(lunaEndpoint: string): Promise<void> {
    this.mainWindow = new BrowserWindow({
      width: 1200,
      height: 800,
      minWidth: 800,
      minHeight: 600,
      show: false, // Don't show until ready
      icon: this.getAppIcon(),
      webPreferences: {
        nodeIntegration: false,
        contextIsolation: true,
        webSecurity: true,
        allowRunningInsecureContent: false,
        preload: path.join(__dirname, 'preload.js')
      },
      titleBarStyle: process.platform === 'darwin' ? 'hiddenInset' : 'default',
      backgroundColor: '#1a1a1a' // Dark theme
    });

    // Load Luna interface from VM
    await this.mainWindow.loadURL(lunaEndpoint);

    // Handle window events
    this.setupMainWindowEvents();
  }

  // Setup main window event handlers
  private setupMainWindowEvents(): void {
    if (!this.mainWindow) return;

    // Window ready to show
    this.mainWindow.once('ready-to-show', () => {
      if (this.mainWindow) {
        this.mainWindow.show();
        this.mainWindow.focus();
      }
    });

    // Handle window close (minimize to tray instead)
    this.mainWindow.on('close', (event) => {
      if (!this.isQuitting && process.platform !== 'darwin') {
        event.preventDefault();
        this.mainWindow?.hide();
        this.showTrayNotification('Luna is still running in the background');
      }
    });

    // Handle window closed
    this.mainWindow.on('closed', () => {
      this.mainWindow = null;
    });

    // Handle navigation (security)
    this.mainWindow.webContents.on('will-navigate', (event, navigationUrl) => {
      const parsedUrl = new URL(navigationUrl);
      
      // Only allow navigation to Luna endpoints
      if (parsedUrl.hostname !== 'localhost' && parsedUrl.hostname !== '127.0.0.1') {
        event.preventDefault();
      }
    });

    // Handle new window requests (security)
    this.mainWindow.webContents.setWindowOpenHandler(({ url }) => {
      // Open external links in default browser
      require('electron').shell.openExternal(url);
      return { action: 'deny' };
    });
  }

  // Setup system tray for background operation
  private setupSystemTray(): void {
    const trayIcon = nativeImage.createFromPath(this.getTrayIcon());
    this.tray = new Tray(trayIcon);

    const contextMenu = Menu.buildFromTemplate([
      {
        label: 'Open Luna',
        click: () => this.showMainWindow()
      },
      {
        label: 'About Luna',
        click: () => this.showAboutDialog()
      },
      { type: 'separator' },
      {
        label: 'Restart Luna',
        click: () => this.restartLuna()
      },
      { type: 'separator' },
      {
        label: 'Quit Luna',
        click: () => this.quitApplication()
      }
    ]);

    this.tray.setToolTip('Luna Agent - Automation Assistant');
    this.tray.setContextMenu(contextMenu);

    // Double-click to show window
    this.tray.on('double-click', () => {
      this.showMainWindow();
    });
  }

  // Update splash screen with progress
  private updateSplash(message: string, progress: number): void {
    if (this.splashWindow && !this.splashWindow.isDestroyed()) {
      this.splashWindow.webContents.send('update-progress', { message, progress });
    }
  }

  // Transition from splash to main window
  private transitionToMainWindow(): void {
    if (this.splashWindow) {
      this.splashWindow.close();
      this.splashWindow = null;
    }

    if (this.mainWindow) {
      this.mainWindow.show();
      this.mainWindow.focus();
    }
  }

  // Handle startup errors gracefully
  private async handleStartupError(error: any): Promise<void> {
    console.error('Luna startup error:', error);

    // Hide splash
    if (this.splashWindow) {
      this.splashWindow.close();
      this.splashWindow = null;
    }

    // Show user-friendly error dialog
    const choice = await dialog.showMessageBox({
      type: 'error',
      title: 'Luna needs help getting started',
      message: 'Luna encountered a problem while starting up.',
      detail: 'What would you like to do?',
      buttons: [
        'Try Again',
        'Reset Luna',
        'Get Help',
        'Close'
      ],
      defaultId: 0,
      cancelId: 3
    });

    switch (choice.response) {
      case 0: // Try Again
        await this.initialize();
        break;
      case 1: // Reset Luna
        await this.resetLuna();
        break;
      case 2: // Get Help
        require('electron').shell.openExternal('https://luna-agent.com/help');
        app.quit();
        break;
      case 3: // Close
        app.quit();
        break;
    }
  }

  // Setup application event handlers
  private setupEventHandlers(): void {
    // App ready
    app.whenReady().then(() => {
      this.initialize();
    });

    // Activate (macOS)
    app.on('activate', () => {
      if (BrowserWindow.getAllWindows().length === 0) {
        this.initialize();
      } else {
        this.showMainWindow();
      }
    });

    // Window all closed
    app.on('window-all-closed', () => {
      if (process.platform !== 'darwin') {
        this.quitApplication();
      }
    });

    // Before quit
    app.on('before-quit', () => {
      this.isQuitting = true;
    });

    // Will quit
    app.on('will-quit', async (event) => {
      if (!this.isQuitting) {
        event.preventDefault();
        await this.cleanupAndQuit();
      }
    });

    // Setup IPC handlers
    this.setupIPC();
  }

  // Setup IPC communication
  private setupIPC(): void {
    // Get Luna status
    ipcMain.handle('luna:get-status', async () => {
      return {
        ready: this.vmManager.isReady(),
        endpoint: this.vmManager.getEndpoint()
      };
    });

    // Restart Luna
    ipcMain.handle('luna:restart', async () => {
      try {
        this.showRestartProgress();
        const endpoint = await this.vmManager.restart();
        if (this.mainWindow) {
          await this.mainWindow.loadURL(endpoint);
        }
        return { success: true };
      } catch (error) {
        return { success: false, error: error.message };
      }
    });

    // Show notification
    ipcMain.handle('luna:show-notification', async (event, message, type = 'info') => {
      if (this.tray) {
        this.tray.displayBalloon({
          title: 'Luna Agent',
          content: message,
          icon: type === 'error' ? 'error' : 'info'
        });
      }
    });

    // Get app info
    ipcMain.handle('app:get-info', async () => {
      return {
        version: app.getVersion(),
        platform: process.platform,
        arch: process.arch
      };
    });
  }

  // Setup auto-updater
  private setupAutoUpdater(): void {
    // Configure auto-updater
    autoUpdater.checkForUpdatesAndNotify();

    autoUpdater.on('update-available', () => {
      this.showUpdateNotification('A new version of Luna is available and will be downloaded in the background.');
    });

    autoUpdater.on('update-downloaded', () => {
      this.showUpdateDialog();
    });
  }

  // Show main window
  private showMainWindow(): void {
    if (this.mainWindow) {
      if (this.mainWindow.isMinimized()) {
        this.mainWindow.restore();
      }
      this.mainWindow.show();
      this.mainWindow.focus();
    }
  }

  // Show about dialog
  private showAboutDialog(): void {
    dialog.showMessageBox({
      type: 'info',
      title: 'About Luna Agent',
      message: 'Luna Agent',
      detail: `Version: ${app.getVersion()}\nPlatform: ${process.platform}\n\nLuna is your intelligent automation assistant.`,
      buttons: ['OK']
    });
  }

  // Restart Luna
  private async restartLuna(): Promise<void> {
    try {
      if (this.mainWindow) {
        this.mainWindow.webContents.send('show-loading', 'Restarting Luna...');
      }

      const endpoint = await this.vmManager.restart();

      if (this.mainWindow) {
        await this.mainWindow.loadURL(endpoint);
      }

      this.showTrayNotification('Luna has been restarted successfully');
    } catch (error) {
      this.showTrayNotification('Failed to restart Luna', 'error');
    }
  }

  // Reset Luna to defaults
  private async resetLuna(): Promise<void> {
    const choice = await dialog.showMessageBox({
      type: 'warning',
      title: 'Reset Luna',
      message: 'This will reset Luna to its default state.',
      detail: 'All settings and data will be lost. Are you sure?',
      buttons: ['Reset', 'Cancel'],
      defaultId: 1
    });

    if (choice.response === 0) {
      // Perform reset
      await this.performLunaReset();
    }
  }

  // Quit application
  private quitApplication(): void {
    this.isQuitting = true;
    app.quit();
  }

  // Show restart progress
  private showRestartProgress(): void {
    if (this.mainWindow) {
      this.mainWindow.webContents.send('show-loading', 'Restarting Luna...');
    }
  }

  // Show tray notification
  private showTrayNotification(message: string, type: 'info' | 'error' = 'info'): void {
    if (this.tray) {
      this.tray.displayBalloon({
        title: 'Luna Agent',
        content: message,
        icon: type
      });
    }
  }

  // Show update notification
  private showUpdateNotification(message: string): void {
    this.showTrayNotification(message);
  }

  // Show update dialog
  private async showUpdateDialog(): Promise<void> {
    const choice = await dialog.showMessageBox({
      type: 'info',
      title: 'Update Ready',
      message: 'A new version of Luna has been downloaded.',
      detail: 'Luna will restart to apply the update.',
      buttons: ['Restart Now', 'Later'],
      defaultId: 0
    });

    if (choice.response === 0) {
      autoUpdater.quitAndInstall();
    }
  }

  // Get application icon
  private getAppIcon(): string {
    const iconName = process.platform === 'win32' ? 'luna-icon.ico' :
                    process.platform === 'darwin' ? 'luna-icon.icns' :
                    'luna-icon.png';
    return path.join(__dirname, 'assets', iconName);
  }

  // Get tray icon
  private getTrayIcon(): string {
    const iconName = process.platform === 'win32' ? 'luna-tray.ico' :
                    process.platform === 'darwin' ? 'luna-tray.png' :
                    'luna-tray.png';
    return path.join(__dirname, 'assets', iconName);
  }

  // Cleanup and quit
  private async cleanupAndQuit(): Promise<void> {
    try {
      await this.vmManager.shutdown();
    } catch (error) {
      console.error('Error during cleanup:', error);
    }
    app.quit();
  }

  // Perform Luna reset
  private async performLunaReset(): Promise<void> {
    // Implementation for resetting Luna to defaults
    try {
      await this.vmManager.shutdown();
      // Clear user data, reset VM, etc.
      app.relaunch();
      app.exit();
    } catch (error) {
      console.error('Error during reset:', error);
    }
  }
}

// Create and start Luna App
const lunaApp = new LunaApp();

// Handle second instance (prevent multiple instances)
const gotTheLock = app.requestSingleInstanceLock();

if (!gotTheLock) {
  app.quit();
} else {
  app.on('second-instance', () => {
    // Someone tried to run a second instance, focus our window instead
    if (lunaApp['mainWindow']) {
      lunaApp['showMainWindow']();
    }
  });
}