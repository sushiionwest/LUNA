import { app, autoUpdater, dialog, BrowserWindow } from 'electron';
import { EventEmitter } from 'events';
import path from 'path';
import fs from 'fs/promises';
import crypto from 'crypto';
import axios from 'axios';

export interface UpdateInfo {
  version: string;
  releaseDate: string;
  downloadUrl: string;
  sha256: string;
  releaseNotes?: string;
  critical?: boolean;
  minimumVersion?: string;
}

export interface UpdateProgress {
  bytesPerSecond: number;
  percent: number;
  transferred: number;
  total: number;
}

export interface UpdateCheckResult {
  updateAvailable: boolean;
  currentVersion: string;
  latestVersion?: string;
  updateInfo?: UpdateInfo;
  error?: string;
}

export class AutoUpdateService extends EventEmitter {
  private updateCheckInterval: NodeJS.Timeout | null = null;
  private isCheckingForUpdates = false;
  private isUpdating = false;
  private lastCheckTime: Date | null = null;
  
  // Configuration
  private readonly UPDATE_CHECK_INTERVAL = 4 * 60 * 60 * 1000; // 4 hours
  private readonly UPDATE_SERVER_URL = process.env.UPDATE_SERVER_URL || 'https://api.github.com/repos/your-username/luna/releases/latest';
  private readonly AUTO_CHECK_ENABLED = process.env.AUTO_UPDATE_CHECK !== 'false';
  private readonly AUTO_DOWNLOAD_ENABLED = process.env.AUTO_UPDATE_DOWNLOAD !== 'false';
  private readonly SILENT_UPDATES = process.env.SILENT_UPDATES === 'true';

  constructor() {
    super();
    this.setupAutoUpdater();
    this.setupEventHandlers();
    
    if (this.AUTO_CHECK_ENABLED) {
      this.startPeriodicChecks();
    }
  }

  private setupAutoUpdater(): void {
    // Configure autoUpdater based on platform
    if (process.platform === 'win32') {
      // Squirrel.Windows configuration
      const updateUrl = process.env.SQUIRREL_UPDATE_URL || `https://github.com/your-username/luna/releases/latest/download`;
      autoUpdater.setFeedURL({ url: updateUrl });
    } else if (process.platform === 'darwin') {
      // macOS configuration (if needed in the future)
      console.log('macOS auto-update not implemented yet');
    } else if (process.platform === 'linux') {
      // Linux configuration (AppImage or custom updater)
      console.log('Linux auto-update not implemented yet');
    }

    // Configure update server
    autoUpdater.on('error', (error) => {
      console.error('AutoUpdater error:', error);
      this.emit('update-error', error);
    });

    autoUpdater.on('checking-for-update', () => {
      console.log('Checking for updates...');
      this.emit('checking-for-update');
    });

    autoUpdater.on('update-available', (info) => {
      console.log('Update available:', info);
      this.emit('update-available', info);
      
      if (this.AUTO_DOWNLOAD_ENABLED) {
        this.downloadUpdate();
      }
    });

    autoUpdater.on('update-not-available', (info) => {
      console.log('Update not available:', info);
      this.emit('update-not-available', info);
    });

    autoUpdater.on('download-progress', (progress: UpdateProgress) => {
      console.log(`Download progress: ${progress.percent}%`);
      this.emit('download-progress', progress);
    });

    autoUpdater.on('update-downloaded', (info) => {
      console.log('Update downloaded:', info);
      this.emit('update-downloaded', info);
      
      if (!this.SILENT_UPDATES) {
        this.showUpdateReadyDialog();
      } else {
        // Install immediately in silent mode
        this.installUpdate();
      }
    });
  }

  private setupEventHandlers(): void {
    // Handle Squirrel.Windows events
    if (process.platform === 'win32') {
      this.handleSquirrelEvent();
    }
  }

  private handleSquirrelEvent(): boolean {
    if (process.argv.length === 1) {
      return false;
    }

    const ChildProcess = require('child_process');
    const path = require('path');

    const appFolder = path.resolve(process.execPath, '..');
    const rootAtomFolder = path.resolve(appFolder, '..');
    const updateDotExe = path.resolve(path.join(rootAtomFolder, 'Update.exe'));
    const exeName = path.basename(process.execPath);

    const spawn = function(command: string, args: string[]) {
      let spawnedProcess: any;
      try {
        spawnedProcess = ChildProcess.spawn(command, args, { detached: true });
      } catch (error) {
        console.error('Squirrel spawn error:', error);
      }
      return spawnedProcess;
    };

    const spawnUpdate = function(args: string[]) {
      return spawn(updateDotExe, args);
    };

    const squirrelEvent = process.argv[1];
    switch (squirrelEvent) {
      case '--squirrel-install':
      case '--squirrel-updated':
        // Install desktop and start menu shortcuts
        spawnUpdate(['--createShortcut', exeName]);
        setTimeout(app.quit, 1000);
        return true;

      case '--squirrel-uninstall':
        // Remove desktop and start menu shortcuts
        spawnUpdate(['--removeShortcut', exeName]);
        setTimeout(app.quit, 1000);
        return true;

      case '--squirrel-obsolete':
        // This is called on the outgoing version of your app before
        // we update to the new version - it's the opposite of
        // --squirrel-updated
        app.quit();
        return true;
    }

    return false;
  }

  public async checkForUpdates(force: boolean = false): Promise<UpdateCheckResult> {
    if (this.isCheckingForUpdates && !force) {
      return {
        updateAvailable: false,
        currentVersion: app.getVersion(),
        error: 'Update check already in progress'
      };
    }

    this.isCheckingForUpdates = true;
    this.lastCheckTime = new Date();

    try {
      const currentVersion = app.getVersion();
      console.log(`Current version: ${currentVersion}`);

      // Check GitHub releases API
      const response = await axios.get(this.UPDATE_SERVER_URL, {
        timeout: 10000,
        headers: {
          'User-Agent': `Luna-Agent/${currentVersion}`,
          'Accept': 'application/vnd.github.v3+json'
        }
      });

      const releaseData = response.data;
      const latestVersion = releaseData.tag_name.replace(/^v/, '');
      
      const updateAvailable = this.isVersionNewer(latestVersion, currentVersion);
      
      if (updateAvailable) {
        // Find Windows installer asset
        const windowsAsset = releaseData.assets.find((asset: any) => 
          asset.name.includes('setup') && asset.name.includes('.exe')
        );

        if (!windowsAsset) {
          throw new Error('No Windows installer found in release');
        }

        const updateInfo: UpdateInfo = {
          version: latestVersion,
          releaseDate: releaseData.published_at,
          downloadUrl: windowsAsset.browser_download_url,
          sha256: '', // Would be populated from release description or separate file
          releaseNotes: releaseData.body,
          critical: releaseData.name.toLowerCase().includes('critical') || 
                   releaseData.body.toLowerCase().includes('security')
        };

        // Try to get SHA256 from release assets
        const checksumAsset = releaseData.assets.find((asset: any) => 
          asset.name === 'SHA256SUMS.txt'
        );
        
        if (checksumAsset) {
          try {
            const checksumResponse = await axios.get(checksumAsset.browser_download_url);
            const checksums = checksumResponse.data;
            const match = checksums.match(new RegExp(`([a-fA-F0-9]{64})\\s+${windowsAsset.name}`));
            if (match) {
              updateInfo.sha256 = match[1];
            }
          } catch (error) {
            console.warn('Could not fetch checksum:', error);
          }
        }

        this.emit('update-available', updateInfo);
        return {
          updateAvailable: true,
          currentVersion,
          latestVersion,
          updateInfo
        };
      } else {
        this.emit('update-not-available', { currentVersion, latestVersion });
        return {
          updateAvailable: false,
          currentVersion,
          latestVersion
        };
      }

    } catch (error) {
      console.error('Update check failed:', error);
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      this.emit('update-error', errorMessage);
      
      return {
        updateAvailable: false,
        currentVersion: app.getVersion(),
        error: errorMessage
      };
    } finally {
      this.isCheckingForUpdates = false;
    }
  }

  public async downloadUpdate(): Promise<void> {
    if (this.isUpdating) {
      throw new Error('Update already in progress');
    }

    this.isUpdating = true;
    
    try {
      console.log('Starting update download...');
      autoUpdater.downloadUpdate();
    } catch (error) {
      this.isUpdating = false;
      throw error;
    }
  }

  public async installUpdate(): Promise<void> {
    console.log('Installing update...');
    autoUpdater.quitAndInstall();
  }

  private async showUpdateReadyDialog(): Promise<void> {
    const mainWindow = BrowserWindow.getFocusedWindow() || BrowserWindow.getAllWindows()[0];
    
    if (!mainWindow) {
      // No window available, install silently
      this.installUpdate();
      return;
    }

    const result = await dialog.showMessageBox(mainWindow, {
      type: 'info',
      title: 'Update Ready',
      message: 'A new version of Luna Agent has been downloaded and is ready to install.',
      detail: 'The application will restart to complete the update.',
      buttons: ['Install Now', 'Install Later'],
      defaultId: 0,
      cancelId: 1
    });

    if (result.response === 0) {
      this.installUpdate();
    }
  }

  private isVersionNewer(newVersion: string, currentVersion: string): boolean {
    const parseVersion = (version: string) => {
      return version.split('.').map(num => parseInt(num, 10));
    };

    const newParts = parseVersion(newVersion);
    const currentParts = parseVersion(currentVersion);

    for (let i = 0; i < Math.max(newParts.length, currentParts.length); i++) {
      const newPart = newParts[i] || 0;
      const currentPart = currentParts[i] || 0;

      if (newPart > currentPart) {
        return true;
      } else if (newPart < currentPart) {
        return false;
      }
    }

    return false;
  }

  private startPeriodicChecks(): void {
    // Check immediately on startup (after a delay)
    setTimeout(() => {
      this.checkForUpdates();
    }, 30000); // 30 seconds after startup

    // Set up periodic checks
    this.updateCheckInterval = setInterval(() => {
      this.checkForUpdates();
    }, this.UPDATE_CHECK_INTERVAL);
  }

  public stopPeriodicChecks(): void {
    if (this.updateCheckInterval) {
      clearInterval(this.updateCheckInterval);
      this.updateCheckInterval = null;
    }
  }

  public getLastCheckTime(): Date | null {
    return this.lastCheckTime;
  }

  public isUpdateInProgress(): boolean {
    return this.isUpdating;
  }

  public async verifyDownloadIntegrity(filePath: string, expectedSha256: string): Promise<boolean> {
    try {
      const fileBuffer = await fs.readFile(filePath);
      const hash = crypto.createHash('sha256');
      hash.update(fileBuffer);
      const actualSha256 = hash.digest('hex');
      
      return actualSha256.toLowerCase() === expectedSha256.toLowerCase();
    } catch (error) {
      console.error('Error verifying download integrity:', error);
      return false;
    }
  }

  public async getUpdateSettings(): Promise<{
    autoCheck: boolean;
    autoDownload: boolean;
    silentUpdates: boolean;
    checkInterval: number;
    lastCheck: Date | null;
  }> {
    return {
      autoCheck: this.AUTO_CHECK_ENABLED,
      autoDownload: this.AUTO_DOWNLOAD_ENABLED,
      silentUpdates: this.SILENT_UPDATES,
      checkInterval: this.UPDATE_CHECK_INTERVAL,
      lastCheck: this.lastCheckTime
    };
  }

  public destroy(): void {
    this.stopPeriodicChecks();
    this.removeAllListeners();
  }
}