/**
 * Example usage of SecureCredentialStorage in Luna Agent
 * This demonstrates how to replace .env patterns with secure credential storage
 */

import { SecureCredentialStorage } from '../services/SecureCredentialStorage';
import { CredentialMigration } from '../services/CredentialMigration';

// Initialize secure credential storage
const secureStorage = new SecureCredentialStorage({
  applicationName: 'LunaAgent',
  encryptionMethod: 'hybrid', // Use both DPAPI and Credential Manager
  enableAuditLog: true
});

/**
 * Example 1: Migrating from existing .env file
 */
export async function migrateExistingCredentials(): Promise<void> {
  console.log('Starting credential migration...');

  const migration = new CredentialMigration({
    envFilePath: '.env',
    secureStorage,
    migratePatterns: [
      'API_KEY',
      'TOKEN',
      'PASSWORD',
      'SECRET',
      'PRIVATE_KEY',
      'DATABASE_URL',
      'WEBHOOK_SECRET',
      'OAUTH_',
      'JWT_SECRET'
    ],
    excludePatterns: [
      'NODE_ENV',
      'PORT',
      'DEBUG',
      'LOG_LEVEL'
    ]
  });

  try {
    const result = await migration.migrateFromEnvFile();
    
    console.log(`Migration completed:`);
    console.log(`- Migrated: ${result.migratedCount} credentials`);
    console.log(`- Skipped: ${result.skippedCount} variables`);
    console.log(`- Errors: ${result.errors.length}`);
    
    if (result.errors.length > 0) {
      console.error('Migration errors:', result.errors);
    }

    // Generate migration report
    const report = migration.generateMigrationReport(result);
    console.log('\n--- Migration Report ---');
    console.log(report);

    // Validate migration
    const validation = await migration.validateMigration(result.migratedCredentials);
    if (!validation.isValid) {
      console.error('Migration validation failed:', validation.errors);
    } else {
      console.log('✅ Migration validation successful');
    }

  } catch (error) {
    console.error('Migration failed:', error);
  }
}

/**
 * Example 2: Storing new credentials securely
 */
export async function storeApiCredentials(): Promise<void> {
  try {
    // Store OpenAI API credentials
    await secureStorage.storeCredential('openai-api', {
      apiToken: 'sk-your-openai-api-key-here',
      additionalData: {
        endpoint: 'https://api.openai.com/v1',
        model: 'gpt-4',
        organization: 'org-your-org-id'
      }
    });

    // Store database credentials
    await secureStorage.storeCredential('database-prod', {
      username: 'luna_user',
      password: 'super_secure_password_123',
      additionalData: {
        host: 'prod-db.company.com',
        port: '5432',
        database: 'luna_production',
        ssl: 'true'
      }
    });

    // Store OAuth credentials
    await secureStorage.storeCredential('google-oauth', {
      additionalData: {
        clientId: 'your-client-id.apps.googleusercontent.com',
        clientSecret: 'your-client-secret',
        redirectUri: 'http://localhost:3000/auth/callback',
        scopes: 'profile email'
      }
    });

    console.log('✅ API credentials stored securely');

  } catch (error) {
    console.error('Failed to store credentials:', error);
  }
}

/**
 * Example 3: Retrieving and using credentials in your application
 */
export class LunaConfigManager {
  private storage: SecureCredentialStorage;

  constructor() {
    this.storage = secureStorage;
  }

  /**
   * Get OpenAI configuration
   */
  async getOpenAIConfig(): Promise<{ apiKey: string; endpoint: string; model: string; organization?: string } | null> {
    try {
      const credential = await this.storage.retrieveCredential('openai-api');
      if (!credential?.apiToken) {
        return null;
      }

      return {
        apiKey: credential.apiToken,
        endpoint: credential.additionalData?.endpoint || 'https://api.openai.com/v1',
        model: credential.additionalData?.model || 'gpt-4',
        organization: credential.additionalData?.organization
      };
    } catch (error) {
      console.error('Failed to retrieve OpenAI config:', error);
      return null;
    }
  }

  /**
   * Get database connection string
   */
  async getDatabaseUrl(): Promise<string | null> {
    try {
      const credential = await this.storage.retrieveCredential('database-prod');
      if (!credential?.username || !credential?.password) {
        return null;
      }

      const { username, password, additionalData } = credential;
      const host = additionalData?.host || 'localhost';
      const port = additionalData?.port || '5432';
      const database = additionalData?.database || 'luna';
      const ssl = additionalData?.ssl === 'true' ? '?ssl=true' : '';

      return `postgresql://${username}:${password}@${host}:${port}/${database}${ssl}`;
    } catch (error) {
      console.error('Failed to retrieve database config:', error);
      return null;
    }
  }

  /**
   * Get OAuth configuration
   */
  async getOAuthConfig(provider: string): Promise<any | null> {
    try {
      const credential = await this.storage.retrieveCredential(`${provider}-oauth`);
      return credential?.additionalData || null;
    } catch (error) {
      console.error(`Failed to retrieve ${provider} OAuth config:`, error);
      return null;
    }
  }

  /**
   * Update API token (e.g., after refresh)
   */
  async updateApiToken(credentialId: string, newToken: string): Promise<void> {
    try {
      const existing = await this.storage.retrieveCredential(credentialId);
      if (existing) {
        await this.storage.storeCredential(credentialId, {
          ...existing,
          apiToken: newToken
        });
        console.log(`✅ Updated API token for ${credentialId}`);
      }
    } catch (error) {
      console.error(`Failed to update API token for ${credentialId}:`, error);
    }
  }
}

/**
 * Example 4: Integration with Express.js application
 */
export class SecureLunaServer {
  private configManager: LunaConfigManager;

  constructor() {
    this.configManager = new LunaConfigManager();
  }

  async startServer(): Promise<void> {
    // Get database configuration
    const dbUrl = await this.configManager.getDatabaseUrl();
    if (!dbUrl) {
      throw new Error('Database credentials not found. Please run credential migration first.');
    }

    // Get OpenAI configuration
    const openaiConfig = await this.configManager.getOpenAIConfig();
    if (!openaiConfig) {
      console.warn('OpenAI credentials not found. AI features will be disabled.');
    }

    // Initialize your application with secure credentials
    console.log('🚀 Starting Luna Agent with secure credentials...');
    
    // Example: Initialize database connection
    // const db = new DatabaseConnection(dbUrl);
    
    // Example: Initialize AI service
    // if (openaiConfig) {
    //   const aiService = new OpenAIService(openaiConfig);
    // }
    
    console.log('✅ Server started with secure credential management');
  }
}

/**
 * Example 5: Credential rotation and management
 */
export class CredentialRotationManager {
  private storage: SecureCredentialStorage;

  constructor() {
    this.storage = secureStorage;
  }

  /**
   * Rotate API key (automated or manual)
   */
  async rotateApiKey(credentialId: string, newApiKey: string): Promise<void> {
    try {
      const existing = await this.storage.retrieveCredential(credentialId);
      if (!existing) {
        throw new Error(`Credential ${credentialId} not found`);
      }

      // Store old token as backup (with rotation timestamp)
      const backupId = `${credentialId}-backup-${Date.now()}`;
      await this.storage.storeCredential(backupId, {
        ...existing,
        additionalData: {
          ...existing.additionalData,
          rotatedAt: new Date().toISOString(),
          originalId: credentialId
        }
      });

      // Update with new token
      await this.storage.storeCredential(credentialId, {
        ...existing,
        apiToken: newApiKey,
        additionalData: {
          ...existing.additionalData,
          lastRotated: new Date().toISOString(),
          rotationCount: (existing.additionalData?.rotationCount || 0) + 1
        }
      });

      console.log(`✅ Rotated API key for ${credentialId}`);
      
      // Clean up old backups (keep only last 5)
      await this.cleanupOldBackups(credentialId);

    } catch (error) {
      console.error(`Failed to rotate API key for ${credentialId}:`, error);
      throw error;
    }
  }

  /**
   * Audit credential usage
   */
  async auditCredentialUsage(): Promise<void> {
    try {
      const credentials = await this.storage.listCredentials();
      
      console.log('=== Credential Audit Report ===');
      
      for (const credentialId of credentials) {
        const credential = await this.storage.retrieveCredential(credentialId);
        if (credential) {
          const daysSinceAccess = Math.floor(
            (Date.now() - credential.lastAccessed.getTime()) / (1000 * 60 * 60 * 24)
          );
          
          console.log(`${credentialId}:`);
          console.log(`  - Created: ${credential.createdAt.toISOString()}`);
          console.log(`  - Last Accessed: ${credential.lastAccessed.toISOString()} (${daysSinceAccess} days ago)`);
          console.log(`  - Type: ${credential.apiToken ? 'API Token' : credential.password ? 'Password' : 'Other'}`);
          
          if (daysSinceAccess > 30) {
            console.log(`  - ⚠️  Warning: Not accessed in ${daysSinceAccess} days`);
          }
          
          console.log('');
        }
      }
      
    } catch (error) {
      console.error('Failed to audit credentials:', error);
    }
  }

  private async cleanupOldBackups(credentialId: string): Promise<void> {
    const credentials = await this.storage.listCredentials();
    const backups = credentials
      .filter(id => id.startsWith(`${credentialId}-backup-`))
      .sort()
      .reverse(); // Most recent first

    // Keep only the 5 most recent backups
    const toDelete = backups.slice(5);
    
    for (const backupId of toDelete) {
      await this.storage.deleteCredential(backupId);
    }
  }
}

/**
 * Example usage in main application
 */
export async function main(): Promise<void> {
  try {
    // 1. First time setup: migrate existing credentials
    await migrateExistingCredentials();
    
    // 2. Store additional credentials as needed
    await storeApiCredentials();
    
    // 3. Start the secure server
    const server = new SecureLunaServer();
    await server.startServer();
    
    // 4. Set up credential rotation (optional)
    const rotationManager = new CredentialRotationManager();
    
    // Schedule periodic audits
    setInterval(async () => {
      await rotationManager.auditCredentialUsage();
    }, 24 * 60 * 60 * 1000); // Daily audit
    
  } catch (error) {
    console.error('Application startup failed:', error);
    process.exit(1);
  }
}

// Example environment configuration after migration
export const getSecureConfig = async () => {
  const configManager = new LunaConfigManager();
  
  return {
    // Non-sensitive configuration (can stay in .env or config files)
    NODE_ENV: process.env.NODE_ENV || 'development',
    PORT: parseInt(process.env.PORT || '3000'),
    LOG_LEVEL: process.env.LOG_LEVEL || 'info',
    
    // Sensitive configuration (retrieved from secure storage)
    database: await configManager.getDatabaseUrl(),
    openai: await configManager.getOpenAIConfig(),
    oauth: {
      google: await configManager.getOAuthConfig('google'),
      github: await configManager.getOAuthConfig('github')
    }
  };
};

// Export the instances for use in other modules
export { secureStorage };
export const configManager = new LunaConfigManager();
export const rotationManager = new CredentialRotationManager();