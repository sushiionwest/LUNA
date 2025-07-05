// Jest setup file for SecureCredentialStorage tests

// Increase timeout for credential operations
jest.setTimeout(30000);

// Mock Windows-specific modules when not on Windows
if (process.platform !== 'win32') {
  jest.mock('node-dpapi', () => ({
    protectData: jest.fn().mockResolvedValue(Buffer.from('mock-encrypted-data')),
    unprotectData: jest.fn().mockResolvedValue(Buffer.from('mock-decrypted-data'))
  }));

  jest.mock('keytar', () => ({
    setPassword: jest.fn().mockResolvedValue(),
    getPassword: jest.fn().mockResolvedValue('mock-password'),
    deletePassword: jest.fn().mockResolvedValue(true),
    findCredentials: jest.fn().mockResolvedValue([])
  }));
}

// Global test utilities
global.isWindows = process.platform === 'win32';

// Cleanup function for tests
global.cleanupTestStorage = (storageDir) => {
  const fs = require('fs');
  if (fs.existsSync(storageDir)) {
    fs.rmSync(storageDir, { recursive: true, force: true });
  }
};