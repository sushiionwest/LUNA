import { execSync, spawn } from 'child_process';
import { promisify } from 'util';
import { randomBytes, createCipher, createDecipher, pbkdf2 } from 'crypto';
import { writeFileSync, readFileSync, existsSync, mkdirSync } from 'fs';
import { join } from 'path';
import { os } from 'os';

const pbkdf2Async = promisify(pbkdf2);

export interface CredentialEntry {
  id: string;
  username?: string;
  password?: string;
  apiToken?: string;
  additionalData?: Record<string, string>;
  createdAt: Date;
  lastAccessed: Date;
}

export interface SecureStorageConfig {
  applicationName: string;
  storageLocation?: string;
  encryptionMethod: 'dpapi' | 'credential-manager' | 'hybrid';
  enableAuditLog?: boolean;
}

/**
 * Secure credential storage service for Windows
 * Uses DPAPI (Data Protection API) and Windows Credential Manager
 * to securely store sensitive data like API tokens and passwords.
 */
export class SecureCredentialStorage {
  private config: SecureStorageConfig;
  private storageDir: string;
  private auditLogPath: string;
  private isWindows: boolean;

  constructor(config: SecureStorageConfig) {
    this.config = {
      enableAuditLog: true,
      encryptionMethod: 'hybrid',
      ...config
    };
    
    this.isWindows = process.platform === 'win32';
    this.storageDir = config.storageLocation || join(os.homedir(), 'AppData', 'Local', config.applicationName, 'SecureStorage');
    this.auditLogPath = join(this.storageDir, 'access-audit.log');
    
    this.ensureStorageDirectory();
  }

  /**
   * Store credentials securely using Windows DPAPI or Credential Manager
   */
  async storeCredential(id: string, credential: Omit<CredentialEntry, 'id' | 'createdAt' | 'lastAccessed'>): Promise<void> {
    this.validateId(id);
    
    const entry: CredentialEntry = {
      ...credential,
      id,
      createdAt: new Date(),
      lastAccessed: new Date()
    };

    try {
      switch (this.config.encryptionMethod) {
        case 'dpapi':
          await this.storeDPAPI(id, entry);
          break;
        case 'credential-manager':
          await this.storeCredentialManager(id, entry);
          break;
        case 'hybrid':
          // Store metadata in DPAPI, actual credentials in Credential Manager
          await this.storeHybrid(id, entry);
          break;
      }

      this.auditLog('STORE', id, 'SUCCESS');
    } catch (error) {
      this.auditLog('STORE', id, 'FAILED', error.message);
      throw new Error(`Failed to store credential '${id}': ${error.message}`);
    }
  }

  /**
   * Retrieve credentials securely
   */
  async retrieveCredential(id: string): Promise<CredentialEntry | null> {
    this.validateId(id);

    try {
      let credential: CredentialEntry | null = null;

      switch (this.config.encryptionMethod) {
        case 'dpapi':
          credential = await this.retrieveDPAPI(id);
          break;
        case 'credential-manager':
          credential = await this.retrieveCredentialManager(id);
          break;
        case 'hybrid':
          credential = await this.retrieveHybrid(id);
          break;
      }

      if (credential) {
        credential.lastAccessed = new Date();
        // Update last accessed time
        await this.storeCredential(id, credential);
        this.auditLog('RETRIEVE', id, 'SUCCESS');
      } else {
        this.auditLog('RETRIEVE', id, 'NOT_FOUND');
      }

      return credential;
    } catch (error) {
      this.auditLog('RETRIEVE', id, 'FAILED', error.message);
      throw new Error(`Failed to retrieve credential '${id}': ${error.message}`);
    }
  }

  /**
   * Delete credentials securely
   */
  async deleteCredential(id: string): Promise<boolean> {
    this.validateId(id);

    try {
      let deleted = false;

      switch (this.config.encryptionMethod) {
        case 'dpapi':
          deleted = await this.deleteDPAPI(id);
          break;
        case 'credential-manager':
          deleted = await this.deleteCredentialManager(id);
          break;
        case 'hybrid':
          deleted = await this.deleteHybrid(id);
          break;
      }

      this.auditLog('DELETE', id, deleted ? 'SUCCESS' : 'NOT_FOUND');
      return deleted;
    } catch (error) {
      this.auditLog('DELETE', id, 'FAILED', error.message);
      throw new Error(`Failed to delete credential '${id}': ${error.message}`);
    }
  }

  /**
   * List all stored credential IDs (metadata only)
   */
  async listCredentials(): Promise<string[]> {
    try {
      let credentialIds: string[] = [];

      switch (this.config.encryptionMethod) {
        case 'dpapi':
          credentialIds = await this.listDPAPI();
          break;
        case 'credential-manager':
          credentialIds = await this.listCredentialManager();
          break;
        case 'hybrid':
          credentialIds = await this.listHybrid();
          break;
      }

      this.auditLog('LIST', 'ALL', 'SUCCESS', `Found ${credentialIds.length} credentials`);
      return credentialIds;
    } catch (error) {
      this.auditLog('LIST', 'ALL', 'FAILED', error.message);
      throw new Error(`Failed to list credentials: ${error.message}`);
    }
  }

  /**
   * Validate that the current user can access stored credentials
   * This helps ensure SID-based protection is working
   */
  async validateAccess(): Promise<{ isValid: boolean; currentUser: string; errors: string[] }> {
    const errors: string[] = [];
    let currentUser = 'unknown';

    try {
      if (this.isWindows) {
        currentUser = this.getCurrentWindowsUser();
      } else {
        currentUser = process.env.USER || 'unknown';
        errors.push('Not running on Windows - DPAPI not available');
      }

      // Test encryption/decryption with a temporary credential
      const testId = `__test_access_${Date.now()}`;
      const testData = {
        username: 'test_user',
        password: 'test_password_123',
        additionalData: { testKey: 'testValue' }
      };

      try {
        await this.storeCredential(testId, testData);
        const retrieved = await this.retrieveCredential(testId);
        
        if (!retrieved || retrieved.password !== testData.password) {
          errors.push('Encryption/decryption test failed');
        }
        
        await this.deleteCredential(testId);
      } catch (error) {
        errors.push(`Access validation test failed: ${error.message}`);
      }

      const isValid = errors.length === 0;
      this.auditLog('VALIDATE_ACCESS', currentUser, isValid ? 'SUCCESS' : 'FAILED', errors.join('; '));

      return { isValid, currentUser, errors };
    } catch (error) {
      errors.push(`Validation error: ${error.message}`);
      return { isValid: false, currentUser, errors };
    }
  }

  /**
   * Store using Windows DPAPI (Data Protection API)
   */
  private async storeDPAPI(id: string, entry: CredentialEntry): Promise<void> {
    if (!this.isWindows) {
      throw new Error('DPAPI is only available on Windows');
    }

    const encryptedData = await this.encryptWithDPAPI(JSON.stringify(entry));
    const filePath = join(this.storageDir, `${id}.dpapi`);
    writeFileSync(filePath, encryptedData);
  }

  /**
   * Retrieve using Windows DPAPI
   */
  private async retrieveDPAPI(id: string): Promise<CredentialEntry | null> {
    if (!this.isWindows) {
      throw new Error('DPAPI is only available on Windows');
    }

    const filePath = join(this.storageDir, `${id}.dpapi`);
    if (!existsSync(filePath)) {
      return null;
    }

    const encryptedData = readFileSync(filePath);
    const decryptedJson = await this.decryptWithDPAPI(encryptedData);
    return JSON.parse(decryptedJson);
  }

  /**
   * Delete using Windows DPAPI
   */
  private async deleteDPAPI(id: string): Promise<boolean> {
    const filePath = join(this.storageDir, `${id}.dpapi`);
    if (existsSync(filePath)) {
      try {
        require('fs').unlinkSync(filePath);
        return true;
      } catch {
        return false;
      }
    }
    return false;
  }

  /**
   * List using Windows DPAPI
   */
  private async listDPAPI(): Promise<string[]> {
    const files = require('fs').readdirSync(this.storageDir);
    return files
      .filter((file: string) => file.endsWith('.dpapi'))
      .map((file: string) => file.replace('.dpapi', ''));
  }

  /**
   * Store using Windows Credential Manager
   */
  private async storeCredentialManager(id: string, entry: CredentialEntry): Promise<void> {
    if (!this.isWindows) {
      throw new Error('Windows Credential Manager is only available on Windows');
    }

    const targetName = `${this.config.applicationName}:${id}`;
    const credentialData = JSON.stringify(entry);

    // Use PowerShell to interact with Credential Manager
    const psScript = `
      $credential = New-Object System.Management.Automation.PSCredential("${entry.username || 'luna-user'}", (ConvertTo-SecureString "${credentialData}" -AsPlainText -Force))
      $target = "${targetName}"
      
      # Store in Windows Credential Manager
      cmdkey /generic:"$target" /user:"$($credential.UserName)" /pass:"${credentialData}"
    `;

    await this.executePowerShell(psScript);
  }

  /**
   * Retrieve using Windows Credential Manager
   */
  private async retrieveCredentialManager(id: string): Promise<CredentialEntry | null> {
    if (!this.isWindows) {
      throw new Error('Windows Credential Manager is only available on Windows');
    }

    const targetName = `${this.config.applicationName}:${id}`;
    
    try {
      const psScript = `
        $target = "${targetName}"
        $cred = cmdkey /list:"$target"
        if ($LASTEXITCODE -eq 0) {
          # Credential exists, retrieve it
          $credText = cmdkey /list:"$target" | Out-String
          Write-Output $credText
        } else {
          Write-Output ""
        }
      `;

      const result = await this.executePowerShell(psScript);
      if (result.trim()) {
        // Parse the credential data (this is simplified - real implementation would be more robust)
        const credentialData = result.split('\n').find(line => line.includes('Password:'))?.split('Password:')[1]?.trim();
        if (credentialData) {
          return JSON.parse(credentialData);
        }
      }
      return null;
    } catch {
      return null;
    }
  }

  /**
   * Delete using Windows Credential Manager
   */
  private async deleteCredentialManager(id: string): Promise<boolean> {
    if (!this.isWindows) {
      return false;
    }

    const targetName = `${this.config.applicationName}:${id}`;
    
    try {
      const psScript = `cmdkey /delete:"${targetName}"`;
      await this.executePowerShell(psScript);
      return true;
    } catch {
      return false;
    }
  }

  /**
   * List using Windows Credential Manager
   */
  private async listCredentialManager(): Promise<string[]> {
    if (!this.isWindows) {
      return [];
    }

    try {
      const psScript = `cmdkey /list | Select-String "${this.config.applicationName}:"`;
      const result = await this.executePowerShell(psScript);
      
      return result
        .split('\n')
        .filter(line => line.includes(this.config.applicationName))
        .map(line => {
          const match = line.match(new RegExp(`${this.config.applicationName}:(.+)`));
          return match ? match[1].trim() : '';
        })
        .filter(id => id);
    } catch {
      return [];
    }
  }

  /**
   * Hybrid storage: metadata in DPAPI, credentials in Credential Manager
   */
  private async storeHybrid(id: string, entry: CredentialEntry): Promise<void> {
    // Store sensitive data in Credential Manager
    if (entry.password || entry.apiToken) {
      const sensitiveData = {
        password: entry.password,
        apiToken: entry.apiToken
      };
      await this.storeCredentialManager(`${id}_sensitive`, { ...entry, ...sensitiveData });
    }

    // Store metadata in DPAPI
    const metadata = {
      ...entry,
      password: entry.password ? '[PROTECTED]' : undefined,
      apiToken: entry.apiToken ? '[PROTECTED]' : undefined
    };
    await this.storeDPAPI(`${id}_metadata`, metadata);
  }

  /**
   * Retrieve using hybrid method
   */
  private async retrieveHybrid(id: string): Promise<CredentialEntry | null> {
    const metadata = await this.retrieveDPAPI(`${id}_metadata`);
    if (!metadata) return null;

    const sensitiveData = await this.retrieveCredentialManager(`${id}_sensitive`);
    
    return {
      ...metadata,
      password: sensitiveData?.password || metadata.password,
      apiToken: sensitiveData?.apiToken || metadata.apiToken
    };
  }

  /**
   * Delete using hybrid method
   */
  private async deleteHybrid(id: string): Promise<boolean> {
    const metadataDeleted = await this.deleteDPAPI(`${id}_metadata`);
    const sensitiveDeleted = await this.deleteCredentialManager(`${id}_sensitive`);
    return metadataDeleted || sensitiveDeleted;
  }

  /**
   * List using hybrid method
   */
  private async listHybrid(): Promise<string[]> {
    const metadataIds = await this.listDPAPI();
    return metadataIds
      .filter(id => id.endsWith('_metadata'))
      .map(id => id.replace('_metadata', ''));
  }

  /**
   * Encrypt data using Windows DPAPI
   */
  private async encryptWithDPAPI(data: string): Promise<Buffer> {
    if (!this.isWindows) {
      // Fallback encryption for non-Windows platforms (for testing)
      return this.encryptFallback(data);
    }

    const psScript = `
      Add-Type -AssemblyName System.Security
      $plainText = @"
${data}
"@
      $plainTextBytes = [System.Text.Encoding]::UTF8.GetBytes($plainText)
      $encryptedBytes = [System.Security.Cryptography.ProtectedData]::Protect($plainTextBytes, $null, [System.Security.Cryptography.DataProtectionScope]::CurrentUser)
      [System.Convert]::ToBase64String($encryptedBytes)
    `;

    const result = await this.executePowerShell(psScript);
    return Buffer.from(result.trim(), 'base64');
  }

  /**
   * Decrypt data using Windows DPAPI
   */
  private async decryptWithDPAPI(encryptedData: Buffer): Promise<string> {
    if (!this.isWindows) {
      // Fallback decryption for non-Windows platforms (for testing)
      return this.decryptFallback(encryptedData);
    }

    const base64Data = encryptedData.toString('base64');
    const psScript = `
      Add-Type -AssemblyName System.Security
      $encryptedBytes = [System.Convert]::FromBase64String("${base64Data}")
      $decryptedBytes = [System.Security.Cryptography.ProtectedData]::Unprotect($encryptedBytes, $null, [System.Security.Cryptography.DataProtectionScope]::CurrentUser)
      [System.Text.Encoding]::UTF8.GetString($decryptedBytes)
    `;

    const result = await this.executePowerShell(psScript);
    return result.trim();
  }

  /**
   * Fallback encryption for non-Windows platforms (testing only)
   */
  private encryptFallback(data: string): Buffer {
    const password = this.getCurrentUserSalt();
    const cipher = createCipher('aes-256-cbc', password);
    let encrypted = cipher.update(data, 'utf8', 'hex');
    encrypted += cipher.final('hex');
    return Buffer.from(encrypted, 'hex');
  }

  /**
   * Fallback decryption for non-Windows platforms (testing only)
   */
  private decryptFallback(encryptedData: Buffer): string {
    const password = this.getCurrentUserSalt();
    const decipher = createDecipher('aes-256-cbc', password);
    let decrypted = decipher.update(encryptedData.toString('hex'), 'hex', 'utf8');
    decrypted += decipher.final('utf8');
    return decrypted;
  }

  /**
   * Execute PowerShell script
   */
  private async executePowerShell(script: string): Promise<string> {
    return new Promise((resolve, reject) => {
      const child = spawn('powershell.exe', ['-Command', script], {
        stdio: ['pipe', 'pipe', 'pipe']
      });

      let output = '';
      let error = '';

      child.stdout.on('data', (data) => {
        output += data.toString();
      });

      child.stderr.on('data', (data) => {
        error += data.toString();
      });

      child.on('close', (code) => {
        if (code === 0) {
          resolve(output);
        } else {
          reject(new Error(`PowerShell error: ${error}`));
        }
      });
    });
  }

  /**
   * Get current Windows user
   */
  private getCurrentWindowsUser(): string {
    try {
      if (this.isWindows) {
        return execSync('whoami', { encoding: 'utf8' }).trim();
      }
      return process.env.USER || 'unknown';
    } catch {
      return 'unknown';
    }
  }

  /**
   * Get user-specific salt for fallback encryption
   */
  private getCurrentUserSalt(): string {
    const user = this.getCurrentWindowsUser();
    const hostname = require('os').hostname();
    return `${user}_${hostname}_luna_salt`;
  }

  /**
   * Validate credential ID
   */
  private validateId(id: string): void {
    if (!id || typeof id !== 'string') {
      throw new Error('Credential ID must be a non-empty string');
    }
    if (!/^[a-zA-Z0-9_-]+$/.test(id)) {
      throw new Error('Credential ID must contain only alphanumeric characters, underscores, and hyphens');
    }
  }

  /**
   * Ensure storage directory exists
   */
  private ensureStorageDirectory(): void {
    if (!existsSync(this.storageDir)) {
      mkdirSync(this.storageDir, { recursive: true });
    }
  }

  /**
   * Audit log for security tracking
   */
  private auditLog(operation: string, credentialId: string, status: string, details?: string): void {
    if (!this.config.enableAuditLog) return;

    const timestamp = new Date().toISOString();
    const user = this.getCurrentWindowsUser();
    const logEntry = `${timestamp} | ${user} | ${operation} | ${credentialId} | ${status}${details ? ` | ${details}` : ''}\n`;

    try {
      require('fs').appendFileSync(this.auditLogPath, logEntry);
    } catch {
      // Silently fail on audit log write errors
    }
  }
}