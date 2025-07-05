import { test, expect, Page } from '@playwright/test';

/**
 * Accessibility Overlay Visual Regression Tests
 * Tests that the accessibility overlay appears correctly and provides proper visual feedback
 */

test.describe('Accessibility Overlay', () => {
  let page: Page;

  test.beforeEach(async ({ page: testPage }) => {
    page = testPage;
    await page.goto('/');
    
    // Enable accessibility overlay
    await page.evaluate(() => {
      window.localStorage.setItem('luna-accessibility-settings', JSON.stringify({
        enabled: true,
        showPreview: true,
        previewDelay: 2000,
        showTrail: true,
        showDescription: true,
        opacity: 0.9,
        sound: false,
        pauseOnHover: true,
        allowUserCancel: true
      }));
    });
    
    await page.reload();
  });

  test('should display overlay when action is previewed', async () => {
    // Trigger a mock action preview
    await page.evaluate(() => {
      const action = {
        id: 'test-action-1',
        type: 'click',
        target: {
          x: 200,
          y: 300,
          width: 100,
          height: 40,
          element: 'button#submit',
          description: 'Submit button'
        },
        action: {
          description: 'Click the submit button',
          duration: 100,
          delay: 2000
        },
        timestamp: new Date(),
        status: 'pending'
      };

      window.dispatchEvent(new CustomEvent('luna:action:preview', { detail: action }));
    });

    // Wait for overlay to appear
    await page.waitForSelector('.luna-target-highlight', { timeout: 5000 });

    // Take screenshot for visual regression
    await expect(page).toHaveScreenshot('overlay-action-preview.png');

    // Verify highlight is positioned correctly
    const highlight = page.locator('.luna-target-highlight');
    await expect(highlight).toBeVisible();
    
    const boundingBox = await highlight.boundingBox();
    expect(boundingBox).toBeTruthy();
    expect(boundingBox!.x).toBeCloseTo(150, 10); // 200 - 100/2 + some tolerance
    expect(boundingBox!.y).toBeCloseTo(280, 10); // 300 - 40/2 + some tolerance
  });

  test('should show action description when enabled', async () => {
    // Trigger action with description
    await page.evaluate(() => {
      const action = {
        id: 'test-action-2',
        type: 'type',
        target: {
          x: 150,
          y: 250,
          width: 200,
          height: 30,
          element: 'input#username',
          description: 'Username field'
        },
        action: {
          description: 'Type "testuser" in username field',
          data: { text: 'testuser' },
          duration: 400,
          delay: 2000
        },
        timestamp: new Date(),
        status: 'pending'
      };

      window.dispatchEvent(new CustomEvent('luna:action:preview', { detail: action }));
    });

    // Wait for description to appear
    await page.waitForSelector('.luna-action-description', { timeout: 5000 });

    // Verify description content
    const description = page.locator('.luna-action-description');
    await expect(description).toBeVisible();
    await expect(description).toContainText('TYPE');
    await expect(description).toContainText('Type "testuser" in username field');
    await expect(description).toContainText('Target: input#username');

    // Take screenshot
    await expect(page).toHaveScreenshot('overlay-with-description.png');
  });

  test('should display countdown timer for pending actions', async () => {
    // Trigger action with countdown
    await page.evaluate(() => {
      const action = {
        id: 'test-action-3',
        type: 'click',
        target: {
          x: 300,
          y: 400,
          width: 80,
          height: 35,
          element: 'button.primary',
          description: 'Primary button'
        },
        action: {
          description: 'Click primary action button',
          duration: 100,
          delay: 3000
        },
        timestamp: new Date(),
        status: 'pending'
      };

      window.dispatchEvent(new CustomEvent('luna:action:preview', { detail: action }));
    });

    // Wait for countdown to appear
    await page.waitForSelector('.luna-action-description', { timeout: 5000 });

    // Verify countdown is visible
    const countdownElement = page.locator('.luna-action-description').locator('text=3s, text=2s');
    await expect(countdownElement.first()).toBeVisible();

    // Take screenshot with countdown
    await expect(page).toHaveScreenshot('overlay-with-countdown.png');

    // Wait for countdown to decrease
    await page.waitForTimeout(1500);
    await expect(page).toHaveScreenshot('overlay-countdown-decreased.png');
  });

  test('should show cancel and pause buttons when cancellation is enabled', async () => {
    // Trigger action
    await page.evaluate(() => {
      const action = {
        id: 'test-action-4',
        type: 'scroll',
        target: {
          x: 500,
          y: 600,
          width: 0,
          height: 0,
          element: 'window',
          description: 'Page scroll'
        },
        action: {
          description: 'Scroll down to next section',
          data: { direction: 'down', amount: 300 },
          duration: 300,
          delay: 2000
        },
        timestamp: new Date(),
        status: 'pending'
      };

      window.dispatchEvent(new CustomEvent('luna:action:preview', { detail: action }));
    });

    // Wait for description with buttons
    await page.waitForSelector('.luna-action-description', { timeout: 5000 });

    // Verify cancel and pause buttons
    const cancelButton = page.locator('button:has-text("Cancel")');
    const pauseButton = page.locator('button:has-text("Pause")');
    
    await expect(cancelButton).toBeVisible();
    await expect(pauseButton).toBeVisible();

    // Take screenshot
    await expect(page).toHaveScreenshot('overlay-with-controls.png');
  });

  test('should handle multiple simultaneous actions', async () => {
    // Trigger multiple actions
    await page.evaluate(() => {
      const actions = [
        {
          id: 'multi-action-1',
          type: 'click',
          target: { x: 100, y: 100, width: 50, height: 30, element: 'button1' },
          action: { description: 'Click button 1', delay: 2000 },
          timestamp: new Date(),
          status: 'pending'
        },
        {
          id: 'multi-action-2',
          type: 'type',
          target: { x: 300, y: 200, width: 150, height: 25, element: 'input1' },
          action: { description: 'Type in input field', delay: 1500 },
          timestamp: new Date(),
          status: 'pending'
        },
        {
          id: 'multi-action-3',
          type: 'hover',
          target: { x: 500, y: 300, width: 80, height: 40, element: 'link1' },
          action: { description: 'Hover over link', delay: 1000 },
          timestamp: new Date(),
          status: 'active'
        }
      ];

      actions.forEach(action => {
        window.dispatchEvent(new CustomEvent('luna:action:preview', { detail: action }));
      });
    });

    // Wait for all overlays to appear
    await page.waitForSelector('.luna-target-highlight', { timeout: 5000 });
    
    // Verify multiple highlights are visible
    const highlights = page.locator('.luna-target-highlight');
    await expect(highlights).toHaveCount(3);

    // Take screenshot of multiple actions
    await expect(page).toHaveScreenshot('overlay-multiple-actions.png');
  });

  test('should display settings panel', async () => {
    // Open settings panel
    await page.click('.luna-settings-panel');

    // Wait for expanded panel
    await page.waitForSelector('.luna-settings-panel input[type="checkbox"]', { timeout: 5000 });

    // Verify settings controls are visible
    await expect(page.locator('text=Show Previews')).toBeVisible();
    await expect(page.locator('text=Show Descriptions')).toBeVisible();
    await expect(page.locator('text=Mouse Trail')).toBeVisible();
    await expect(page.locator('text=Allow Cancel')).toBeVisible();
    await expect(page.locator('text=Preview Delay')).toBeVisible();
    await expect(page.locator('text=Opacity')).toBeVisible();

    // Take screenshot
    await expect(page).toHaveScreenshot('overlay-settings-panel.png');
  });

  test('should show mouse trail when enabled', async () => {
    // Move mouse to generate trail
    await page.mouse.move(100, 100);
    await page.waitForTimeout(100);
    await page.mouse.move(200, 150);
    await page.waitForTimeout(100);
    await page.mouse.move(300, 200);
    await page.waitForTimeout(100);
    await page.mouse.move(400, 250);

    // Wait for trail points to appear
    await page.waitForSelector('.luna-mouse-trail-point', { timeout: 5000 });

    // Verify trail points are visible
    const trailPoints = page.locator('.luna-mouse-trail-point');
    await expect(trailPoints.first()).toBeVisible();

    // Take screenshot
    await expect(page).toHaveScreenshot('overlay-mouse-trail.png');
  });

  test('should handle action status changes', async () => {
    // Trigger action
    await page.evaluate(() => {
      const action = {
        id: 'status-test-action',
        type: 'click',
        target: {
          x: 250,
          y: 350,
          width: 100,
          height: 40,
          element: 'button.test',
        },
        action: {
          description: 'Test status changes',
          delay: 1000
        },
        timestamp: new Date(),
        status: 'pending'
      };

      window.dispatchEvent(new CustomEvent('luna:action:preview', { detail: action }));
    });

    // Take screenshot in pending state
    await page.waitForSelector('.luna-target-highlight', { timeout: 5000 });
    await expect(page).toHaveScreenshot('overlay-status-pending.png');

    // Update to active status
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('luna:action:update', {
        detail: { id: 'status-test-action', status: 'active' }
      }));
    });

    await page.waitForTimeout(500);
    await expect(page).toHaveScreenshot('overlay-status-active.png');

    // Update to completed status
    await page.evaluate(() => {
      window.dispatchEvent(new CustomEvent('luna:action:update', {
        detail: { id: 'status-test-action', status: 'completed' }
      }));
    });

    await page.waitForTimeout(500);
    await expect(page).toHaveScreenshot('overlay-status-completed.png');
  });

  test('should handle keyboard shortcuts', async () => {
    // Trigger action first
    await page.evaluate(() => {
      const action = {
        id: 'keyboard-test-action',
        type: 'click',
        target: { x: 200, y: 200, width: 50, height: 30 },
        action: { description: 'Test keyboard shortcuts', delay: 5000 },
        timestamp: new Date(),
        status: 'pending'
      };

      window.dispatchEvent(new CustomEvent('luna:action:preview', { detail: action }));
    });

    await page.waitForSelector('.luna-target-highlight', { timeout: 5000 });

    // Test ESC key cancellation
    await page.keyboard.press('Escape');
    await page.waitForTimeout(500);

    // Verify action was cancelled (highlight should be gone or changed)
    const highlights = page.locator('.luna-target-highlight');
    const count = await highlights.count();
    expect(count).toBe(0); // Should be cancelled

    // Test Ctrl+Shift+A to toggle overlay
    await page.keyboard.press('Control+Shift+A');
    await page.waitForTimeout(500);
    
    // Settings panel should still be there but overlay functionality disabled
    await expect(page).toHaveScreenshot('overlay-toggled-off.png');
  });

  test('should be responsive on different screen sizes', async () => {
    // Test mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    await page.evaluate(() => {
      const action = {
        id: 'mobile-test-action',
        type: 'click',
        target: {
          x: 187, // Center of mobile screen
          y: 300,
          width: 150,
          height: 44,
          element: 'button.mobile',
        },
        action: {
          description: 'Mobile button click',
          delay: 2000
        },
        timestamp: new Date(),
        status: 'pending'
      };

      window.dispatchEvent(new CustomEvent('luna:action:preview', { detail: action }));
    });

    await page.waitForSelector('.luna-target-highlight', { timeout: 5000 });
    await expect(page).toHaveScreenshot('overlay-mobile-viewport.png');

    // Test tablet viewport
    await page.setViewportSize({ width: 768, height: 1024 });
    await page.waitForTimeout(500);
    await expect(page).toHaveScreenshot('overlay-tablet-viewport.png');
  });

  test('should handle edge cases gracefully', async () => {
    // Test action at screen edge
    await page.evaluate(() => {
      const action = {
        id: 'edge-case-action',
        type: 'click',
        target: {
          x: 10, // Very close to edge
          y: 10,
          width: 20,
          height: 20,
          element: 'button.edge',
        },
        action: {
          description: 'Click button near screen edge',
          delay: 1000
        },
        timestamp: new Date(),
        status: 'pending'
      };

      window.dispatchEvent(new CustomEvent('luna:action:preview', { detail: action }));
    });

    await page.waitForSelector('.luna-target-highlight', { timeout: 5000 });
    
    // Description should be repositioned to stay visible
    const description = page.locator('.luna-action-description');
    if (await description.isVisible()) {
      const box = await description.boundingBox();
      expect(box!.x).toBeGreaterThan(0); // Should not be cut off
    }

    await expect(page).toHaveScreenshot('overlay-edge-case.png');
  });
});