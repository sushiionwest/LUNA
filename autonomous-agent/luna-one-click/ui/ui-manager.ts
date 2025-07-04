// ui-manager.ts - Seamless User Interface
import { BrowserWindow, screen, ipcMain } from 'electron';
import path from 'path';

export class LunaUI {
  private splashWindow: BrowserWindow | null = null;
  private mainWindow: BrowserWindow | null = null;
  private isMainReady = false;

  // Show splash screen immediately
  async showSplash(): Promise<void> {
    const { width, height } = screen.getPrimaryDisplay().workAreaSize;
    
    this.splashWindow = new BrowserWindow({
      width: 400,
      height: 300,
      x: Math.floor((width - 400) / 2),
      y: Math.floor((height - 300) / 2),
      frame: false,
      alwaysOnTop: true,
      transparent: true,
      resizable: false,
      webPreferences: {
        nodeIntegration: false,
        contextIsolation: true,
        preload: path.join(__dirname, 'preload.js')
      }
    });

    // Load splash content
    this.splashWindow.loadFile(path.join(__dirname, '../ui/splash.html'));
    
    this.splashWindow.show();
  }

  // Update splash screen with progress
  updateSplash(message: string, progress?: number): void {
    if (this.splashWindow) {
      this.splashWindow.webContents.send('splash:update', {
        message,
        progress
      });
    }
  }

  // Show main Luna interface
  async showMain(): Promise<void> {
    const { width, height } = screen.getPrimaryDisplay().workAreaSize;
    
    this.mainWindow = new BrowserWindow({
      width: Math.min(1400, width - 100),
      height: Math.min(900, height - 100),
      minWidth: 800,
      minHeight: 600,
      show: false, // Don't show until ready
      icon: this.getAppIcon(),
      webPreferences: {
        nodeIntegration: false,
        contextIsolation: true,
        enableRemoteModule: false,
        preload: path.join(__dirname, 'preload.js')
      },
      titleBarStyle: 'hiddenInset', // Modern look
      vibrancy: 'under-window', // macOS transparency
      frame: true
    });

    // Load Luna interface
    await this.loadLunaInterface();
    
    // Show main window when ready
    this.mainWindow.once('ready-to-show', () => {
      this.hideSplash();
      this.mainWindow?.show();
      this.mainWindow?.focus();
      this.isMainReady = true;
    });

    // Handle window closed
    this.mainWindow.on('closed', () => {
      this.mainWindow = null;
    });

    // Handle minimize to tray (optional)
    this.mainWindow.on('minimize', () => {
      // Could minimize to system tray here
    });
  }

  // Load Luna interface from VM
  private async loadLunaInterface(): Promise<void> {
    if (!this.mainWindow) return;

    // Load Luna web interface
    const lunaEndpoint = 'http://localhost:8080';
    
    try {
      await this.mainWindow.loadURL(lunaEndpoint);
    } catch (error) {
      // Fallback to local error page
      await this.mainWindow.loadFile(path.join(__dirname, '../ui/error.html'));
    }
  }

  // Hide splash screen
  private hideSplash(): void {
    if (this.splashWindow) {
      this.splashWindow.close();
      this.splashWindow = null;
    }
  }

  // Connect to Luna VM interface
  connectToLuna(endpoint: string): void {
    if (this.mainWindow) {
      this.mainWindow.loadURL(endpoint);
    }
  }

  // Show error dialog
  async showError(title: string, message: string, details?: string): Promise<void> {
    const { dialog } = require('electron');
    
    await dialog.showMessageBox(this.mainWindow || this.splashWindow!, {
      type: 'error',
      title,
      message,
      detail: details,
      buttons: ['OK', 'Get Help', 'Report Issue']
    });
  }

  // Show success message
  async showSuccess(message: string): Promise<void> {
    if (this.mainWindow) {
      this.mainWindow.webContents.send('luna:notification', {
        type: 'success',
        message
      });
    }
  }

  // Focus main window
  focus(): void {
    if (this.mainWindow) {
      if (this.mainWindow.isMinimized()) {
        this.mainWindow.restore();
      }
      this.mainWindow.focus();
    }
  }

  // Get application icon based on platform
  private getAppIcon(): string {
    const iconName = process.platform === 'win32' ? 'luna-icon.ico' :
                    process.platform === 'darwin' ? 'luna-icon.icns' :
                    'luna-icon.png';
    
    return path.join(__dirname, '../assets', iconName);
  }

  // Check if main window is ready
  isReady(): boolean {
    return this.isMainReady && this.mainWindow !== null;
  }

  // Get window bounds for saving/restoring
  getWindowBounds(): any {
    return this.mainWindow?.getBounds();
  }

  // Set window bounds
  setWindowBounds(bounds: any): void {
    if (this.mainWindow && bounds) {
      this.mainWindow.setBounds(bounds);
    }
  }
}