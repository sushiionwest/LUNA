import { FullConfig } from '@playwright/test';

/**
 * Global teardown for accessibility tests
 * Cleans up test environment and generates reports
 */
async function globalTeardown(config: FullConfig) {
  console.log('🧹 Cleaning up accessibility testing environment...');

  try {
    const fs = require('fs');
    const path = require('path');

    // Generate accessibility test report
    const testResultsPath = path.join(__dirname, 'test-results', 'results.json');
    const reportPath = path.join(__dirname, 'test-results', 'accessibility-report.html');

    if (fs.existsSync(testResultsPath)) {
      const results = JSON.parse(fs.readFileSync(testResultsPath, 'utf8'));
      const report = generateAccessibilityReport(results);
      fs.writeFileSync(reportPath, report);
      console.log(`📊 Accessibility report generated: ${reportPath}`);
    }

    // Clean up temporary test data (but keep screenshots for debugging)
    const tempDataPath = path.join(__dirname, 'test-data', 'temp');
    if (fs.existsSync(tempDataPath)) {
      fs.rmSync(tempDataPath, { recursive: true, force: true });
    }

    // Archive test screenshots if successful
    const screenshotsPath = path.join(__dirname, 'test-results');
    const archivePath = path.join(__dirname, 'archived-results', new Date().toISOString().split('T')[0]);
    
    if (fs.existsSync(screenshotsPath) && !process.env.CI) {
      if (!fs.existsSync(archivePath)) {
        fs.mkdirSync(archivePath, { recursive: true });
      }
      
      // Copy important files to archive
      const filesToArchive = ['accessibility-report.html', 'results.json'];
      filesToArchive.forEach(file => {
        const sourcePath = path.join(screenshotsPath, file);
        const destPath = path.join(archivePath, file);
        if (fs.existsSync(sourcePath)) {
          fs.copyFileSync(sourcePath, destPath);
        }
      });
    }

    console.log('✅ Accessibility testing cleanup completed');

  } catch (error) {
    console.error('❌ Failed to cleanup accessibility testing environment:', error);
    // Don't throw error in teardown as it would mask test failures
  }
}

function generateAccessibilityReport(results: any): string {
  const { stats, suites } = results;
  const accessibilityTests = suites.find((suite: any) => suite.title === 'Accessibility Overlay');
  
  const passedTests = accessibilityTests?.specs.filter((spec: any) => spec.ok) || [];
  const failedTests = accessibilityTests?.specs.filter((spec: any) => !spec.ok) || [];

  return `
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Luna Accessibility Test Report</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background: #f5f5f5;
        }
        .header {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 30px;
            border-radius: 10px;
            margin-bottom: 30px;
            text-align: center;
        }
        .stats {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        .stat-card {
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            text-align: center;
        }
        .stat-number {
            font-size: 2em;
            font-weight: bold;
            margin-bottom: 5px;
        }
        .stat-label {
            color: #666;
            font-size: 0.9em;
        }
        .success { color: #10b981; }
        .error { color: #ef4444; }
        .warning { color: #f59e0b; }
        .info { color: #3b82f6; }
        .test-results {
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            overflow: hidden;
        }
        .test-category {
            padding: 20px;
            border-bottom: 1px solid #e5e7eb;
        }
        .test-category h3 {
            margin-top: 0;
            color: #1f2937;
        }
        .test-item {
            padding: 15px;
            border-left: 4px solid #e5e7eb;
            margin-bottom: 10px;
            background: #f9fafb;
        }
        .test-item.passed {
            border-left-color: #10b981;
            background: #ecfdf5;
        }
        .test-item.failed {
            border-left-color: #ef4444;
            background: #fef2f2;
        }
        .test-title {
            font-weight: 600;
            margin-bottom: 5px;
        }
        .test-duration {
            font-size: 0.8em;
            color: #666;
        }
        .footer {
            text-align: center;
            padding: 20px;
            color: #666;
            border-top: 1px solid #e5e7eb;
            margin-top: 30px;
        }
        .visual-coverage {
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            margin-bottom: 20px;
        }
        .screenshot-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 15px;
            margin-top: 15px;
        }
        .screenshot-item {
            text-align: center;
            padding: 10px;
            border: 1px solid #e5e7eb;
            border-radius: 6px;
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>🎯 Luna Accessibility Test Report</h1>
        <p>Visual regression testing for accessibility overlay components</p>
        <p><strong>Generated:</strong> ${new Date().toLocaleString()}</p>
    </div>

    <div class="stats">
        <div class="stat-card">
            <div class="stat-number success">${stats.passed || 0}</div>
            <div class="stat-label">Passed Tests</div>
        </div>
        <div class="stat-card">
            <div class="stat-number error">${stats.failed || 0}</div>
            <div class="stat-label">Failed Tests</div>
        </div>
        <div class="stat-card">
            <div class="stat-number info">${stats.total || 0}</div>
            <div class="stat-label">Total Tests</div>
        </div>
        <div class="stat-card">
            <div class="stat-number ${stats.duration < 30000 ? 'success' : 'warning'}">${Math.round((stats.duration || 0) / 1000)}s</div>
            <div class="stat-label">Duration</div>
        </div>
    </div>

    <div class="visual-coverage">
        <h3>📸 Visual Coverage</h3>
        <p>These screenshots verify that accessibility overlays appear correctly across different scenarios:</p>
        <div class="screenshot-grid">
            <div class="screenshot-item">
                <strong>Action Preview</strong><br>
                <small>Basic overlay appearance</small>
            </div>
            <div class="screenshot-item">
                <strong>With Description</strong><br>
                <small>Descriptive text display</small>
            </div>
            <div class="screenshot-item">
                <strong>Countdown Timer</strong><br>
                <small>Preview delay countdown</small>
            </div>
            <div class="screenshot-item">
                <strong>Multiple Actions</strong><br>
                <small>Concurrent action handling</small>
            </div>
            <div class="screenshot-item">
                <strong>Settings Panel</strong><br>
                <small>Configuration interface</small>
            </div>
            <div class="screenshot-item">
                <strong>Mouse Trail</strong><br>
                <small>Visual feedback trail</small>
            </div>
        </div>
    </div>

    <div class="test-results">
        <div class="test-category">
            <h3>✅ Passed Tests (${passedTests.length})</h3>
            ${passedTests.map((test: any) => `
                <div class="test-item passed">
                    <div class="test-title">${test.title}</div>
                    <div class="test-duration">${Math.round(test.duration || 0)}ms</div>
                </div>
            `).join('')}
        </div>

        ${failedTests.length > 0 ? `
        <div class="test-category">
            <h3>❌ Failed Tests (${failedTests.length})</h3>
            ${failedTests.map((test: any) => `
                <div class="test-item failed">
                    <div class="test-title">${test.title}</div>
                    <div class="test-duration">${Math.round(test.duration || 0)}ms</div>
                    ${test.error ? `<div style="color: #dc2626; font-size: 0.9em; margin-top: 5px;">${test.error}</div>` : ''}
                </div>
            `).join('')}
        </div>
        ` : ''}
    </div>

    <div class="footer">
        <p>
            <strong>Accessibility Standards:</strong> This report verifies that Luna's agent actions provide 
            appropriate visual feedback to users, meeting WCAG 2.1 guidelines for transparency and user control.
        </p>
        <p>
            <strong>Test Coverage:</strong> Visual regression tests ensure consistent overlay appearance, 
            interaction responsiveness, and accessibility feature functionality across browsers and viewports.
        </p>
    </div>
</body>
</html>
  `;
}

export default globalTeardown;