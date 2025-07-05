import { chromium, FullConfig } from '@playwright/test';

/**
 * Global setup for accessibility tests
 * Prepares the test environment and Luna Agent
 */
async function globalSetup(config: FullConfig) {
  console.log('🔍 Setting up accessibility testing environment...');

  // Launch browser for setup
  const browser = await chromium.launch();
  const page = await browser.newPage();

  try {
    // Wait for Luna Agent to be ready
    const baseURL = config.projects[0]?.use?.baseURL || 'http://localhost:3000';
    
    console.log(`Waiting for Luna Agent at ${baseURL}...`);
    await page.goto(baseURL, { waitUntil: 'networkidle' });

    // Check that the application is ready
    const healthCheck = await page.request.get(`${baseURL}/health`);
    if (!healthCheck.ok()) {
      throw new Error(`Luna Agent health check failed: ${healthCheck.status()}`);
    }

    // Initialize accessibility settings for testing
    await page.evaluate(() => {
      // Set up test-friendly accessibility settings
      const testSettings = {
        enabled: true,
        showPreview: true,
        previewDelay: 1000, // Shorter for tests
        showTrail: true,
        showDescription: true,
        opacity: 0.9,
        sound: false, // Disabled for testing
        pauseOnHover: true,
        allowUserCancel: true
      };

      window.localStorage.setItem('luna-accessibility-settings', JSON.stringify(testSettings));
      
      // Also set up test mode flag
      window.localStorage.setItem('luna-test-mode', 'true');

      // Add test helper functions to window
      (window as any).lunaTestHelpers = {
        triggerAction: (action: any) => {
          window.dispatchEvent(new CustomEvent('luna:action:preview', { detail: action }));
        },
        updateAction: (id: string, status: string) => {
          window.dispatchEvent(new CustomEvent('luna:action:update', { detail: { id, status } }));
        },
        cancelAction: (id: string) => {
          window.dispatchEvent(new CustomEvent('luna:action:cancel', { detail: { id } }));
        },
        executeAction: (id: string) => {
          window.dispatchEvent(new CustomEvent('luna:action:execute', { detail: { id } }));
        }
      };
    });

    // Create test data directory
    const fs = require('fs');
    const path = require('path');
    
    const testDataDir = path.join(__dirname, 'test-data');
    if (!fs.existsSync(testDataDir)) {
      fs.mkdirSync(testDataDir, { recursive: true });
    }

    // Create baseline screenshots directory
    const baselineDir = path.join(__dirname, 'tests', 'accessibility-overlay.spec.ts-snapshots');
    if (!fs.existsSync(baselineDir)) {
      fs.mkdirSync(baselineDir, { recursive: true });
    }

    console.log('✅ Accessibility testing environment ready');

  } catch (error) {
    console.error('❌ Failed to setup accessibility testing environment:', error);
    throw error;
  } finally {
    await browser.close();
  }
}

export default globalSetup;