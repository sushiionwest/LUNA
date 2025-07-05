import { describe, it, expect, beforeEach, afterEach, jest } from '@jest/globals';
import { SecureCredentialStorage, CredentialEntry, SecureStorageConfig } from '../SecureCredentialStorage';
import { join } from 'path';
import { tmpdir } from 'os';
import { rmSync, existsSync, readFileSync } from 'fs';

describe('SecureCredentialStorage', () => {
  let storage: SecureCredentialStorage;
  let testStorageDir: string;
  let config: SecureStorageConfig;

  beforeEach(() => {
    testStorageDir = join(tmpdir(), `luna-test-${Date.now()}`);
    config = {
      applicationName: 'LunaTestApp',
      storageLocation: testStorageDir,
      encryptionMethod: 'dpapi',
      enableAuditLog: true
    };
    storage = new SecureCredentialStorage(config);
  });

  afterEach(() => {
    if (existsSync(testStorageDir)) {
      rmSync(testStorageDir, { recursive: true, force: true });
    }
  });

  describe('Basic Operations', () => {
    it('should store and retrieve credentials', async () => {
      const credentialData = {
        username: 'testuser',
        password: 'supersecret123',
        apiToken: 'token_abc123',
        additionalData: { 
          endpoint: 'https://api.example.com',
          refreshToken: 'refresh_xyz789'
        }
      };

      await storage.storeCredential('test-cred', credentialData);
      const retrieved = await storage.retrieveCredential('test-cred');

      expect(retrieved).toBeTruthy();
      expect(retrieved!.username).toBe(credentialData.username);
      expect(retrieved!.password).toBe(credentialData.password);
      expect(retrieved!.apiToken).toBe(credentialData.apiToken);
      expect(retrieved!.additionalData).toEqual(credentialData.additionalData);
      expect(retrieved!.id).toBe('test-cred');
      expect(retrieved!.createdAt).toBeInstanceOf(Date);
      expect(retrieved!.lastAccessed).toBeInstanceOf(Date);
    });

    it('should return null for non-existent credentials', async () => {
      const retrieved = await storage.retrieveCredential('non-existent');
      expect(retrieved).toBeNull();
    });

    it('should delete credentials successfully', async () => {
      const credentialData = {
        username: 'testuser',
        password: 'supersecret123'
      };

      await storage.storeCredential('delete-test', credentialData);
      let retrieved = await storage.retrieveCredential('delete-test');
      expect(retrieved).toBeTruthy();

      const deleted = await storage.deleteCredential('delete-test');
      expect(deleted).toBe(true);

      retrieved = await storage.retrieveCredential('delete-test');
      expect(retrieved).toBeNull();
    });

    it('should list stored credentials', async () => {
      const creds = [
        { id: 'cred1', data: { username: 'user1', password: 'pass1' } },
        { id: 'cred2', data: { username: 'user2', password: 'pass2' } },
        { id: 'cred3', data: { username: 'user3', password: 'pass3' } }
      ];

      for (const cred of creds) {
        await storage.storeCredential(cred.id, cred.data);
      }

      const credentialIds = await storage.listCredentials();
      expect(credentialIds).toHaveLength(3);
      expect(credentialIds).toContain('cred1');
      expect(credentialIds).toContain('cred2');
      expect(credentialIds).toContain('cred3');
    });
  });

  describe('Encryption and Security', () => {
    it('should encrypt stored data on disk', async () => {
      const sensitiveData = {
        username: 'admin',
        password: 'topsecret123!@#',
        apiToken: 'very_secret_token_12345'
      };

      await storage.storeCredential('encryption-test', sensitiveData);

      // Check that the raw file data doesn't contain plaintext credentials
      const files = require('fs').readdirSync(testStorageDir);
      const credentialFile = files.find((file: string) => file.includes('encryption-test'));
      
      if (credentialFile) {
        const rawData = readFileSync(join(testStorageDir, credentialFile), 'utf8');
        
        // Verify sensitive data is not in plaintext
        expect(rawData).not.toContain('topsecret123!@#');
        expect(rawData).not.toContain('very_secret_token_12345');
        expect(rawData).not.toContain('admin');
      }
    });

    it('should validate access and user context', async () => {
      const validation = await storage.validateAccess();
      
      expect(validation).toHaveProperty('isValid');
      expect(validation).toHaveProperty('currentUser');
      expect(validation).toHaveProperty('errors');
      expect(Array.isArray(validation.errors)).toBe(true);
      
      if (process.platform === 'win32') {
        expect(validation.currentUser).not.toBe('unknown');
      }
    });

    it('should handle different encryption methods', async () => {
      const methods: Array<'dpapi' | 'credential-manager' | 'hybrid'> = ['dpapi', 'credential-manager', 'hybrid'];
      
      for (const method of methods) {
        const methodConfig = { ...config, encryptionMethod: method };
        const methodStorage = new SecureCredentialStorage(methodConfig);
        
        const testData = {
          username: `user_${method}`,
          password: `password_${method}`,
          apiToken: `token_${method}`
        };

        try {
          await methodStorage.storeCredential(`test-${method}`, testData);
          const retrieved = await methodStorage.retrieveCredential(`test-${method}`);
          
          expect(retrieved).toBeTruthy();
          expect(retrieved!.username).toBe(testData.username);
          expect(retrieved!.password).toBe(testData.password);
          expect(retrieved!.apiToken).toBe(testData.apiToken);
        } catch (error) {
          // On non-Windows platforms, some methods may not be available
          if (process.platform !== 'win32' && (method === 'credential-manager' || method === 'dpapi')) {
            expect(error.message).toContain('only available on Windows');
          } else {
            throw error;
          }
        }
      }
    });

    it('should maintain audit logs', async () => {
      const auditStorage = new SecureCredentialStorage({
        ...config,
        enableAuditLog: true
      });

      await auditStorage.storeCredential('audit-test', { username: 'test', password: 'test' });
      await auditStorage.retrieveCredential('audit-test');
      await auditStorage.deleteCredential('audit-test');

      const auditLogPath = join(testStorageDir, 'access-audit.log');
      
      if (existsSync(auditLogPath)) {
        const auditLog = readFileSync(auditLogPath, 'utf8');
        
        expect(auditLog).toContain('STORE | audit-test | SUCCESS');
        expect(auditLog).toContain('RETRIEVE | audit-test | SUCCESS');
        expect(auditLog).toContain('DELETE | audit-test | SUCCESS');
      }
    });
  });

  describe('Input Validation', () => {
    it('should validate credential IDs', async () => {
      const invalidIds = ['', ' ', 'with spaces', 'with@symbols', 'with.dots'];
      
      for (const invalidId of invalidIds) {
        await expect(storage.storeCredential(invalidId, { username: 'test' }))
          .rejects.toThrow();
      }
    });

    it('should accept valid credential IDs', async () => {
      const validIds = ['valid-id', 'valid_id', 'ValidId123', 'api-token-1'];
      
      for (const validId of validIds) {
        await expect(storage.storeCredential(validId, { username: 'test' }))
          .resolves.not.toThrow();
      }
    });
  });

  describe('Error Handling', () => {
    it('should handle storage directory creation', () => {
      const config = {
        applicationName: 'TestApp',
        storageLocation: join(tmpdir(), 'new-dir', 'nested', 'deep'),
        encryptionMethod: 'dpapi' as const
      };

      expect(() => new SecureCredentialStorage(config)).not.toThrow();
    });

    it('should handle encryption failures gracefully', async () => {
      // Mock encryption failure
      const originalExecutePowerShell = (storage as any).executePowerShell;
      (storage as any).executePowerShell = jest.fn().mockRejectedValue(new Error('PowerShell failed'));

      await expect(storage.storeCredential('fail-test', { username: 'test' }))
        .rejects.toThrow('Failed to store credential');

      // Restore original method
      (storage as any).executePowerShell = originalExecutePowerShell;
    });

    it('should handle missing credentials gracefully', async () => {
      const retrieved = await storage.retrieveCredential('definitely-does-not-exist');
      expect(retrieved).toBeNull();

      const deleted = await storage.deleteCredential('definitely-does-not-exist');
      expect(deleted).toBe(false);
    });
  });

  describe('Cross-Platform Compatibility', () => {
    it('should work on non-Windows platforms with fallback encryption', async () => {
      // Force non-Windows behavior for testing
      const originalPlatform = process.platform;
      Object.defineProperty(process, 'platform', { value: 'linux' });

      try {
        const fallbackStorage = new SecureCredentialStorage(config);
        const testData = {
          username: 'crossplatform',
          password: 'testpassword',
          apiToken: 'testtoken'
        };

        await fallbackStorage.storeCredential('crossplatform-test', testData);
        const retrieved = await fallbackStorage.retrieveCredential('crossplatform-test');

        expect(retrieved).toBeTruthy();
        expect(retrieved!.username).toBe(testData.username);
        expect(retrieved!.password).toBe(testData.password);
        expect(retrieved!.apiToken).toBe(testData.apiToken);
      } finally {
        Object.defineProperty(process, 'platform', { value: originalPlatform });
      }
    });
  });

  describe('Performance and Concurrency', () => {
    it('should handle concurrent operations', async () => {
      const operations = [];
      
      // Create multiple concurrent store operations
      for (let i = 0; i < 10; i++) {
        operations.push(
          storage.storeCredential(`concurrent-${i}`, {
            username: `user${i}`,
            password: `password${i}`
          })
        );
      }

      await Promise.all(operations);

      // Verify all credentials were stored
      const credentialIds = await storage.listCredentials();
      expect(credentialIds).toHaveLength(10);

      // Concurrent retrieval
      const retrieveOperations = [];
      for (let i = 0; i < 10; i++) {
        retrieveOperations.push(storage.retrieveCredential(`concurrent-${i}`));
      }

      const results = await Promise.all(retrieveOperations);
      expect(results.every(result => result !== null)).toBe(true);
    });

    it('should perform reasonably well for common operations', async () => {
      const start = Date.now();
      
      // Store 20 credentials
      for (let i = 0; i < 20; i++) {
        await storage.storeCredential(`perf-test-${i}`, {
          username: `user${i}`,
          password: `password${i}`,
          apiToken: `token${i}`
        });
      }

      const storeTime = Date.now() - start;
      expect(storeTime).toBeLessThan(10000); // Should take less than 10 seconds

      const retrieveStart = Date.now();
      
      // Retrieve all credentials
      for (let i = 0; i < 20; i++) {
        await storage.retrieveCredential(`perf-test-${i}`);
      }

      const retrieveTime = Date.now() - retrieveStart;
      expect(retrieveTime).toBeLessThan(5000); // Should take less than 5 seconds
    });
  });

  describe('Security Edge Cases', () => {
    it('should not leak sensitive data in error messages', async () => {
      const sensitiveData = {
        username: 'admin',
        password: 'super_secret_password_123',
        apiToken: 'ultra_secret_token_xyz'
      };

      // Store credential
      await storage.storeCredential('sensitive-test', sensitiveData);

      // Mock an error scenario
      const originalDecrypt = (storage as any).decryptWithDPAPI;
      (storage as any).decryptWithDPAPI = jest.fn().mockRejectedValue(new Error('Decryption failed'));

      try {
        await storage.retrieveCredential('sensitive-test');
      } catch (error) {
        // Ensure error message doesn't contain sensitive data
        expect(error.message).not.toContain('super_secret_password_123');
        expect(error.message).not.toContain('ultra_secret_token_xyz');
      }

      // Restore original method
      (storage as any).decryptWithDPAPI = originalDecrypt;
    });

    it('should handle corrupted storage files', async () => {
      await storage.storeCredential('corruption-test', { username: 'test', password: 'test' });

      // Corrupt the storage file
      const files = require('fs').readdirSync(testStorageDir);
      const credentialFile = files.find((file: string) => file.includes('corruption-test'));
      
      if (credentialFile) {
        require('fs').writeFileSync(join(testStorageDir, credentialFile), 'corrupted data');
        
        // Should handle corruption gracefully
        await expect(storage.retrieveCredential('corruption-test'))
          .rejects.toThrow();
      }
    });

    it('should properly clean up temporary test credentials', async () => {
      const initialValidation = await storage.validateAccess();
      
      // Validation should clean up its test credential
      const credentials = await storage.listCredentials();
      const testCredentials = credentials.filter(id => id.startsWith('__test_access_'));
      
      expect(testCredentials).toHaveLength(0);
    });
  });
});