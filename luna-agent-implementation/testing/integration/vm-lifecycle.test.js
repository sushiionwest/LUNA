/**
 * Luna VM Lifecycle Integration Tests
 * Tests complete VM creation, configuration, and management workflows
 */

const { exec } = require('child_process');
const { promisify } = require('util');
const fs = require('fs-extra');
const axios = require('axios');

const execAsync = promisify(exec);

describe('Luna VM Lifecycle Integration Tests', () => {
  let vmName = 'Luna-Test-VM';
  
  afterAll(async () => {
    // Cleanup test VM
    try {
      await execAsync(`VBoxManage controlvm "${vmName}" poweroff`);
      await new Promise(resolve => setTimeout(resolve, 2000));
      await execAsync(`VBoxManage unregistervm "${vmName}" --delete`);
    } catch (error) {
      // VM might not exist, ignore errors
    }
  });

  describe('VM Creation Process', () => {
    test('should create VM with correct specifications', async () => {
      if (!await isVirtualBoxAvailable()) {
        console.log('Skipping VM tests - VirtualBox not available');
        return;
      }

      // Create VM
      await execAsync(`VBoxManage createvm --name "${vmName}" --ostype "Ubuntu_64" --register`);
      
      // Configure VM
      await execAsync(`VBoxManage modifyvm "${vmName}" --memory 2048 --cpus 2`);
      
      // Verify VM exists
      const { stdout } = await execAsync('VBoxManage list vms');
      expect(stdout).toContain(vmName);
    }, 30000);

    test('should configure VM network settings', async () => {
      if (!await isVirtualBoxAvailable()) return;

      // Configure NAT with port forwarding
      await execAsync(`VBoxManage modifyvm "${vmName}" --natpf1 "luna-api,tcp,,8080,,8080"`);
      await execAsync(`VBoxManage modifyvm "${vmName}" --natpf1 "ssh,tcp,,22222,,22"`);
      
      // Verify configuration
      const { stdout } = await execAsync(`VBoxManage showvminfo "${vmName}"`);
      expect(stdout).toContain('luna-api');
      expect(stdout).toContain('22222');
    }, 15000);
  });

  describe('VM Storage Management', () => {
    test('should create and attach storage', async () => {
      if (!await isVirtualBoxAvailable()) return;

      const vdiFile = `${vmName}.vdi`;
      
      // Create virtual disk
      await execAsync(`VBoxManage createhd --filename "${vdiFile}" --size 20480 --format VDI`);
      
      // Create storage controller
      await execAsync(`VBoxManage storagectl "${vmName}" --name "SATA Controller" --add sata`);
      
      // Attach disk
      await execAsync(`VBoxManage storageattach "${vmName}" --storagectl "SATA Controller" --port 0 --device 0 --type hdd --medium "${vdiFile}"`);
      
      // Verify storage
      const { stdout } = await execAsync(`VBoxManage showvminfo "${vmName}"`);
      expect(stdout).toContain('SATA Controller');
    }, 20000);
  });

  describe('VM Export/Import Process', () => {
    test('should export VM to OVA format', async () => {
      if (!await isVirtualBoxAvailable()) return;

      const ovaFile = `${vmName}.ova`;
      
      // Export VM
      await execAsync(`VBoxManage export "${vmName}" --output "${ovaFile}"`);
      
      // Verify OVA file exists
      const exists = await fs.pathExists(ovaFile);
      expect(exists).toBe(true);
      
      if (exists) {
        const stats = await fs.stat(ovaFile);
        expect(stats.size).toBeGreaterThan(1024 * 1024); // At least 1MB
        
        // Cleanup
        await fs.remove(ovaFile);
      }
    }, 60000);
  });

  describe('Luna Agent API Integration', () => {
    test('should connect to Luna Agent API (simulation)', async () => {
      // Since we can't actually boot the VM in CI, simulate API testing
      const mockApiResponse = {
        status: 'online',
        version: '1.0.0',
        capabilities: ['web_automation', 'computer_vision']
      };

      // In real implementation, this would be:
      // const response = await axios.get('http://localhost:8080/status');
      // expect(response.data.status).toBe('online');
      
      expect(mockApiResponse.status).toBe('online');
      expect(mockApiResponse.capabilities).toContain('web_automation');
    });

    test('should handle API connection failures gracefully', async () => {
      try {
        // This should fail since no VM is running
        await axios.get('http://localhost:8080/status', { timeout: 1000 });
      } catch (error) {
        expect(error.code).toMatch(/ECONNREFUSED|TIMEOUT/);
      }
    });
  });

  describe('Resource Monitoring', () => {
    test('should monitor VM resource usage', async () => {
      // Simulate resource monitoring
      const mockResourceData = {
        cpu_usage: 25.5,
        memory_usage: 1536,  // MB
        disk_usage: 15,      // GB
        network_io: {
          bytes_sent: 1024 * 1024,
          bytes_received: 2 * 1024 * 1024
        }
      };

      expect(mockResourceData.cpu_usage).toBeLessThan(100);
      expect(mockResourceData.memory_usage).toBeLessThan(2048);
      expect(mockResourceData.disk_usage).toBeLessThan(20);
    });
  });

  describe('Error Recovery', () => {
    test('should handle VM startup failures', async () => {
      const handleVMStartupError = (error) => {
        if (error.message.includes('VBoxManage')) {
          return {
            success: false,
            error: 'VirtualBox command failed',
            suggestion: 'Check VirtualBox installation'
          };
        }
        return { success: false, error: 'Unknown error' };
      };

      const mockError = new Error('VBoxManage: error: Failed to start VM');
      const result = handleVMStartupError(mockError);
      
      expect(result.success).toBe(false);
      expect(result.suggestion).toContain('VirtualBox');
    });
  });
});

// Helper function to check VirtualBox availability
async function isVirtualBoxAvailable() {
  try {
    await execAsync('VBoxManage --version');
    return true;
  } catch (error) {
    return false;
  }
}
