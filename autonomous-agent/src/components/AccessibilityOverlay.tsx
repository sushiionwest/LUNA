import React, { useState, useEffect, useCallback } from 'react';
import { createPortal } from 'react-dom';

export interface OverlayAction {
  id: string;
  type: 'click' | 'type' | 'scroll' | 'hover' | 'drag' | 'screenshot';
  target: {
    x: number;
    y: number;
    width?: number;
    height?: number;
    element?: string;
    description?: string;
  };
  action: {
    description: string;
    data?: any;
    duration?: number;
    delay?: number;
  };
  timestamp: Date;
  status: 'pending' | 'active' | 'completed' | 'cancelled';
}

export interface AccessibilitySettings {
  enabled: boolean;
  showPreview: boolean;
  previewDelay: number; // ms before action executes
  showTrail: boolean;
  showDescription: boolean;
  opacity: number;
  sound: boolean;
  pauseOnHover: boolean;
  allowUserCancel: boolean;
}

interface AccessibilityOverlayProps {
  settings: AccessibilitySettings;
  onSettingsChange: (settings: AccessibilitySettings) => void;
  onActionCancel?: (actionId: string) => void;
}

/**
 * Accessibility overlay that provides visual feedback for agent actions
 * Shows users what the agent is about to do before it acts
 */
export const AccessibilityOverlay: React.FC<AccessibilityOverlayProps> = ({
  settings,
  onSettingsChange,
  onActionCancel
}) => {
  const [actions, setActions] = useState<OverlayAction[]>([]);
  const [isVisible, setIsVisible] = useState(false);
  const [mouseTrail, setMouseTrail] = useState<Array<{x: number, y: number, timestamp: number}>>([]);

  // Listen for agent actions via window events
  useEffect(() => {
    const handleAgentAction = (event: CustomEvent<OverlayAction>) => {
      if (!settings.enabled) return;
      
      const action = event.detail;
      setActions(prev => [...prev.filter(a => a.status !== 'completed'), action]);
      setIsVisible(true);

      // Auto-remove completed actions after delay
      setTimeout(() => {
        setActions(prev => prev.filter(a => a.id !== action.id));
      }, 3000);
    };

    const handleActionUpdate = (event: CustomEvent<{id: string, status: OverlayAction['status']}>) => {
      const { id, status } = event.detail;
      setActions(prev => prev.map(a => a.id === id ? { ...a, status } : a));
    };

    window.addEventListener('luna:action:preview', handleAgentAction as EventListener);
    window.addEventListener('luna:action:update', handleActionUpdate as EventListener);

    return () => {
      window.removeEventListener('luna:action:preview', handleAgentAction as EventListener);
      window.removeEventListener('luna:action:update', handleActionUpdate as EventListener);
    };
  }, [settings.enabled]);

  // Track mouse trail for visual feedback
  useEffect(() => {
    if (!settings.showTrail) return;

    const handleMouseMove = (e: MouseEvent) => {
      setMouseTrail(prev => {
        const newTrail = [...prev, { x: e.clientX, y: e.clientY, timestamp: Date.now() }];
        // Keep only last 20 points and recent ones
        return newTrail
          .filter(point => Date.now() - point.timestamp < 2000)
          .slice(-20);
      });
    };

    window.addEventListener('mousemove', handleMouseMove);
    return () => window.removeEventListener('mousemove', handleMouseMove);
  }, [settings.showTrail]);

  // Cancel action handler
  const handleCancelAction = useCallback((actionId: string) => {
    if (settings.allowUserCancel && onActionCancel) {
      onActionCancel(actionId);
      setActions(prev => prev.map(a => a.id === actionId ? { ...a, status: 'cancelled' } : a));
    }
  }, [settings.allowUserCancel, onActionCancel]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyPress = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && settings.allowUserCancel) {
        const activeActions = actions.filter(a => a.status === 'pending' || a.status === 'active');
        activeActions.forEach(action => handleCancelAction(action.id));
      }
      
      if (e.ctrlKey && e.shiftKey && e.key === 'A') {
        onSettingsChange({ ...settings, enabled: !settings.enabled });
      }
    };

    window.addEventListener('keydown', handleKeyPress);
    return () => window.removeEventListener('keydown', handleKeyPress);
  }, [actions, settings, onSettingsChange, handleCancelAction]);

  if (!settings.enabled || !isVisible) {
    return null;
  }

  return createPortal(
    <div className="luna-accessibility-overlay">
      {/* Action Previews */}
      {actions.map(action => (
        <ActionPreview
          key={action.id}
          action={action}
          settings={settings}
          onCancel={() => handleCancelAction(action.id)}
        />
      ))}

      {/* Mouse Trail */}
      {settings.showTrail && mouseTrail.map((point, index) => (
        <div
          key={`${point.x}-${point.y}-${point.timestamp}`}
          className="luna-mouse-trail-point"
          style={{
            position: 'fixed',
            left: point.x - 2,
            top: point.y - 2,
            width: 4,
            height: 4,
            backgroundColor: '#3b82f6',
            borderRadius: '50%',
            opacity: (index / mouseTrail.length) * settings.opacity,
            pointerEvents: 'none',
            zIndex: 10000 + index,
            transition: 'opacity 0.5s ease-out'
          }}
        />
      ))}

      {/* Settings Panel */}
      <SettingsPanel
        settings={settings}
        onSettingsChange={onSettingsChange}
        actionsCount={actions.length}
      />
    </div>,
    document.body
  );
};

const ActionPreview: React.FC<{
  action: OverlayAction;
  settings: AccessibilitySettings;
  onCancel: () => void;
}> = ({ action, settings, onCancel }) => {
  const [countdown, setCountdown] = useState(settings.previewDelay);
  const [isPaused, setIsPaused] = useState(false);

  useEffect(() => {
    if (action.status !== 'pending' || isPaused) return;

    const interval = setInterval(() => {
      setCountdown(prev => {
        if (prev <= 100) {
          // Action should execute now
          window.dispatchEvent(new CustomEvent('luna:action:execute', { 
            detail: { id: action.id } 
          }));
          return 0;
        }
        return prev - 100;
      });
    }, 100);

    return () => clearInterval(interval);
  }, [action.status, action.id, isPaused]);

  const getActionIcon = () => {
    switch (action.type) {
      case 'click': return '👆';
      case 'type': return '⌨️';
      case 'scroll': return '🔄';
      case 'hover': return '👋';
      case 'drag': return '👐';
      case 'screenshot': return '📸';
      default: return '🤖';
    }
  };

  const getStatusColor = () => {
    switch (action.status) {
      case 'pending': return '#f59e0b';
      case 'active': return '#3b82f6';
      case 'completed': return '#10b981';
      case 'cancelled': return '#ef4444';
      default: return '#6b7280';
    }
  };

  return (
    <>
      {/* Target Highlight */}
      <div
        className="luna-target-highlight"
        style={{
          position: 'fixed',
          left: action.target.x - (action.target.width ? action.target.width / 2 : 10),
          top: action.target.y - (action.target.height ? action.target.height / 2 : 10),
          width: action.target.width || 20,
          height: action.target.height || 20,
          border: `3px solid ${getStatusColor()}`,
          borderRadius: '8px',
          backgroundColor: `${getStatusColor()}20`,
          pointerEvents: 'none',
          zIndex: 9999,
          opacity: settings.opacity,
          animation: action.status === 'pending' ? 'luna-pulse 1s infinite' : 'none',
          transition: 'all 0.3s ease'
        }}
        onMouseEnter={() => settings.pauseOnHover && setIsPaused(true)}
        onMouseLeave={() => setIsPaused(false)}
      />

      {/* Action Description */}
      {settings.showDescription && (
        <div
          className="luna-action-description"
          style={{
            position: 'fixed',
            left: action.target.x + 20,
            top: action.target.y - 40,
            backgroundColor: '#1f2937',
            color: 'white',
            padding: '8px 12px',
            borderRadius: '6px',
            fontSize: '14px',
            fontFamily: 'system-ui, sans-serif',
            maxWidth: '300px',
            boxShadow: '0 4px 12px rgba(0, 0, 0, 0.3)',
            zIndex: 10001,
            opacity: settings.opacity,
            pointerEvents: settings.allowUserCancel ? 'auto' : 'none'
          }}
        >
          <div className="flex items-center gap-2 mb-1">
            <span className="text-lg">{getActionIcon()}</span>
            <span className="font-semibold">{action.type.toUpperCase()}</span>
            {action.status === 'pending' && countdown > 0 && (
              <span className="text-xs bg-orange-500 px-2 py-1 rounded">
                {Math.ceil(countdown / 1000)}s
              </span>
            )}
          </div>
          
          <div className="text-sm text-gray-300">
            {action.action.description}
          </div>
          
          {action.target.element && (
            <div className="text-xs text-gray-400 mt-1">
              Target: {action.target.element}
            </div>
          )}

          {settings.allowUserCancel && action.status === 'pending' && (
            <div className="mt-2 flex gap-2">
              <button
                onClick={onCancel}
                className="px-3 py-1 text-xs bg-red-600 hover:bg-red-700 rounded transition-colors"
              >
                Cancel (ESC)
              </button>
              <button
                onClick={() => setIsPaused(!isPaused)}
                className="px-3 py-1 text-xs bg-blue-600 hover:bg-blue-700 rounded transition-colors"
              >
                {isPaused ? 'Resume' : 'Pause'}
              </button>
            </div>
          )}
        </div>
      )}

      {/* Countdown Indicator */}
      {action.status === 'pending' && settings.showPreview && (
        <div
          style={{
            position: 'fixed',
            left: action.target.x - 15,
            top: action.target.y - 15,
            width: 30,
            height: 30,
            border: '3px solid transparent',
            borderTop: `3px solid ${getStatusColor()}`,
            borderRadius: '50%',
            zIndex: 10000,
            opacity: settings.opacity,
            pointerEvents: 'none',
            animation: isPaused ? 'none' : 'luna-spin 1s linear infinite'
          }}
        />
      )}
    </>
  );
};

const SettingsPanel: React.FC<{
  settings: AccessibilitySettings;
  onSettingsChange: (settings: AccessibilitySettings) => void;
  actionsCount: number;
}> = ({ settings, onSettingsChange, actionsCount }) => {
  const [isExpanded, setIsExpanded] = useState(false);

  return (
    <div
      className="luna-settings-panel"
      style={{
        position: 'fixed',
        top: 20,
        right: 20,
        backgroundColor: '#1f2937',
        color: 'white',
        borderRadius: '8px',
        boxShadow: '0 4px 12px rgba(0, 0, 0, 0.3)',
        zIndex: 10002,
        fontFamily: 'system-ui, sans-serif',
        fontSize: '14px',
        opacity: settings.opacity
      }}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between p-3 cursor-pointer"
        onClick={() => setIsExpanded(!isExpanded)}
      >
        <div className="flex items-center gap-2">
          <span className="text-lg">♿</span>
          <span className="font-semibold">Luna Assistant</span>
          {actionsCount > 0 && (
            <span className="bg-blue-600 px-2 py-1 rounded-full text-xs">
              {actionsCount}
            </span>
          )}
        </div>
        <span className="text-gray-400">
          {isExpanded ? '▼' : '▶'}
        </span>
      </div>

      {/* Settings */}
      {isExpanded && (
        <div className="border-t border-gray-600 p-3 space-y-3">
          <div className="flex items-center justify-between">
            <label className="text-sm">Show Previews</label>
            <input
              type="checkbox"
              checked={settings.showPreview}
              onChange={(e) => onSettingsChange({
                ...settings,
                showPreview: e.target.checked
              })}
              className="w-4 h-4"
            />
          </div>

          <div className="flex items-center justify-between">
            <label className="text-sm">Show Descriptions</label>
            <input
              type="checkbox"
              checked={settings.showDescription}
              onChange={(e) => onSettingsChange({
                ...settings,
                showDescription: e.target.checked
              })}
              className="w-4 h-4"
            />
          </div>

          <div className="flex items-center justify-between">
            <label className="text-sm">Mouse Trail</label>
            <input
              type="checkbox"
              checked={settings.showTrail}
              onChange={(e) => onSettingsChange({
                ...settings,
                showTrail: e.target.checked
              })}
              className="w-4 h-4"
            />
          </div>

          <div className="flex items-center justify-between">
            <label className="text-sm">Allow Cancel</label>
            <input
              type="checkbox"
              checked={settings.allowUserCancel}
              onChange={(e) => onSettingsChange({
                ...settings,
                allowUserCancel: e.target.checked
              })}
              className="w-4 h-4"
            />
          </div>

          <div>
            <label className="text-sm block mb-1">Preview Delay: {settings.previewDelay}ms</label>
            <input
              type="range"
              min="0"
              max="5000"
              step="200"
              value={settings.previewDelay}
              onChange={(e) => onSettingsChange({
                ...settings,
                previewDelay: Number(e.target.value)
              })}
              className="w-full"
            />
          </div>

          <div>
            <label className="text-sm block mb-1">Opacity: {Math.round(settings.opacity * 100)}%</label>
            <input
              type="range"
              min="0.1"
              max="1"
              step="0.1"
              value={settings.opacity}
              onChange={(e) => onSettingsChange({
                ...settings,
                opacity: Number(e.target.value)
              })}
              className="w-full"
            />
          </div>

          <div className="pt-2 border-t border-gray-600 text-xs text-gray-400">
            <div>Ctrl+Shift+A: Toggle overlay</div>
            <div>ESC: Cancel pending actions</div>
          </div>
        </div>
      )}
    </div>
  );
};

// CSS-in-JS styles for animations
const style = document.createElement('style');
style.textContent = `
  @keyframes luna-pulse {
    0%, 100% { transform: scale(1); opacity: 1; }
    50% { transform: scale(1.1); opacity: 0.7; }
  }
  
  @keyframes luna-spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .luna-accessibility-overlay * {
    box-sizing: border-box;
  }
`;
document.head.appendChild(style);

export default AccessibilityOverlay;