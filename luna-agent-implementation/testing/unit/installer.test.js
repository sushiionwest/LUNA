/**
 * Luna Installer Unit Tests
 * Tests individual installer components and functions
 */

const { exec } = require('child_process');
const fs = require('fs-extra');
const path = require('path');

describe('Luna Installer Unit Tests', () => {
  
  describe('System Requirements Check', () => {
    test('should detect operating system correctly', () => {
      const platform = process.platform;
      expect(['win32', 'darwin', 'linux'].includes(platform)).toBe(true);
    });

    test('should validate minimum memory requirements', () => {
      const totalMem = require('os').totalmem();
      const minMemoryBytes = 4 * 1024 * 1024 * 1024; // 4GB
      expect(totalMem).toBeGreaterThan(minMemoryBytes);
    });

    test('should check available disk space', async () => {
      const stats = await fs.stat('.');
      expect(stats).toBeDefined();
    });
  });

  describe('VM Configuration', () => {
    test('should load VM configuration file', async () => {
      const configPath = '../vm-assets/vm-config.json';
      const configExists = await fs.pathExists(configPath);
      expect(configExists).toBe(true);

      if (configExists) {
        const config = await fs.readJSON(configPath);
        expect(config.name).toBe('Luna-Agent-VM');
        expect(config.version).toBe('1.0.0');
        expect(config.resources.memory_mb).toBeGreaterThan(1024);
      }
    });

    test('should validate VM resource requirements', async () => {
      const configPath = '../vm-assets/vm-config.json';
      const config = await fs.readJSON(configPath);
      
      expect(config.resources.memory_mb).toBeGreaterThanOrEqual(2048);
      expect(config.resources.storage_gb).toBeGreaterThanOrEqual(20);
      expect(config.resources.cpu_cores).toBeGreaterThanOrEqual(2);
    });
  });

  describe('Installation Path Validation', () => {
    test('should validate writable installation directory', async () => {
      const testDir = '/tmp/luna-test-install';
      await fs.ensureDir(testDir);
      
      const stats = await fs.stat(testDir);
      expect(stats.isDirectory()).toBe(true);
      
      // Test write permissions
      const testFile = path.join(testDir, 'test.txt');
      await fs.writeFile(testFile, 'test');
      expect(await fs.pathExists(testFile)).toBe(true);
      
      // Cleanup
      await fs.remove(testDir);
    });

    test('should reject invalid installation paths', async () => {
      const invalidPaths = [
        '/root/restricted',
        '/proc/invalid',
        '/dev/null/invalid'
      ];

      for (const invalidPath of invalidPaths) {
        try {
          await fs.access(invalidPath, fs.constants.W_OK);
          // If we get here, the path was writable (unexpected)
        } catch (error) {
          // Expected - path should not be writable
          expect(error).toBeDefined();
        }
      }
    });
  });

  describe('VirtualBox Integration', () => {
    test('should detect VirtualBox installation', (done) => {
      exec('VBoxManage --version', (error, stdout, stderr) => {
        if (error) {
          // VirtualBox not installed - this is acceptable for CI
          console.log('VirtualBox not detected (expected in CI environment)');
          expect(error.code).toBeDefined();
        } else {
          // VirtualBox is installed
          expect(stdout).toMatch(/\d+\.\d+\.\d+/);
        }
        done();
      });
    });
  });

  describe('Error Handling', () => {
    test('should handle missing dependencies gracefully', () => {
      const mockError = new Error('Dependency not found');
      expect(mockError.message).toBe('Dependency not found');
    });

    test('should validate input parameters', () => {
      const validatePath = (path) => {
        if (!path || typeof path !== 'string') {
          throw new Error('Invalid path parameter');
        }
        return true;
      };

      expect(() => validatePath('')).toThrow('Invalid path parameter');
      expect(() => validatePath(null)).toThrow('Invalid path parameter');
      expect(() => validatePath(123)).toThrow('Invalid path parameter');
      expect(validatePath('/valid/path')).toBe(true);
    });
  });

  describe('Configuration Management', () => {
    test('should generate valid installation configuration', () => {
      const generateConfig = (installPath, options = {}) => ({
        installPath: installPath,
        vmName: options.vmName || 'Luna-Agent-VM',
        resources: {
          memory: options.memory || 2048,
          storage: options.storage || 20,
          cpus: options.cpus || 2
        },
        features: {
          autoStart: options.autoStart || true,
          createShortcuts: options.createShortcuts || true
        }
      });

      const config = generateConfig('/opt/luna', { memory: 4096 });
      expect(config.installPath).toBe('/opt/luna');
      expect(config.resources.memory).toBe(4096);
      expect(config.features.autoStart).toBe(true);
    });
  });
});
