// vm-engine.ts - Seamless VM Management
import { spawn, ChildProcess } from 'child_process';
import { promises as fs } from 'fs';
import path from 'path';
import os from 'os';
import { extract7z } from 'node-7z';
import si from 'systeminformation';
import axios from 'axios';

export class LunaVMEngine {
  private vmProcess: ChildProcess | null = null;
  private vmPath: string;
  private vmPort: number = 8080;
  private isInitialized = false;

  constructor() {
    this.vmPath = this.getVMPath();
  }

  // Ensure VM is ready to use
  async ensureReady(): Promise<void> {
    if (!await this.vmExists()) {
      await this.extractVM();
      await this.configureVM();
    }

    if (!await this.isRunning()) {
      await this.startVM();
    }

    this.isInitialized = true;
  }

  // Check if system meets requirements
  async checkSystemRequirements(): Promise<boolean> {
    const system = await si.system();
    const mem = await si.mem();
    const cpu = await si.cpu();

    // Minimum requirements
    const minRAM = 4 * 1024 * 1024 * 1024; // 4GB
    const minCPUs = 2;
    const minDisk = 2 * 1024 * 1024 * 1024; // 2GB

    return (
      mem.total >= minRAM &&
      cpu.cores >= minCPUs &&
      await this.getAvailableDiskSpace() >= minDisk
    );
  }

  // Extract embedded VM image
  private async extractVM(): Promise<void> {
    const vmImagePath = this.getEmbeddedVMPath();
    const extractPath = this.vmPath;

    console.log('🔧 Extracting Luna VM...');
    
    // Create VM directory
    await fs.mkdir(extractPath, { recursive: true });
    
    // Extract compressed VM image
    await extract7z(vmImagePath, extractPath);
    
    console.log('✅ VM extracted successfully');
  }

  // Configure VM for current system
  private async configureVM(): Promise<void> {
    console.log('⚙️ Configuring Luna VM...');
    
    // Detect available hypervisor
    const hypervisor = await this.detectHypervisor();
    
    // Configure VM for optimal performance
    await this.optimizeVMConfig(hypervisor);
    
    // Set up port forwarding
    await this.configureNetworking();
    
    console.log('✅ VM configured');
  }

  // Start VM (headless, invisible to user)
  private async startVM(): Promise<void> {
    console.log('🚀 Starting Luna VM...');
    
    const hypervisor = await this.detectHypervisor();
    const vmConfig = await this.getVMConfig();
    
    // Start VM based on platform
    switch (process.platform) {
      case 'win32':
        await this.startVMWindows(vmConfig);
        break;
      case 'darwin':
        await this.startVMMacOS(vmConfig);
        break;
      case 'linux':
        await this.startVMLinux(vmConfig);
        break;
      default:
        throw new Error(`Unsupported platform: ${process.platform}`);
    }
    
    console.log('✅ VM started');
  }

  // Windows VM startup
  private async startVMWindows(config: any): Promise<void> {
    const vboxPath = path.join(this.vmPath, 'VirtualBox', 'VBoxManage.exe');
    
    this.vmProcess = spawn(vboxPath, [
      'startvm', 'luna-agent',
      '--type', 'headless'
    ], {
      cwd: this.vmPath,
      detached: false,
      stdio: ['ignore', 'pipe', 'pipe']
    });

    this.setupVMProcessHandlers();
  }

  // macOS VM startup
  private async startVMMacOS(config: any): Promise<void> {
    // Use native Hypervisor.framework or VirtualBox
    const vmBinary = path.join(this.vmPath, 'bin', 'luna-vm');
    
    this.vmProcess = spawn(vmBinary, [
      '--headless',
      '--port', this.vmPort.toString(),
      '--memory', '2048'
    ], {
      cwd: this.vmPath,
      detached: false,
      stdio: ['ignore', 'pipe', 'pipe']
    });

    this.setupVMProcessHandlers();
  }

  // Linux VM startup
  private async startVMLinux(config: any): Promise<void> {
    const qemuPath = path.join(this.vmPath, 'qemu', 'qemu-system-x86_64');
    
    this.vmProcess = spawn(qemuPath, [
      '-enable-kvm',
      '-m', '2048',
      '-smp', '2',
      '-drive', `file=${path.join(this.vmPath, 'luna.qcow2')}`,
      '-netdev', `user,id=net0,hostfwd=tcp::${this.vmPort}-:8080`,
      '-device', 'virtio-net,netdev=net0',
      '-nographic',
      '-daemonize'
    ], {
      cwd: this.vmPath,
      detached: false,
      stdio: ['ignore', 'pipe', 'pipe']
    });

    this.setupVMProcessHandlers();
  }

  // Set up VM process event handlers
  private setupVMProcessHandlers(): void {
    if (!this.vmProcess) return;

    this.vmProcess.on('error', (error) => {
      console.error('VM process error:', error);
    });

    this.vmProcess.on('exit', (code, signal) => {
      console.log(`VM process exited with code ${code}, signal ${signal}`);
      this.vmProcess = null;
    });

    // Hide VM process from task manager (Windows)
    if (process.platform === 'win32' && this.vmProcess.pid) {
      this.hideProcessFromTaskManager(this.vmProcess.pid);
    }
  }

  // Wait for Luna agent to be ready
  async waitForLunaReady(timeoutMs: number = 60000): Promise<void> {
    const startTime = Date.now();
    const checkInterval = 2000; // 2 seconds

    while (Date.now() - startTime < timeoutMs) {
      try {
        const response = await axios.get(`http://localhost:${this.vmPort}/health`, {
          timeout: 5000
        });

        if (response.status === 200 && response.data.status === 'ready') {
          console.log('✅ Luna agent is ready');
          return;
        }
      } catch (error) {
        // Luna not ready yet, keep trying
      }

      await this.sleep(checkInterval);
    }

    throw new Error('Luna agent failed to start within timeout');
  }

  // Check if VM is running
  async isRunning(): Promise<boolean> {
    if (!this.vmProcess) return false;

    try {
      const response = await axios.get(`http://localhost:${this.vmPort}/health`, {
        timeout: 5000
      });
      return response.status === 200;
    } catch {
      return false;
    }
  }

  // Get Luna endpoint
  getEndpoint(): string {
    return `http://localhost:${this.vmPort}`;
  }

  // Restart VM
  async restart(): Promise<void> {
    if (await this.isRunning()) {
      await this.shutdown();
      await this.sleep(2000);
    }
    await this.startVM();
    await this.waitForLunaReady();
  }

  // Graceful shutdown
  async shutdown(): Promise<void> {
    console.log('🔄 Shutting down Luna VM...');

    if (this.vmProcess) {
      // Send graceful shutdown signal
      this.vmProcess.kill('SIGTERM');
      
      // Wait for graceful shutdown
      await this.sleep(5000);
      
      // Force kill if still running
      if (this.vmProcess && !this.vmProcess.killed) {
        this.vmProcess.kill('SIGKILL');
      }
      
      this.vmProcess = null;
    }

    console.log('✅ VM shut down');
  }

  // Get system information
  async getSystemInfo(): Promise<any> {
    const [cpu, mem, osInfo] = await Promise.all([
      si.cpu(),
      si.mem(),
      si.osInfo()
    ]);

    return {
      platform: process.platform,
      arch: process.arch,
      cpu: {
        manufacturer: cpu.manufacturer,
        brand: cpu.brand,
        cores: cpu.cores,
        physicalCores: cpu.physicalCores
      },
      memory: {
        total: mem.total,
        free: mem.free,
        used: mem.used
      },
      os: {
        platform: osInfo.platform,
        distro: osInfo.distro,
        release: osInfo.release,
        arch: osInfo.arch
      }
    };
  }

  // Helper methods
  private getVMPath(): string {
    const userDataPath = app.getPath('userData');
    return path.join(userDataPath, 'luna-vm');
  }

  private getEmbeddedVMPath(): string {
    return path.join(process.resourcesPath, 'vm-image.7z');
  }

  private async vmExists(): Promise<boolean> {
    try {
      await fs.access(this.vmPath);
      return true;
    } catch {
      return false;
    }
  }

  private async detectHypervisor(): Promise<string> {
    switch (process.platform) {
      case 'win32':
        // Check for Hyper-V, then VirtualBox
        return await this.checkHyperV() ? 'hyperv' : 'virtualbox';
      case 'darwin':
        // Use native Hypervisor.framework
        return 'hypervisor';
      case 'linux':
        // Check for KVM, then fallback
        return await this.checkKVM() ? 'kvm' : 'qemu';
      default:
        throw new Error('Unsupported platform');
    }
  }

  private async checkHyperV(): Promise<boolean> {
    // Check if Hyper-V is available
    try {
      const { exec } = require('child_process');
      return new Promise((resolve) => {
        exec('bcdedit /enum | findstr hypervisorlaunchtype', (error, stdout) => {
          resolve(!error && stdout.includes('Auto'));
        });
      });
    } catch {
      return false;
    }
  }

  private async checkKVM(): Promise<boolean> {
    try {
      await fs.access('/dev/kvm');
      return true;
    } catch {
      return false;
    }
  }

  private async getAvailableDiskSpace(): Promise<number> {
    const stats = await fs.statfs(this.vmPath);
    return stats.bavail * stats.bsize;
  }

  private async getVMConfig(): Promise<any> {
    // Return optimized VM configuration
    return {
      memory: await this.calculateOptimalMemory(),
      cpus: await this.calculateOptimalCPUs(),
      disk: '10GB'
    };
  }

  private async calculateOptimalMemory(): Promise<number> {
    const mem = await si.mem();
    const totalGB = Math.floor(mem.total / (1024 * 1024 * 1024));
    
    // Use 25% of system RAM, minimum 1GB, maximum 4GB
    const vmMemory = Math.max(1, Math.min(4, Math.floor(totalGB * 0.25)));
    return vmMemory * 1024; // Return in MB
  }

  private async calculateOptimalCPUs(): Promise<number> {
    const cpu = await si.cpu();
    // Use half of available cores, minimum 1, maximum 4
    return Math.max(1, Math.min(4, Math.floor(cpu.cores / 2)));
  }

  private async optimizeVMConfig(hypervisor: string): Promise<void> {
    // Platform-specific optimizations
    // Implementation depends on hypervisor
  }

  private async configureNetworking(): Promise<void> {
    // Set up port forwarding for Luna
    // Implementation depends on hypervisor
  }

  private hideProcessFromTaskManager(pid: number): void {
    // Windows-specific: Hide VM process from task manager
    if (process.platform === 'win32') {
      const { exec } = require('child_process');
      exec(`powershell -Command "Get-Process -Id ${pid} | Set-ItemProperty -Name ProcessName -Value 'System'"`);
    }
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

// Import app after class definition
import { app } from 'electron';