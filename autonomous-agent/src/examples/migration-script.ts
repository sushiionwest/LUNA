#!/usr/bin/env ts-node

/**
 * Luna Credential Migration Script
 * 
 * This script migrates credentials from .env files to SecureCredentialStorage.
 * Run with: npm run migrate-credentials
 */

import { SecureCredentialStorage } from '../services/SecureCredentialStorage';
import { CredentialMigration } from '../services/CredentialMigration';
import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join } from 'path';

async function main() {
  console.log('🔐 Luna Credential Migration Tool\n');

  // Initialize secure storage
  const secureStorage = new SecureCredentialStorage({
    applicationName: 'LunaAgent',
    encryptionMethod: 'hybrid',
    enableAuditLog: true
  });

  // Validate that secure storage is working
  console.log('Validating secure storage...');
  const validation = await secureStorage.validateAccess();
  
  if (!validation.isValid) {
    console.error('❌ Secure storage validation failed:');
    validation.errors.forEach(error => console.error(`  - ${error}`));
    process.exit(1);
  }
  
  console.log(`✅ Secure storage validated (user: ${validation.currentUser})\n`);

  // Check for .env file
  const envPath = '.env';
  if (!existsSync(envPath)) {
    console.log('📝 No .env file found. Creating example .env file...');
    createExampleEnvFile();
    console.log('Example .env file created. Please add your credentials and run this script again.');
    return;
  }

  // Initialize migration
  const migration = new CredentialMigration({
    envFilePath: envPath,
    secureStorage,
    migratePatterns: [
      'API_KEY',
      'TOKEN',
      'PASSWORD',
      'SECRET',
      'PRIVATE_KEY',
      'DATABASE_URL',
      'WEBHOOK_SECRET',
      'OAUTH_CLIENT_SECRET',
      'JWT_SECRET',
      'ENCRYPTION_KEY',
      'SERVICE_ACCOUNT_KEY'
    ],
    excludePatterns: [
      'NODE_ENV',
      'PORT',
      'DEBUG',
      'LOG_LEVEL',
      'HOST',
      'HOSTNAME'
    ]
  });

  try {
    console.log('🔄 Starting credential migration...');
    
    // Perform migration
    const result = await migration.migrateFromEnvFile();
    
    console.log('\n📊 Migration Results:');
    console.log(`  ✅ Migrated: ${result.migratedCount} credentials`);
    console.log(`  ⏭️  Skipped: ${result.skippedCount} variables`);
    console.log(`  ❌ Errors: ${result.errors.length}`);
    
    if (result.backupPath) {
      console.log(`  💾 Backup: ${result.backupPath}`);
    }

    if (result.migratedCredentials.length > 0) {
      console.log('\n🔑 Migrated Credentials:');
      result.migratedCredentials.forEach(cred => console.log(`  - ${cred}`));
    }

    if (result.errors.length > 0) {
      console.log('\n⚠️  Migration Errors:');
      result.errors.forEach(error => console.error(`  - ${error}`));
    }

    // Validate migration
    if (result.migratedCredentials.length > 0) {
      console.log('\n🔍 Validating migration...');
      const validationResult = await migration.validateMigration(result.migratedCredentials);
      
      if (validationResult.isValid) {
        console.log('✅ Migration validation successful');
      } else {
        console.log('❌ Migration validation failed:');
        validationResult.errors.forEach(error => console.error(`  - ${error}`));
      }
    }

    // Generate report
    const report = migration.generateMigrationReport(result);
    const reportPath = 'credential-migration-report.md';
    writeFileSync(reportPath, report);
    console.log(`\n📄 Migration report saved to: ${reportPath}`);

    // Show next steps
    console.log('\n🚀 Next Steps:');
    console.log('1. Review the new .env file - sensitive values are now marked as migrated');
    console.log('2. Update your application code to use SecureCredentialStorage');
    console.log('3. Test your application thoroughly');
    console.log('4. Remove the backup file after confirming everything works');
    console.log('\nExample usage:');
    console.log('```typescript');
    console.log('import { secureStorage } from "./src/services/SecureCredentialStorage";');
    console.log('const credential = await secureStorage.retrieveCredential("api-key");');
    console.log('```');

  } catch (error) {
    console.error('❌ Migration failed:', error.message);
    process.exit(1);
  }
}

function createExampleEnvFile() {
  const exampleEnv = `# Example .env file for Luna Agent
# Replace these with your actual credentials

# Development settings (these won't be migrated)
NODE_ENV=development
PORT=3000
LOG_LEVEL=info

# API credentials (these will be migrated to secure storage)
OPENAI_API_KEY=sk-your-openai-api-key-here
ANTHROPIC_API_KEY=sk-ant-your-anthropic-key-here
GOOGLE_API_KEY=your-google-api-key
AZURE_COGNITIVE_SERVICES_KEY=your-azure-key

# Database credentials (these will be migrated)
DATABASE_URL=postgresql://username:password@localhost:5432/luna
DATABASE_PASSWORD=your-database-password

# OAuth secrets (these will be migrated)
OAUTH_CLIENT_SECRET=your-oauth-client-secret
GITHUB_OAUTH_TOKEN=ghp_your-github-token
GOOGLE_OAUTH_CLIENT_SECRET=your-google-oauth-secret

# Other secrets (these will be migrated)
JWT_SECRET=your-jwt-secret-key
ENCRYPTION_KEY=your-encryption-key
WEBHOOK_SECRET=your-webhook-secret

# External service tokens (these will be migrated)
STRIPE_SECRET_KEY=sk_test_your-stripe-key
SENDGRID_API_KEY=SG.your-sendgrid-key
SLACK_BOT_TOKEN=xoxb-your-slack-token
`;

  writeFileSync('.env', exampleEnv);
}

// Handle CLI arguments
const args = process.argv.slice(2);

if (args.includes('--help') || args.includes('-h')) {
  console.log(`
Luna Credential Migration Tool

Usage: npm run migrate-credentials [options]

Options:
  --help, -h     Show this help message
  --dry-run      Show what would be migrated without actually doing it
  --backup-only  Create backup without migrating

Examples:
  npm run migrate-credentials
  npm run migrate-credentials -- --dry-run
`);
  process.exit(0);
}

if (args.includes('--dry-run')) {
  console.log('🧪 DRY RUN MODE - No changes will be made\n');
  // TODO: Implement dry run mode
}

// Run the migration
main().catch(error => {
  console.error('Migration script failed:', error);
  process.exit(1);
});