// seamless-vm-manager.ts - The Magic Behind One-Click Luna
import { spawn, ChildProcess } from 'child_process';
import { promises as fs } from 'fs';
import path from 'path';
import os from 'os';
import { app } from 'electron';

export class SeamlessVMManager {
  private vmProcess: ChildProcess | null = null;
  private vmPath: string;
  private appDataPath: string;
  private progressCallback?: (message: string, progress: number) => void;
  private isVMReady = false;
  private lunaPort = 8080;

  constructor() {
    this.appDataPath = app.getPath('userData');
    this.vmPath = path.join(this.appDataPath, 'luna-vm');
  }

  // The main magic: Start Luna seamlessly
  async startLunaSeamlessly(onProgress?: (message: string, progress: number) => void): Promise<string> {
    this.progressCallback = onProgress;
    
    try {
      // Phase 1: Ensure VM environment exists
      await this.ensureVMEnvironment();
      
      // Phase 2: Start VM invisibly
      await this.startVMInvisibly();
      
      // Phase 3: Wait for Luna to be ready
      await this.waitForLunaReady();
      
      return `http://localhost:${this.lunaPort}`;
      
    } catch (error) {
      // If anything fails, try to auto-fix
      return await this.attemptAutoRecovery(error);
    }
  }

  // Phase 1: Ensure VM environment exists (first time setup)
  private async ensureVMEnvironment(): Promise<void> {
    this.updateProgress("Setting up Luna environment...", 10);
    
    // Check if VM already exists
    if (await this.vmExists()) {
      this.updateProgress("Luna environment ready", 30);
      return;
    }
    
    // First time: Extract embedded VM
    this.updateProgress("Installing Luna (this happens once)...", 15);
    await this.extractEmbeddedVM();
    
    this.updateProgress("Configuring automation tools...", 25);
    await this.configureVMForCurrentSystem();
    
    this.updateProgress("Luna environment ready", 30);
  }

  // Phase 2: Start VM without user seeing anything
  private async startVMInvisibly(): Promise<void> {
    this.updateProgress("Starting Luna...", 40);
    
    if (await this.isVMRunning()) {
      this.updateProgress("Luna is ready", 80);
      return;
    }
    
    // Detect best hypervisor for this system
    const hypervisor = await this.detectOptimalHypervisor();
    
    this.updateProgress("Initializing automation engine...", 50);
    
    // Start VM based on platform with zero visibility
    await this.startVM(hypervisor);
    
    this.updateProgress("Connecting services...", 70);
  }

  // Phase 3: Wait for Luna to be responsive
  private async waitForLunaReady(): Promise<void> {
    this.updateProgress("Almost ready...", 85);
    
    const maxWaitTime = 60000; // 60 seconds max
    const checkInterval = 2000; // Check every 2 seconds
    const startTime = Date.now();
    
    while (Date.now() - startTime < maxWaitTime) {
      try {
        const response = await fetch(`http://localhost:${this.lunaPort}/health`, {
          signal: AbortSignal.timeout(5000)
        });
        
        if (response.ok) {
          const data = await response.json();
          if (data.status === 'ready') {
            this.updateProgress("Luna is ready!", 100);
            this.isVMReady = true;
            return;
          }
        }
      } catch (error) {
        // Luna not ready yet, keep waiting
      }
      
      await this.sleep(checkInterval);
    }
    
    throw new Error('Luna failed to start within expected time');
  }

  // Extract embedded VM image (happens once)
  private async extractEmbeddedVM(): Promise<void> {
    const embeddedVMPath = path.join(process.resourcesPath, 'luna-vm.7z');
    
    // Check if embedded VM exists
    try {
      await fs.access(embeddedVMPath);
    } catch {
      throw new Error('Luna installation is incomplete');
    }
    
    // Create VM directory
    await fs.mkdir(this.vmPath, { recursive: true });
    
    // Extract VM image
    await this.extract7z(embeddedVMPath, this.vmPath);
  }

  // Configure VM for optimal performance on current system
  private async configureVMForCurrentSystem(): Promise<void> {
    const systemInfo = await this.getSystemInfo();
    
    // Calculate optimal resources
    const optimalConfig = {
      memory: this.calculateOptimalMemory(systemInfo.totalMemory),
      cpus: this.calculateOptimalCPUs(systemInfo.cpuCores),
      port: await this.findAvailablePort(8080, 8090)
    };
    
    // Update VM configuration
    await this.updateVMConfig(optimalConfig);
    
    this.lunaPort = optimalConfig.port;
  }

  // Detect the best hypervisor for this system
  private async detectOptimalHypervisor(): Promise<string> {
    switch (process.platform) {
      case 'win32':
        // Check for Hyper-V first (fastest), then VirtualBox
        if (await this.isHyperVAvailable()) {
          return 'hyperv';
        }
        return 'virtualbox';
        
      case 'darwin':
        // Use native Hypervisor.framework (best performance)
        return 'hypervisor';
        
      case 'linux':
        // Check for KVM (fastest), fallback to QEMU
        if (await this.isKVMAvailable()) {
          return 'kvm';
        }
        return 'qemu';
        
      default:
        throw new Error('Unsupported platform');
    }
  }

  // Start VM with complete invisibility
  private async startVM(hypervisor: string): Promise<void> {
    switch (hypervisor) {
      case 'virtualbox':
        await this.startVirtualBoxVM();
        break;
      case 'hyperv':
        await this.startHyperVVM();
        break;
      case 'hypervisor':
        await this.startMacOSHypervisorVM();
        break;
      case 'kvm':
      case 'qemu':
        await this.startLinuxVM(hypervisor);
        break;
      default:
        throw new Error(`Unsupported hypervisor: ${hypervisor}`);
    }
  }

  // VirtualBox VM startup (Windows fallback)
  private async startVirtualBoxVM(): Promise<void> {
    const vboxPath = path.join(this.vmPath, 'VirtualBox', 'VBoxManage.exe');
    
    // Import VM if not already imported
    if (!await this.isVMImported()) {
      await this.importVMFromOVA();
    }
    
    // Start VM headless (completely invisible)
    this.vmProcess = spawn(vboxPath, [
      'startvm', 'luna-agent',
      '--type', 'headless'
    ], {
      cwd: this.vmPath,
      detached: false,
      stdio: ['ignore', 'ignore', 'ignore'], // No output visible
      windowsHide: true // Hide on Windows
    });
    
    this.setupVMProcessHandlers();
  }

  // Hyper-V VM startup (Windows native)
  private async startHyperVVM(): Promise<void> {
    // Use PowerShell to manage Hyper-V VM
    const psScript = `
      $VM = Get-VM -Name "Luna-Agent" -ErrorAction SilentlyContinue
      if (-not $VM) {
        Import-VM -Path "${path.join(this.vmPath, 'luna-agent.xml')}"
      }
      Start-VM -Name "Luna-Agent"
    `;
    
    this.vmProcess = spawn('powershell.exe', [
      '-WindowStyle', 'Hidden',
      '-Command', psScript
    ], {
      detached: false,
      stdio: ['ignore', 'ignore', 'ignore'],
      windowsHide: true
    });
    
    this.setupVMProcessHandlers();
  }

  // macOS Hypervisor.framework startup
  private async startMacOSHypervisorVM(): Promise<void> {
    const vmBinary = path.join(this.vmPath, 'luna-vm-macos');
    
    this.vmProcess = spawn(vmBinary, [
      '--headless',
      '--memory', '2048',
      '--cpus', '2',
      '--port', this.lunaPort.toString(),
      '--disk', path.join(this.vmPath, 'luna.qcow2')
    ], {
      cwd: this.vmPath,
      detached: false,
      stdio: ['ignore', 'ignore', 'ignore']
    });
    
    this.setupVMProcessHandlers();
  }

  // Linux KVM/QEMU startup
  private async startLinuxVM(hypervisor: string): Promise<void> {
    const qemuBinary = hypervisor === 'kvm' ? 'qemu-system-x86_64' : 'qemu-system-x86_64';
    const qemuPath = path.join(this.vmPath, 'qemu', qemuBinary);
    
    const args = [
      '-m', '2048',
      '-smp', '2',
      '-drive', `file=${path.join(this.vmPath, 'luna.qcow2')}`,
      '-netdev', `user,id=net0,hostfwd=tcp::${this.lunaPort}-:8080`,
      '-device', 'virtio-net,netdev=net0',
      '-nographic',
      '-daemonize'
    ];
    
    if (hypervisor === 'kvm') {
      args.unshift('-enable-kvm');
    }
    
    this.vmProcess = spawn(qemuPath, args, {
      cwd: this.vmPath,
      detached: false,
      stdio: ['ignore', 'ignore', 'ignore']
    });
    
    this.setupVMProcessHandlers();
  }

  // Auto-recovery: Try to fix any startup issues
  private async attemptAutoRecovery(originalError: any): Promise<string> {
    console.log('Attempting auto-recovery for:', originalError.message);
    
    // Recovery strategies (in order)
    const recoveryStrategies = [
      () => this.fixPortConflict(),
      () => this.fixMemoryIssue(),
      () => this.fixPermissionIssue(),
      () => this.reinstallVM(),
      () => this.downloadFreshVM()
    ];
    
    for (const strategy of recoveryStrategies) {
      try {
        this.updateProgress("Fixing startup issue...", 50);
        
        if (await strategy()) {
          this.updateProgress("Issue resolved, starting Luna...", 60);
          return await this.startLunaSeamlessly();
        }
      } catch (error) {
        // Try next strategy
      }
    }
    
    // If all recovery fails, throw user-friendly error
    throw new Error('Luna is having trouble starting. Please restart your computer and try again.');
  }

  // Recovery strategy: Fix port conflicts
  private async fixPortConflict(): Promise<boolean> {
    const newPort = await this.findAvailablePort(8081, 8100);
    if (newPort !== this.lunaPort) {
      this.lunaPort = newPort;
      await this.updateVMPortConfig(newPort);
      return true;
    }
    return false;
  }

  // Recovery strategy: Fix memory issues
  private async fixMemoryIssue(): Promise<boolean> {
    const systemInfo = await this.getSystemInfo();
    const availableMemory = systemInfo.availableMemory;
    
    if (availableMemory < 1024) { // Less than 1GB available
      // Reduce VM memory allocation
      await this.updateVMMemoryConfig(Math.max(512, availableMemory - 512));
      return true;
    }
    
    return false;
  }

  // Recovery strategy: Fix permission issues
  private async fixPermissionIssue(): Promise<boolean> {
    try {
      // Try to fix common permission issues
      if (process.platform === 'win32') {
        // Windows: Try to run with elevated privileges if needed
        return await this.requestElevatedPrivileges();
      } else {
        // Unix: Fix file permissions
        await this.fixUnixPermissions();
        return true;
      }
    } catch {
      return false;
    }
  }

  // Helper methods
  private async vmExists(): Promise<boolean> {
    try {
      await fs.access(path.join(this.vmPath, 'luna.qcow2'));
      return true;
    } catch {
      return false;
    }
  }

  private async isVMRunning(): Promise<boolean> {
    try {
      const response = await fetch(`http://localhost:${this.lunaPort}/health`, {
        signal: AbortSignal.timeout(2000)
      });
      return response.ok;
    } catch {
      return false;
    }
  }

  private updateProgress(message: string, progress: number): void {
    if (this.progressCallback) {
      this.progressCallback(message, progress);
    }
  }

  private async sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  private setupVMProcessHandlers(): void {
    if (!this.vmProcess) return;

    this.vmProcess.on('error', (error) => {
      console.error('VM process error:', error);
    });

    this.vmProcess.on('exit', (code, signal) => {
      console.log(`VM process exited: code=${code}, signal=${signal}`);
      this.vmProcess = null;
      this.isVMReady = false;
    });

    // Hide process from task manager on Windows
    if (process.platform === 'win32') {
      this.hideProcessFromTaskManager();
    }
  }

  private hideProcessFromTaskManager(): void {
    // Windows-specific: Hide VM process from task manager
    // Implementation depends on hypervisor
  }

  // System detection helpers
  private async isHyperVAvailable(): Promise<boolean> {
    // Check if Hyper-V is available and enabled
    try {
      const { exec } = require('child_process');
      return new Promise((resolve) => {
        exec('powershell.exe -Command "Get-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V"', 
          (error, stdout) => {
            resolve(!error && stdout.includes('Enabled'));
          });
      });
    } catch {
      return false;
    }
  }

  private async isKVMAvailable(): Promise<boolean> {
    try {
      await fs.access('/dev/kvm');
      return true;
    } catch {
      return false;
    }
  }

  private async findAvailablePort(start: number, end: number): Promise<number> {
    for (let port = start; port <= end; port++) {
      if (await this.isPortAvailable(port)) {
        return port;
      }
    }
    return start; // Fallback
  }

  private async isPortAvailable(port: number): Promise<boolean> {
    try {
      await fetch(`http://localhost:${port}`, { signal: AbortSignal.timeout(1000) });
      return false; // Port in use
    } catch {
      return true; // Port available
    }
  }

  // System info helpers
  private async getSystemInfo(): Promise<any> {
    const os = require('os');
    return {
      platform: process.platform,
      totalMemory: os.totalmem(),
      availableMemory: os.freemem(),
      cpuCores: os.cpus().length
    };
  }

  private calculateOptimalMemory(totalMemory: number): number {
    const totalGB = totalMemory / (1024 * 1024 * 1024);
    // Use 25% of system RAM, minimum 1GB, maximum 4GB
    return Math.max(1024, Math.min(4096, Math.floor(totalGB * 0.25 * 1024)));
  }

  private calculateOptimalCPUs(totalCPUs: number): number {
    // Use half of available CPUs, minimum 1, maximum 4
    return Math.max(1, Math.min(4, Math.floor(totalCPUs / 2)));
  }

  // Placeholder methods for VM operations
  private async extract7z(source: string, destination: string): Promise<void> {
    // Implementation depends on platform
  }

  private async updateVMConfig(config: any): Promise<void> {
    // Update VM configuration files
  }

  private async updateVMPortConfig(port: number): Promise<void> {
    // Update VM port forwarding configuration
  }

  private async updateVMMemoryConfig(memory: number): Promise<void> {
    // Update VM memory allocation
  }

  // Public interface
  async shutdown(): Promise<void> {
    if (this.vmProcess) {
      this.vmProcess.kill('SIGTERM');
      this.vmProcess = null;
    }
    this.isVMReady = false;
  }

  async restart(): Promise<string> {
    await this.shutdown();
    await this.sleep(2000);
    return await this.startLunaSeamlessly(this.progressCallback);
  }

  isReady(): boolean {
    return this.isVMReady;
  }

  getEndpoint(): string {
    return `http://localhost:${this.lunaPort}`;
  }
}