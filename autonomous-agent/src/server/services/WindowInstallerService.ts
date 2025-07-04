import { promises as fs } from 'fs';
import * as path from 'path';
import { spawn, exec } from 'child_process';
import { promisify } from 'util';
import { DatabaseService } from './DatabaseService';
import { ScreenCaptureService } from './ScreenCaptureService';

const execAsync = promisify(exec);

export interface InstallationPackage {
  id: string;
  name: string;
  version: string;
  description: string;
  downloadUrl: string;
  checksum?: string;
  installPath?: string;
  requirements?: string[];
  category: 'application' | 'system' | 'development' | 'media' | 'game' | 'utility';
  platform: 'linux' | 'windows' | 'macos' | 'any';
  architecture: 'x64' | 'arm64' | 'x86' | 'any';
  fileSize: number;
  installationType: 'deb' | 'rpm' | 'snap' | 'flatpak' | 'appimage' | 'tarball' | 'script';
}

export interface InstallationTask {
  id: string;
  packageId: string;
  status: 'pending' | 'downloading' | 'installing' | 'completed' | 'failed' | 'cancelled';
  progress: number;
  startTime: Date;
  endTime?: Date;
  errorMessage?: string;
  installPath?: string;
  logPath?: string;
}

export interface InstalledApplication {
  id: string;
  packageId: string;
  name: string;
  version: string;
  installPath: string;
  installDate: Date;
  size: number;
  executable?: string;
  uninstallCommand?: string;
  autoStart: boolean;
  dependencies?: string[];
}

export interface WindowManager {
  id: string;
  applicationId: string;
  windowTitle: string;
  processId: number;
  windowId: string;
  position: { x: number; y: number };
  size: { width: number; height: number };
  state: 'normal' | 'minimized' | 'maximized' | 'fullscreen';
  isActive: boolean;
  workspace?: string;
}

export interface InstallationOptions {
  autoStart?: boolean;
  createDesktopShortcut?: boolean;
  addToPath?: boolean;
  customInstallPath?: string;
  installDependencies?: boolean;
  verifyChecksum?: boolean;
  backupBeforeInstall?: boolean;
  silentInstall?: boolean;
}

export class WindowInstallerService {
  private downloadDir: string;
  private installDir: string;
  private logDir: string;
  private installations = new Map<string, InstallationTask>();
  private installedApps = new Map<string, InstalledApplication>();
  private managedWindows = new Map<string, WindowManager>();
  private packageCatalog = new Map<string, InstallationPackage>();

  constructor(
    private databaseService: DatabaseService,
    private screenCapture: ScreenCaptureService
  ) {
    this.downloadDir = '/tmp/autonomous-agent-downloads';
    this.installDir = '/opt/autonomous-agent-apps';
    this.logDir = '/var/log/autonomous-agent';
    this.initializeDirectories();
    this.loadPackageCatalog();
    this.loadInstalledApplications();
  }

  private async initializeDirectories(): Promise<void> {
    try {
      await fs.mkdir(this.downloadDir, { recursive: true });
      await fs.mkdir(this.installDir, { recursive: true });
      await fs.mkdir(this.logDir, { recursive: true });
    } catch (error) {
      console.error('Failed to initialize directories:', error);
    }
  }

  private async loadPackageCatalog(): Promise<void> {
    // Default package catalog with popular Linux applications
    const defaultPackages: InstallationPackage[] = [
      {
        id: 'vscode',
        name: 'Visual Studio Code',
        version: 'latest',
        description: 'Lightweight but powerful source code editor',
        downloadUrl: 'https://update.code.visualstudio.com/latest/linux-deb-x64/stable',
        category: 'development',
        platform: 'linux',
        architecture: 'x64',
        fileSize: 80 * 1024 * 1024,
        installationType: 'deb'
      },
      {
        id: 'firefox',
        name: 'Firefox Browser',
        version: 'latest',
        description: 'Free and open-source web browser',
        downloadUrl: 'https://download.mozilla.org/?product=firefox-latest&os=linux64&lang=en-US',
        category: 'application',
        platform: 'linux',
        architecture: 'x64',
        fileSize: 70 * 1024 * 1024,
        installationType: 'tarball'
      },
      {
        id: 'discord',
        name: 'Discord',
        version: 'latest',
        description: 'Voice and text chat for gamers',
        downloadUrl: 'https://discord.com/api/download?platform=linux&format=deb',
        category: 'application',
        platform: 'linux',
        architecture: 'x64',
        fileSize: 60 * 1024 * 1024,
        installationType: 'deb'
      },
      {
        id: 'gimp',
        name: 'GIMP',
        version: 'latest',
        description: 'GNU Image Manipulation Program',
        downloadUrl: 'snap://gimp',
        category: 'media',
        platform: 'linux',
        architecture: 'any',
        fileSize: 200 * 1024 * 1024,
        installationType: 'snap'
      },
      {
        id: 'vlc',
        name: 'VLC Media Player',
        version: 'latest',
        description: 'Free and open-source multimedia player',
        downloadUrl: 'snap://vlc',
        category: 'media',
        platform: 'linux',
        architecture: 'any',
        fileSize: 100 * 1024 * 1024,
        installationType: 'snap'
      }
    ];

    defaultPackages.forEach(pkg => {
      this.packageCatalog.set(pkg.id, pkg);
    });
  }

  private async loadInstalledApplications(): Promise<void> {
    try {
      // Query system for installed applications
      const { stdout: debApps } = await execAsync('dpkg-query -W -f="${Package}\\t${Version}\\t${Installed-Size}\\n" 2>/dev/null || true');
      const { stdout: snapApps } = await execAsync('snap list --unicode=never 2>/dev/null || true');
      
      // Parse and store installed applications
      this.parseInstalledApps(debApps, 'deb');
      this.parseInstalledApps(snapApps, 'snap');
    } catch (error) {
      console.error('Failed to load installed applications:', error);
    }
  }

  private parseInstalledApps(output: string, type: string): void {
    const lines = output.split('\n').filter(line => line.trim());
    
    lines.forEach(line => {
      const parts = line.split('\t');
      if (parts.length >= 2) {
        const [name, version, size] = parts;
        const app: InstalledApplication = {
          id: `${type}-${name}`,
          packageId: name,
          name,
          version,
          installPath: type === 'snap' ? `/snap/${name}` : '/usr',
          installDate: new Date(),
          size: parseInt(size) || 0,
          autoStart: false
        };
        this.installedApps.set(app.id, app);
      }
    });
  }

  public async getAvailablePackages(category?: string, platform?: string): Promise<InstallationPackage[]> {
    let packages = Array.from(this.packageCatalog.values());
    
    if (category) {
      packages = packages.filter(pkg => pkg.category === category);
    }
    
    if (platform) {
      packages = packages.filter(pkg => pkg.platform === platform || pkg.platform === 'any');
    }
    
    return packages;
  }

  public async getInstalledApplications(): Promise<InstalledApplication[]> {
    return Array.from(this.installedApps.values());
  }

  public async installPackage(
    packageId: string, 
    options: InstallationOptions = {}
  ): Promise<InstallationTask> {
    const pkg = this.packageCatalog.get(packageId);
    if (!pkg) {
      throw new Error(`Package ${packageId} not found`);
    }

    const taskId = `install-${packageId}-${Date.now()}`;
    const task: InstallationTask = {
      id: taskId,
      packageId,
      status: 'pending',
      progress: 0,
      startTime: new Date(),
      logPath: path.join(this.logDir, `${taskId}.log`)
    };

    this.installations.set(taskId, task);

    // Start installation in background
    this.performInstallation(task, pkg, options).catch(error => {
      task.status = 'failed';
      task.errorMessage = error.message;
      task.endTime = new Date();
    });

    return task;
  }

  private async performInstallation(
    task: InstallationTask,
    pkg: InstallationPackage,
    options: InstallationOptions
  ): Promise<void> {
    try {
      task.status = 'downloading';
      this.updateTaskProgress(task, 10);

      // Download package if needed
      let filePath: string;
      if (pkg.downloadUrl.startsWith('snap://')) {
        filePath = pkg.downloadUrl;
      } else {
        filePath = await this.downloadPackage(pkg, task);
      }

      task.status = 'installing';
      this.updateTaskProgress(task, 50);

      // Install package based on type
      await this.installByType(pkg, filePath, options, task);

      // Post-installation setup
      await this.postInstallSetup(pkg, options, task);

      task.status = 'completed';
      task.progress = 100;
      task.endTime = new Date();

      // Register installed application
      const installedApp: InstalledApplication = {
        id: `installed-${pkg.id}-${Date.now()}`,
        packageId: pkg.id,
        name: pkg.name,
        version: pkg.version,
        installPath: options.customInstallPath || pkg.installPath || this.installDir,
        installDate: new Date(),
        size: pkg.fileSize,
        autoStart: options.autoStart || false
      };

      this.installedApps.set(installedApp.id, installedApp);

    } catch (error) {
      task.status = 'failed';
      task.errorMessage = error instanceof Error ? error.message : 'Unknown error';
      task.endTime = new Date();
      throw error;
    }
  }

  private async downloadPackage(pkg: InstallationPackage, task: InstallationTask): Promise<string> {
    const fileName = `${pkg.id}-${pkg.version}.${this.getFileExtension(pkg.installationType)}`;
    const filePath = path.join(this.downloadDir, fileName);

    // Simulate download progress (in real implementation, use actual download with progress)
    for (let i = 10; i <= 40; i += 5) {
      await new Promise(resolve => setTimeout(resolve, 100));
      this.updateTaskProgress(task, i);
    }

    // For demo purposes, create a placeholder file
    await fs.writeFile(filePath, 'placeholder package data');
    
    return filePath;
  }

  private async installByType(
    pkg: InstallationPackage,
    filePath: string,
    options: InstallationOptions,
    task: InstallationTask
  ): Promise<void> {
    const silentFlag = options.silentInstall ? ' -y' : '';
    
    switch (pkg.installationType) {
      case 'deb':
        await this.executeCommand(`sudo dpkg -i ${filePath}${silentFlag}`, task);
        break;
      
      case 'rpm':
        await this.executeCommand(`sudo rpm -i ${filePath}${silentFlag}`, task);
        break;
      
      case 'snap':
        const snapName = filePath.replace('snap://', '');
        await this.executeCommand(`sudo snap install ${snapName}`, task);
        break;
      
      case 'flatpak':
        await this.executeCommand(`flatpak install${silentFlag} ${filePath}`, task);
        break;
      
      case 'appimage':
        const appImagePath = path.join(this.installDir, `${pkg.name}.AppImage`);
        await fs.copyFile(filePath, appImagePath);
        await this.executeCommand(`chmod +x ${appImagePath}`, task);
        break;
      
      case 'tarball':
        const extractPath = path.join(this.installDir, pkg.id);
        await fs.mkdir(extractPath, { recursive: true });
        await this.executeCommand(`tar -xzf ${filePath} -C ${extractPath}`, task);
        break;
      
      default:
        throw new Error(`Unsupported installation type: ${pkg.installationType}`);
    }
  }

  private async postInstallSetup(
    pkg: InstallationPackage,
    options: InstallationOptions,
    task: InstallationTask
  ): Promise<void> {
    this.updateTaskProgress(task, 80);

    if (options.createDesktopShortcut) {
      await this.createDesktopShortcut(pkg);
    }

    if (options.addToPath) {
      await this.addToPath(pkg);
    }

    if (options.autoStart) {
      await this.setupAutoStart(pkg);
    }

    this.updateTaskProgress(task, 90);
  }

  private async executeCommand(command: string, task: InstallationTask): Promise<string> {
    return new Promise((resolve, reject) => {
      const process = spawn('bash', ['-c', command]);
      let output = '';

      process.stdout.on('data', (data) => {
        output += data.toString();
      });

      process.stderr.on('data', (data) => {
        output += data.toString();
      });

      process.on('close', (code) => {
        if (code === 0) {
          resolve(output);
        } else {
          reject(new Error(`Command failed with code ${code}: ${output}`));
        }
      });
    });
  }

  private updateTaskProgress(task: InstallationTask, progress: number): void {
    task.progress = Math.min(progress, 100);
    this.installations.set(task.id, task);
  }

  private getFileExtension(type: string): string {
    const extensions: Record<string, string> = {
      'deb': 'deb',
      'rpm': 'rpm',
      'appimage': 'AppImage',
      'tarball': 'tar.gz',
      'script': 'sh'
    };
    return extensions[type] || 'bin';
  }

  private async createDesktopShortcut(pkg: InstallationPackage): Promise<void> {
    const desktopPath = `/home/${process.env.USER}/Desktop/${pkg.name}.desktop`;
    const shortcutContent = `[Desktop Entry]
Version=1.0
Type=Application
Name=${pkg.name}
Comment=${pkg.description}
Exec=${pkg.executable || pkg.name}
Icon=${pkg.name.toLowerCase()}
Terminal=false
Categories=${pkg.category};`;

    await fs.writeFile(desktopPath, shortcutContent);
    await this.executeCommand(`chmod +x ${desktopPath}`, { id: 'temp' } as InstallationTask);
  }

  private async addToPath(pkg: InstallationPackage): Promise<void> {
    // Add to PATH (simplified implementation)
    const pathEntry = `export PATH=$PATH:${pkg.installPath}/bin`;
    const bashrcPath = `/home/${process.env.USER}/.bashrc`;
    
    try {
      const bashrc = await fs.readFile(bashrcPath, 'utf8');
      if (!bashrc.includes(pathEntry)) {
        await fs.appendFile(bashrcPath, `\n${pathEntry}\n`);
      }
    } catch (error) {
      console.error('Failed to add to PATH:', error);
    }
  }

  private async setupAutoStart(pkg: InstallationPackage): Promise<void> {
    const autostartDir = `/home/${process.env.USER}/.config/autostart`;
    await fs.mkdir(autostartDir, { recursive: true });
    
    const autostartFile = `${autostartDir}/${pkg.id}.desktop`;
    const autostartContent = `[Desktop Entry]
Type=Application
Name=${pkg.name}
Exec=${pkg.executable || pkg.name}
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true`;

    await fs.writeFile(autostartFile, autostartContent);
  }

  public async uninstallApplication(appId: string): Promise<boolean> {
    const app = this.installedApps.get(appId);
    if (!app) {
      throw new Error(`Application ${appId} not found`);
    }

    try {
      if (app.uninstallCommand) {
        await this.executeCommand(app.uninstallCommand, { id: 'temp' } as InstallationTask);
      } else {
        // Default uninstall commands based on package type
        if (appId.startsWith('deb-')) {
          await this.executeCommand(`sudo apt remove -y ${app.packageId}`, { id: 'temp' } as InstallationTask);
        } else if (appId.startsWith('snap-')) {
          await this.executeCommand(`sudo snap remove ${app.packageId}`, { id: 'temp' } as InstallationTask);
        }
      }

      this.installedApps.delete(appId);
      return true;
    } catch (error) {
      console.error(`Failed to uninstall ${app.name}:`, error);
      return false;
    }
  }

  public async getWindowList(): Promise<WindowManager[]> {
    try {
      const { stdout } = await execAsync('wmctrl -l -p -G 2>/dev/null || true');
      const windows: WindowManager[] = [];

      stdout.split('\n').forEach(line => {
        if (line.trim()) {
          const parts = line.split(/\s+/);
          if (parts.length >= 8) {
            const [windowId, desktop, pid, x, y, width, height, ...titleParts] = parts;
            const title = titleParts.join(' ');

            windows.push({
              id: windowId,
              applicationId: title.toLowerCase().replace(/\s+/g, '-'),
              windowTitle: title,
              processId: parseInt(pid),
              windowId,
              position: { x: parseInt(x), y: parseInt(y) },
              size: { width: parseInt(width), height: parseInt(height) },
              state: 'normal',
              isActive: false,
              workspace: desktop
            });
          }
        }
      });

      return windows;
    } catch (error) {
      console.error('Failed to get window list:', error);
      return [];
    }
  }

  public async manageWindow(windowId: string, action: 'minimize' | 'maximize' | 'close' | 'focus'): Promise<boolean> {
    try {
      const commands: Record<string, string> = {
        minimize: `wmctrl -i -c ${windowId}`,
        maximize: `wmctrl -i -b toggle,maximized_vert,maximized_horz ${windowId}`,
        close: `wmctrl -i -c ${windowId}`,
        focus: `wmctrl -i -a ${windowId}`
      };

      const command = commands[action];
      if (command) {
        await this.executeCommand(command, { id: 'temp' } as InstallationTask);
        return true;
      }
      return false;
    } catch (error) {
      console.error(`Failed to ${action} window ${windowId}:`, error);
      return false;
    }
  }

  public getInstallationStatus(taskId: string): InstallationTask | undefined {
    return this.installations.get(taskId);
  }

  public getAllInstallations(): InstallationTask[] {
    return Array.from(this.installations.values());
  }

  public async cancelInstallation(taskId: string): Promise<boolean> {
    const task = this.installations.get(taskId);
    if (task && task.status !== 'completed' && task.status !== 'failed') {
      task.status = 'cancelled';
      task.endTime = new Date();
      return true;
    }
    return false;
  }

  public async addCustomPackage(pkg: InstallationPackage): Promise<void> {
    this.packageCatalog.set(pkg.id, pkg);
  }

  public async removePackage(packageId: string): Promise<boolean> {
    return this.packageCatalog.delete(packageId);
  }

  public async searchPackages(query: string): Promise<InstallationPackage[]> {
    const packages = Array.from(this.packageCatalog.values());
    const searchTerm = query.toLowerCase();
    
    return packages.filter(pkg => 
      pkg.name.toLowerCase().includes(searchTerm) ||
      pkg.description.toLowerCase().includes(searchTerm) ||
      pkg.category.toLowerCase().includes(searchTerm)
    );
  }
}