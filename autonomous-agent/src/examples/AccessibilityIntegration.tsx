/**
 * Example integration of AccessibilityOverlay with Luna Agent
 * Shows how to provide visual feedback for agent actions
 */

import React, { useState, useCallback, useEffect } from 'react';
import { AccessibilityOverlay, AccessibilitySettings } from '../components/AccessibilityOverlay';
import { AccessibilityService } from '../services/AccessibilityService';

// Default accessibility settings
const defaultSettings: AccessibilitySettings = {
  enabled: true,
  showPreview: true,
  previewDelay: 2000,
  showTrail: true,
  showDescription: true,
  opacity: 0.9,
  sound: true,
  pauseOnHover: true,
  allowUserCancel: true
};

// Initialize accessibility service
const accessibilityService = new AccessibilityService(
  {
    enabled: true,
    defaultDelay: 2000,
    minDelay: 500,
    maxDelay: 10000,
    soundEnabled: true,
    announceActions: true,
    highlightDuration: 2000
  },
  defaultSettings
);

/**
 * Main Luna Agent component with accessibility integration
 */
export const LunaAgentWithAccessibility: React.FC = () => {
  const [settings, setSettings] = useState<AccessibilitySettings>(defaultSettings);
  const [isAgentActive, setIsAgentActive] = useState(false);
  const [currentTask, setCurrentTask] = useState<string>('');

  // Handle settings changes
  const handleSettingsChange = useCallback((newSettings: AccessibilitySettings) => {
    setSettings(newSettings);
    accessibilityService.updateSettings(newSettings);
  }, []);

  // Handle action cancellation
  const handleActionCancel = useCallback((actionId: string) => {
    console.log(`User cancelled action: ${actionId}`);
    // Implement actual cancellation logic here
    accessibilityService.cancelAllActions();
  }, []);

  // Example: Simulate agent clicking a button
  const simulateAgentClick = useCallback(async () => {
    if (!isAgentActive) return;

    const elementInfo = {
      tagName: 'BUTTON',
      id: 'demo-button',
      className: 'btn btn-primary',
      text: 'Click Me',
      ariaLabel: 'Demo button for testing',
      role: 'button',
      bounds: {
        x: Math.random() * (window.innerWidth - 200) + 100,
        y: Math.random() * (window.innerHeight - 200) + 100,
        width: 120,
        height: 40
      }
    };

    const shouldProceed = await accessibilityService.previewAction(
      'click',
      elementInfo,
      'Click the demo button to test the accessibility overlay',
      { force: false }
    );

    if (shouldProceed) {
      // Simulate the actual click action
      console.log('Executing click action...');
      await new Promise(resolve => setTimeout(resolve, 100));
      accessibilityService.completeAction('current-action', true);
      console.log('Click action completed');
    } else {
      console.log('Click action was cancelled by user');
    }
  }, [isAgentActive]);

  // Example: Simulate agent typing text
  const simulateAgentType = useCallback(async () => {
    if (!isAgentActive) return;

    const elementInfo = {
      tagName: 'INPUT',
      id: 'demo-input',
      className: 'form-control',
      ariaLabel: 'Demo text input',
      role: 'textbox',
      bounds: {
        x: Math.random() * (window.innerWidth - 300) + 100,
        y: Math.random() * (window.innerHeight - 200) + 100,
        width: 250,
        height: 35
      }
    };

    const textToType = 'Hello, accessibility!';
    const shouldProceed = await accessibilityService.previewAction(
      'type',
      elementInfo,
      `Type "${textToType}" in the input field`,
      { text: textToType }
    );

    if (shouldProceed) {
      console.log('Executing type action...');
      // Simulate typing with delay
      await new Promise(resolve => setTimeout(resolve, textToType.length * 50));
      accessibilityService.completeAction('current-action', true);
      console.log('Type action completed');
    }
  }, [isAgentActive]);

  // Example: Simulate agent scrolling
  const simulateAgentScroll = useCallback(async () => {
    if (!isAgentActive) return;

    const elementInfo = {
      tagName: 'BODY',
      role: 'main',
      bounds: {
        x: window.innerWidth / 2,
        y: window.innerHeight / 2,
        width: 0,
        height: 0
      }
    };

    const shouldProceed = await accessibilityService.previewAction(
      'scroll',
      elementInfo,
      'Scroll down to view more content',
      { direction: 'down', amount: 300 }
    );

    if (shouldProceed) {
      console.log('Executing scroll action...');
      window.scrollBy({ top: 300, behavior: 'smooth' });
      await new Promise(resolve => setTimeout(resolve, 500));
      accessibilityService.completeAction('current-action', true);
      console.log('Scroll action completed');
    }
  }, [isAgentActive]);

  // Example: Automated workflow with multiple actions
  const runAutomatedWorkflow = useCallback(async () => {
    if (!isAgentActive) return;

    setCurrentTask('Running automated workflow...');

    const workflow = [
      {
        type: 'click' as const,
        description: 'Click navigation menu',
        element: {
          tagName: 'NAV',
          id: 'main-nav',
          bounds: { x: 50, y: 50, width: 100, height: 30 }
        }
      },
      {
        type: 'type' as const,
        description: 'Search for accessibility features',
        element: {
          tagName: 'INPUT',
          id: 'search-box',
          bounds: { x: 200, y: 80, width: 200, height: 35 }
        },
        data: { text: 'accessibility' }
      },
      {
        type: 'click' as const,
        description: 'Click search button',
        element: {
          tagName: 'BUTTON',
          id: 'search-btn',
          bounds: { x: 420, y: 80, width: 80, height: 35 }
        }
      }
    ];

    for (const step of workflow) {
      const elementInfo = {
        ...step.element,
        className: 'workflow-element',
        ariaLabel: step.description,
        role: step.element.tagName.toLowerCase() === 'input' ? 'textbox' : 'button'
      };

      const shouldProceed = await accessibilityService.previewAction(
        step.type,
        elementInfo,
        step.description,
        step.data
      );

      if (!shouldProceed) {
        setCurrentTask('Workflow cancelled by user');
        return;
      }

      // Simulate action execution
      await new Promise(resolve => setTimeout(resolve, 300));
      accessibilityService.completeAction('current-action', true);
    }

    setCurrentTask('Workflow completed successfully!');
    setTimeout(() => setCurrentTask(''), 3000);
  }, [isAgentActive]);

  // Load settings from localStorage on mount
  useEffect(() => {
    const savedSettings = localStorage.getItem('luna-accessibility-settings');
    if (savedSettings) {
      try {
        const parsed = JSON.parse(savedSettings);
        setSettings({ ...defaultSettings, ...parsed });
        accessibilityService.updateSettings({ ...defaultSettings, ...parsed });
      } catch (error) {
        console.warn('Failed to load accessibility settings:', error);
      }
    }
  }, []);

  return (
    <div className="luna-agent-app">
      {/* Accessibility Overlay */}
      <AccessibilityOverlay
        settings={settings}
        onSettingsChange={handleSettingsChange}
        onActionCancel={handleActionCancel}
      />

      {/* Main App UI */}
      <div style={{ padding: '20px', fontFamily: 'system-ui, sans-serif' }}>
        <h1>🤖 Luna Agent with Accessibility</h1>
        <p>This demonstrates the accessibility overlay that provides visual feedback for agent actions.</p>

        {/* Agent Controls */}
        <div style={{ marginBottom: '30px', padding: '20px', background: '#f5f5f5', borderRadius: '8px' }}>
          <h3>Agent Controls</h3>
          <div style={{ display: 'flex', gap: '10px', flexWrap: 'wrap', marginBottom: '15px' }}>
            <button
              onClick={() => setIsAgentActive(!isAgentActive)}
              style={{
                padding: '10px 20px',
                backgroundColor: isAgentActive ? '#ef4444' : '#10b981',
                color: 'white',
                border: 'none',
                borderRadius: '6px',
                cursor: 'pointer'
              }}
            >
              {isAgentActive ? '⏹️ Stop Agent' : '▶️ Start Agent'}
            </button>
            
            <button
              onClick={simulateAgentClick}
              disabled={!isAgentActive}
              style={{
                padding: '10px 20px',
                backgroundColor: isAgentActive ? '#3b82f6' : '#9ca3af',
                color: 'white',
                border: 'none',
                borderRadius: '6px',
                cursor: isAgentActive ? 'pointer' : 'not-allowed'
              }}
            >
              👆 Simulate Click
            </button>

            <button
              onClick={simulateAgentType}
              disabled={!isAgentActive}
              style={{
                padding: '10px 20px',
                backgroundColor: isAgentActive ? '#8b5cf6' : '#9ca3af',
                color: 'white',
                border: 'none',
                borderRadius: '6px',
                cursor: isAgentActive ? 'pointer' : 'not-allowed'
              }}
            >
              ⌨️ Simulate Type
            </button>

            <button
              onClick={simulateAgentScroll}
              disabled={!isAgentActive}
              style={{
                padding: '10px 20px',
                backgroundColor: isAgentActive ? '#f59e0b' : '#9ca3af',
                color: 'white',
                border: 'none',
                borderRadius: '6px',
                cursor: isAgentActive ? 'pointer' : 'not-allowed'
              }}
            >
              🔄 Simulate Scroll
            </button>

            <button
              onClick={runAutomatedWorkflow}
              disabled={!isAgentActive}
              style={{
                padding: '10px 20px',
                backgroundColor: isAgentActive ? '#06b6d4' : '#9ca3af',
                color: 'white',
                border: 'none',
                borderRadius: '6px',
                cursor: isAgentActive ? 'pointer' : 'not-allowed'
              }}
            >
              🔄 Run Workflow
            </button>
          </div>

          {currentTask && (
            <div style={{
              padding: '10px',
              backgroundColor: '#dbeafe',
              border: '1px solid #3b82f6',
              borderRadius: '4px',
              color: '#1e40af'
            }}>
              <strong>Current Task:</strong> {currentTask}
            </div>
          )}
        </div>

        {/* Accessibility Features Info */}
        <div style={{ marginBottom: '30px', padding: '20px', background: '#ecfdf5', borderRadius: '8px' }}>
          <h3>♿ Accessibility Features</h3>
          <ul style={{ lineHeight: '1.8' }}>
            <li><strong>Visual Preview:</strong> See what the agent will do before it acts (200ms+ delay)</li>
            <li><strong>Action Descriptions:</strong> Clear explanations of each planned action</li>
            <li><strong>User Control:</strong> Cancel pending actions with ESC key or click Cancel</li>
            <li><strong>Mouse Trail:</strong> Visual feedback showing agent cursor movement</li>
            <li><strong>Keyboard Shortcuts:</strong> Ctrl+Shift+A to toggle overlay</li>
            <li><strong>Sound Feedback:</strong> Audio cues for different action types (optional)</li>
            <li><strong>Customizable:</strong> Adjust timing, opacity, and behavior to your needs</li>
          </ul>
        </div>

        {/* Demo Elements */}
        <div style={{ marginBottom: '30px', padding: '20px', background: '#fef3c7', borderRadius: '8px' }}>
          <h3>Demo Elements</h3>
          <p>These elements are here for testing the accessibility overlay:</p>
          
          <div style={{ display: 'flex', gap: '15px', flexWrap: 'wrap', marginTop: '15px' }}>
            <button
              id="demo-button"
              style={{
                padding: '10px 20px',
                backgroundColor: '#7c3aed',
                color: 'white',
                border: 'none',
                borderRadius: '6px',
                cursor: 'pointer'
              }}
            >
              Demo Button
            </button>

            <input
              id="demo-input"
              type="text"
              placeholder="Demo input field"
              style={{
                padding: '10px',
                border: '1px solid #d1d5db',
                borderRadius: '6px',
                width: '200px'
              }}
            />

            <select
              style={{
                padding: '10px',
                border: '1px solid #d1d5db',
                borderRadius: '6px'
              }}
            >
              <option>Demo Select</option>
              <option>Option 1</option>
              <option>Option 2</option>
            </select>
          </div>
        </div>

        {/* Instructions */}
        <div style={{ padding: '20px', background: '#e0e7ff', borderRadius: '8px' }}>
          <h3>📋 How to Test</h3>
          <ol style={{ lineHeight: '1.8' }}>
            <li>Click "Start Agent" to enable the agent</li>
            <li>Use the simulation buttons to trigger different agent actions</li>
            <li>Watch for the accessibility overlay to appear before each action</li>
            <li>Try cancelling actions with the ESC key or Cancel button</li>
            <li>Adjust settings using the overlay settings panel (top-right corner)</li>
            <li>Test keyboard shortcuts: Ctrl+Shift+A to toggle the overlay</li>
          </ol>
          
          <p><strong>Note:</strong> In a real Luna Agent, these actions would be triggered by AI decisions, not manual buttons. The accessibility overlay ensures users always know what the agent is about to do.</p>
        </div>
      </div>
    </div>
  );
};

/**
 * Usage with React Router or other routing solutions
 */
export const AccessibilityProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [settings, setSettings] = useState<AccessibilitySettings>(defaultSettings);

  const handleSettingsChange = useCallback((newSettings: AccessibilitySettings) => {
    setSettings(newSettings);
    accessibilityService.updateSettings(newSettings);
  }, []);

  const handleActionCancel = useCallback((actionId: string) => {
    accessibilityService.cancelAllActions();
  }, []);

  return (
    <>
      {children}
      <AccessibilityOverlay
        settings={settings}
        onSettingsChange={handleSettingsChange}
        onActionCancel={handleActionCancel}
      />
    </>
  );
};

export default LunaAgentWithAccessibility;