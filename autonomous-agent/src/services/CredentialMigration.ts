import { SecureCredentialStorage, CredentialEntry } from './SecureCredentialStorage';
import { readFileSync, writeFileSync, existsSync, renameSync } from 'fs';
import { join } from 'path';

export interface MigrationConfig {
  envFilePath?: string;
  backupLocation?: string;
  secureStorage: SecureCredentialStorage;
  migratePatterns: string[];
  excludePatterns?: string[];
}

export interface MigrationResult {
  migratedCount: number;
  skippedCount: number;
  errors: string[];
  migratedCredentials: string[];
  backupPath?: string;
}

/**
 * Service to migrate credentials from .env files and other insecure storage
 * to the SecureCredentialStorage system
 */
export class CredentialMigration {
  private config: MigrationConfig;

  constructor(config: MigrationConfig) {
    this.config = {
      envFilePath: '.env',
      backupLocation: '.env.backup',
      excludePatterns: [],
      ...config
    };
  }

  /**
   * Migrate credentials from .env file to secure storage
   */
  async migrateFromEnvFile(): Promise<MigrationResult> {
    const result: MigrationResult = {
      migratedCount: 0,
      skippedCount: 0,
      errors: [],
      migratedCredentials: []
    };

    try {
      if (!existsSync(this.config.envFilePath!)) {
        result.errors.push(`Environment file not found: ${this.config.envFilePath}`);
        return result;
      }

      // Create backup
      const backupPath = this.createBackup();
      result.backupPath = backupPath;

      // Parse .env file
      const envContent = readFileSync(this.config.envFilePath!, 'utf8');
      const envVars = this.parseEnvFile(envContent);

      // Identify credentials to migrate
      const credentialsToMigrate = this.identifyCredentials(envVars);

      // Migrate each credential
      for (const [key, value] of credentialsToMigrate) {
        try {
          await this.migrateCredential(key, value);
          result.migratedCount++;
          result.migratedCredentials.push(key);
        } catch (error) {
          result.errors.push(`Failed to migrate ${key}: ${error.message}`);
          result.skippedCount++;
        }
      }

      // Generate new .env file with references to secure storage
      if (result.migratedCount > 0) {
        this.generateSecureEnvFile(envVars, credentialsToMigrate);
      }

      return result;

    } catch (error) {
      result.errors.push(`Migration failed: ${error.message}`);
      return result;
    }
  }

  /**
   * Migrate from other credential sources (config files, databases, etc.)
   */
  async migrateFromSource(source: Record<string, any>, sourceType: 'json' | 'yaml' | 'ini'): Promise<MigrationResult> {
    const result: MigrationResult = {
      migratedCount: 0,
      skippedCount: 0,
      errors: [],
      migratedCredentials: []
    };

    try {
      const credentials = this.extractCredentialsFromSource(source, sourceType);

      for (const [key, credentialData] of credentials) {
        try {
          await this.config.secureStorage.storeCredential(key, credentialData);
          result.migratedCount++;
          result.migratedCredentials.push(key);
        } catch (error) {
          result.errors.push(`Failed to migrate ${key}: ${error.message}`);
          result.skippedCount++;
        }
      }

      return result;

    } catch (error) {
      result.errors.push(`Source migration failed: ${error.message}`);
      return result;
    }
  }

  /**
   * Validate that migrated credentials work correctly
   */
  async validateMigration(migratedCredentials: string[]): Promise<{ isValid: boolean; errors: string[] }> {
    const errors: string[] = [];

    for (const credentialId of migratedCredentials) {
      try {
        const credential = await this.config.secureStorage.retrieveCredential(credentialId);
        if (!credential) {
          errors.push(`Migrated credential '${credentialId}' cannot be retrieved`);
        } else if (!this.validateCredentialStructure(credential)) {
          errors.push(`Migrated credential '${credentialId}' has invalid structure`);
        }
      } catch (error) {
        errors.push(`Failed to validate '${credentialId}': ${error.message}`);
      }
    }

    return {
      isValid: errors.length === 0,
      errors
    };
  }

  /**
   * Rollback migration (restore from backup)
   */
  async rollbackMigration(backupPath: string): Promise<boolean> {
    try {
      if (!existsSync(backupPath)) {
        throw new Error(`Backup file not found: ${backupPath}`);
      }

      // Restore original .env file
      renameSync(backupPath, this.config.envFilePath!);

      // Optionally remove migrated credentials from secure storage
      // (This is commented out to be safe - manual cleanup may be preferred)
      /*
      const migratedCredentials = await this.config.secureStorage.listCredentials();
      for (const credentialId of migratedCredentials) {
        await this.config.secureStorage.deleteCredential(credentialId);
      }
      */

      return true;
    } catch (error) {
      console.error(`Rollback failed: ${error.message}`);
      return false;
    }
  }

  /**
   * Generate migration report
   */
  generateMigrationReport(result: MigrationResult): string {
    const report = [
      '# Credential Migration Report',
      '',
      `**Migration Date:** ${new Date().toISOString()}`,
      `**Migrated Credentials:** ${result.migratedCount}`,
      `**Skipped Credentials:** ${result.skippedCount}`,
      `**Backup Location:** ${result.backupPath || 'N/A'}`,
      '',
      '## Migrated Credentials',
      ...result.migratedCredentials.map(cred => `- ${cred}`),
      ''
    ];

    if (result.errors.length > 0) {
      report.push(
        '## Errors',
        ...result.errors.map(error => `- ${error}`),
        ''
      );
    }

    report.push(
      '## Next Steps',
      '1. Test your application to ensure all credentials are working',
      '2. Update your application code to use SecureCredentialStorage.retrieveCredential()',
      '3. Remove the backup file after confirming everything works',
      '4. Update documentation and deployment processes',
      ''
    );

    return report.join('\n');
  }

  /**
   * Create backup of original .env file
   */
  private createBackup(): string {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const backupPath = `${this.config.backupLocation}.${timestamp}`;
    
    renameSync(this.config.envFilePath!, backupPath);
    return backupPath;
  }

  /**
   * Parse .env file content
   */
  private parseEnvFile(content: string): Map<string, string> {
    const envVars = new Map<string, string>();
    const lines = content.split('\n');

    for (const line of lines) {
      const trimmed = line.trim();
      
      // Skip comments and empty lines
      if (!trimmed || trimmed.startsWith('#')) {
        continue;
      }

      const equalIndex = trimmed.indexOf('=');
      if (equalIndex === -1) {
        continue;
      }

      const key = trimmed.substring(0, equalIndex).trim();
      let value = trimmed.substring(equalIndex + 1).trim();

      // Remove quotes if present
      if ((value.startsWith('"') && value.endsWith('"')) || 
          (value.startsWith("'") && value.endsWith("'"))) {
        value = value.slice(1, -1);
      }

      envVars.set(key, value);
    }

    return envVars;
  }

  /**
   * Identify which environment variables are credentials
   */
  private identifyCredentials(envVars: Map<string, string>): Map<string, string> {
    const credentials = new Map<string, string>();

    for (const [key, value] of envVars) {
      // Skip if in exclude patterns
      if (this.config.excludePatterns?.some(pattern => key.includes(pattern))) {
        continue;
      }

      // Check if key matches any migration patterns
      const shouldMigrate = this.config.migratePatterns.some(pattern => {
        const regex = new RegExp(pattern, 'i');
        return regex.test(key);
      });

      if (shouldMigrate) {
        credentials.set(key, value);
      }
    }

    return credentials;
  }

  /**
   * Migrate a single credential
   */
  private async migrateCredential(key: string, value: string): Promise<void> {
    const credentialData = this.parseCredentialValue(key, value);
    const credentialId = this.generateCredentialId(key);
    
    await this.config.secureStorage.storeCredential(credentialId, credentialData);
  }

  /**
   * Parse credential value and determine its type
   */
  private parseCredentialValue(key: string, value: string): Omit<CredentialEntry, 'id' | 'createdAt' | 'lastAccessed'> {
    const keyLower = key.toLowerCase();
    
    // Determine credential type based on key name
    if (keyLower.includes('token') || keyLower.includes('api_key') || keyLower.includes('apikey')) {
      return {
        apiToken: value,
        additionalData: { originalKey: key }
      };
    } else if (keyLower.includes('password') || keyLower.includes('pass') || keyLower.includes('secret')) {
      return {
        password: value,
        additionalData: { originalKey: key }
      };
    } else if (keyLower.includes('user') || keyLower.includes('username') || keyLower.includes('login')) {
      return {
        username: value,
        additionalData: { originalKey: key }
      };
    } else {
      // Generic credential
      return {
        additionalData: { 
          originalKey: key,
          value: value 
        }
      };
    }
  }

  /**
   * Generate secure credential ID from environment variable key
   */
  private generateCredentialId(key: string): string {
    return key.toLowerCase()
      .replace(/_/g, '-')
      .replace(/[^a-z0-9-]/g, '')
      .replace(/-+/g, '-')
      .replace(/^-|-$/g, '');
  }

  /**
   * Generate new .env file with references to secure storage
   */
  private generateSecureEnvFile(originalVars: Map<string, string>, migratedVars: Map<string, string>): void {
    const lines: string[] = [
      '# Environment Configuration',
      '# Sensitive credentials have been migrated to SecureCredentialStorage',
      '# Generated by Luna Credential Migration',
      `# Migration Date: ${new Date().toISOString()}`,
      ''
    ];

    for (const [key, value] of originalVars) {
      if (migratedVars.has(key)) {
        const credentialId = this.generateCredentialId(key);
        lines.push(
          `# ${key} - MIGRATED to secure storage as '${credentialId}'`,
          `# Use: SecureCredentialStorage.retrieveCredential('${credentialId}')`,
          `${key}=__MIGRATED_TO_SECURE_STORAGE__`
        );
      } else {
        lines.push(`${key}=${value}`);
      }
      lines.push('');
    }

    lines.push(
      '# Migration Notes:',
      '# - Credentials marked as __MIGRATED_TO_SECURE_STORAGE__ are now stored securely',
      '# - Update your application code to use SecureCredentialStorage for these values',
      '# - Remove this file after migration is complete and tested'
    );

    writeFileSync(this.config.envFilePath!, lines.join('\n'));
  }

  /**
   * Extract credentials from other source formats
   */
  private extractCredentialsFromSource(source: Record<string, any>, sourceType: string): Map<string, Omit<CredentialEntry, 'id' | 'createdAt' | 'lastAccessed'>> {
    const credentials = new Map();

    const traverse = (obj: any, prefix = ''): void => {
      for (const [key, value] of Object.entries(obj)) {
        const fullKey = prefix ? `${prefix}-${key}` : key;
        
        if (typeof value === 'object' && value !== null) {
          traverse(value, fullKey);
        } else if (typeof value === 'string') {
          // Check if this looks like a credential
          const shouldMigrate = this.config.migratePatterns.some(pattern => {
            const regex = new RegExp(pattern, 'i');
            return regex.test(key) || regex.test(fullKey);
          });

          if (shouldMigrate) {
            credentials.set(fullKey, this.parseCredentialValue(key, value));
          }
        }
      }
    };

    traverse(source);
    return credentials;
  }

  /**
   * Validate credential structure
   */
  private validateCredentialStructure(credential: CredentialEntry): boolean {
    return (
      credential.id &&
      credential.createdAt instanceof Date &&
      credential.lastAccessed instanceof Date &&
      (credential.username || credential.password || credential.apiToken || credential.additionalData)
    );
  }
}