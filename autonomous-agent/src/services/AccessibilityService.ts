import { OverlayAction, AccessibilitySettings } from '../components/AccessibilityOverlay';

export interface AccessibilityConfig {
  enabled: boolean;
  defaultDelay: number;
  minDelay: number;
  maxDelay: number;
  soundEnabled: boolean;
  announceActions: boolean;
  highlightDuration: number;
}

export interface ElementInfo {
  tagName: string;
  id?: string;
  className?: string;
  text?: string;
  ariaLabel?: string;
  role?: string;
  bounds: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
}

/**
 * Accessibility service that coordinates with the overlay to provide
 * visual feedback for agent actions
 */
export class AccessibilityService {
  private config: AccessibilityConfig;
  private settings: AccessibilitySettings;
  private pendingActions = new Map<string, OverlayAction>();
  private actionHistory: OverlayAction[] = [];
  private isEnabled = true;
  private soundContext?: AudioContext;

  constructor(config: AccessibilityConfig, settings: AccessibilitySettings) {
    this.config = config;
    this.settings = settings;
    
    this.initializeAudio();
    this.setupEventListeners();
  }

  /**
   * Preview an action before executing it
   */
  async previewAction(
    type: OverlayAction['type'],
    element: ElementInfo,
    description: string,
    actionData?: any,
    customDelay?: number
  ): Promise<boolean> {
    if (!this.isEnabled || !this.settings.enabled) {
      return true; // Skip preview and execute immediately
    }

    const actionId = this.generateActionId();
    const delay = customDelay ?? this.settings.previewDelay;

    const action: OverlayAction = {
      id: actionId,
      type,
      target: {
        x: element.bounds.x + element.bounds.width / 2,
        y: element.bounds.y + element.bounds.height / 2,
        width: element.bounds.width,
        height: element.bounds.height,
        element: this.getElementDescription(element),
        description: element.ariaLabel || element.text
      },
      action: {
        description,
        data: actionData,
        duration: this.estimateActionDuration(type, actionData),
        delay
      },
      timestamp: new Date(),
      status: 'pending'
    };

    // Store pending action
    this.pendingActions.set(actionId, action);

    // Announce action if enabled
    if (this.config.announceActions) {
      this.announceAction(action);
    }

    // Play preview sound
    if (this.config.soundEnabled && this.settings.sound) {
      this.playSound('preview');
    }

    // Dispatch preview event to overlay
    window.dispatchEvent(new CustomEvent('luna:action:preview', {
      detail: action
    }));

    // Wait for user confirmation or timeout
    return new Promise((resolve) => {
      let timeoutId: NodeJS.Timeout;
      let cancelled = false;

      // Handle cancellation
      const handleCancel = (event: CustomEvent) => {
        if (event.detail.id === actionId) {
          cancelled = true;
          clearTimeout(timeoutId);
          this.updateActionStatus(actionId, 'cancelled');
          resolve(false);
        }
      };

      // Handle execution
      const handleExecute = (event: CustomEvent) => {
        if (event.detail.id === actionId && !cancelled) {
          clearTimeout(timeoutId);
          this.updateActionStatus(actionId, 'active');
          resolve(true);
        }
      };

      // Set up event listeners
      window.addEventListener('luna:action:cancel', handleCancel as EventListener);
      window.addEventListener('luna:action:execute', handleExecute as EventListener);

      // Auto-execute after delay
      timeoutId = setTimeout(() => {
        if (!cancelled) {
          this.updateActionStatus(actionId, 'active');
          window.removeEventListener('luna:action:cancel', handleCancel as EventListener);
          window.removeEventListener('luna:action:execute', handleExecute as EventListener);
          resolve(true);
        }
      }, delay);

      // Clean up listeners after timeout
      setTimeout(() => {
        window.removeEventListener('luna:action:cancel', handleCancel as EventListener);
        window.removeEventListener('luna:action:execute', handleExecute as EventListener);
      }, delay + 1000);
    });
  }

  /**
   * Mark an action as completed
   */
  completeAction(actionId: string, success = true): void {
    const action = this.pendingActions.get(actionId);
    if (action) {
      this.updateActionStatus(actionId, 'completed');
      this.actionHistory.push({ ...action, status: 'completed' });
      this.pendingActions.delete(actionId);

      // Play completion sound
      if (this.config.soundEnabled && this.settings.sound) {
        this.playSound(success ? 'success' : 'error');
      }

      // Keep history limited
      if (this.actionHistory.length > 100) {
        this.actionHistory = this.actionHistory.slice(-100);
      }
    }
  }

  /**
   * Cancel all pending actions
   */
  cancelAllActions(): void {
    Array.from(this.pendingActions.keys()).forEach(actionId => {
      this.updateActionStatus(actionId, 'cancelled');
      window.dispatchEvent(new CustomEvent('luna:action:cancel', {
        detail: { id: actionId }
      }));
    });
    this.pendingActions.clear();
  }

  /**
   * Get action history for analysis
   */
  getActionHistory(count = 50): OverlayAction[] {
    return this.actionHistory.slice(-count);
  }

  /**
   * Update accessibility settings
   */
  updateSettings(newSettings: Partial<AccessibilitySettings>): void {
    this.settings = { ...this.settings, ...newSettings };
    
    // Save to localStorage
    localStorage.setItem('luna-accessibility-settings', JSON.stringify(this.settings));
  }

  /**
   * Enable or disable the accessibility service
   */
  setEnabled(enabled: boolean): void {
    this.isEnabled = enabled;
    
    if (!enabled) {
      this.cancelAllActions();
    }
  }

  /**
   * Get element information for accessibility
   */
  async getElementInfo(selector: string): Promise<ElementInfo | null> {
    try {
      // This would typically interact with a browser automation tool
      // For now, we'll return mock data
      return {
        tagName: 'BUTTON',
        id: 'submit-btn',
        className: 'btn btn-primary',
        text: 'Submit',
        ariaLabel: 'Submit form',
        role: 'button',
        bounds: {
          x: 100,
          y: 200,
          width: 80,
          height: 40
        }
      };
    } catch (error) {
      console.error('Failed to get element info:', error);
      return null;
    }
  }

  /**
   * Highlight element without action preview
   */
  highlightElement(element: ElementInfo, duration = 2000): void {
    const highlightId = this.generateActionId();
    
    const highlightAction: OverlayAction = {
      id: highlightId,
      type: 'hover',
      target: {
        x: element.bounds.x + element.bounds.width / 2,
        y: element.bounds.y + element.bounds.height / 2,
        width: element.bounds.width,
        height: element.bounds.height,
        element: this.getElementDescription(element),
        description: 'Highlighting element'
      },
      action: {
        description: 'Element highlighted for inspection',
        duration
      },
      timestamp: new Date(),
      status: 'active'
    };

    window.dispatchEvent(new CustomEvent('luna:action:preview', {
      detail: highlightAction
    }));

    setTimeout(() => {
      this.updateActionStatus(highlightId, 'completed');
    }, duration);
  }

  /**
   * Check if action should be previewed based on type and settings
   */
  shouldPreviewAction(type: OverlayAction['type']): boolean {
    if (!this.isEnabled || !this.settings.enabled) {
      return false;
    }

    // Always preview potentially destructive actions
    const alwaysPreview = ['click', 'type', 'drag'];
    if (alwaysPreview.includes(type)) {
      return true;
    }

    // Optional preview for other actions
    return this.settings.showPreview;
  }

  /**
   * Generate accessibility report
   */
  generateAccessibilityReport(): {
    totalActions: number;
    completedActions: number;
    cancelledActions: number;
    averagePreviewTime: number;
    mostCommonActions: Array<{ type: string; count: number }>;
    settings: AccessibilitySettings;
  } {
    const completed = this.actionHistory.filter(a => a.status === 'completed').length;
    const cancelled = this.actionHistory.filter(a => a.status === 'cancelled').length;
    
    const actionTypes = this.actionHistory.reduce((acc, action) => {
      acc[action.type] = (acc[action.type] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);

    const mostCommon = Object.entries(actionTypes)
      .map(([type, count]) => ({ type, count }))
      .sort((a, b) => b.count - a.count);

    const previewTimes = this.actionHistory
      .filter(a => a.action.delay)
      .map(a => a.action.delay!);
    
    const averagePreviewTime = previewTimes.length > 0 
      ? previewTimes.reduce((sum, time) => sum + time, 0) / previewTimes.length 
      : 0;

    return {
      totalActions: this.actionHistory.length,
      completedActions: completed,
      cancelledActions: cancelled,
      averagePreviewTime,
      mostCommonActions: mostCommon,
      settings: this.settings
    };
  }

  /**
   * Private helper methods
   */
  private generateActionId(): string {
    return `action-${Date.now()}-${Math.random().toString(36).substr(2, 8)}`;
  }

  private getElementDescription(element: ElementInfo): string {
    const parts = [element.tagName.toLowerCase()];
    
    if (element.id) parts.push(`#${element.id}`);
    if (element.className) parts.push(`.${element.className.split(' ')[0]}`);
    if (element.text) parts.push(`"${element.text.slice(0, 20)}..."`);
    
    return parts.join('');
  }

  private estimateActionDuration(type: OverlayAction['type'], data?: any): number {
    switch (type) {
      case 'click': return 100;
      case 'type': return data?.text ? data.text.length * 50 : 500;
      case 'scroll': return 300;
      case 'hover': return 200;
      case 'drag': return 800;
      case 'screenshot': return 1000;
      default: return 300;
    }
  }

  private updateActionStatus(actionId: string, status: OverlayAction['status']): void {
    const action = this.pendingActions.get(actionId);
    if (action) {
      action.status = status;
      window.dispatchEvent(new CustomEvent('luna:action:update', {
        detail: { id: actionId, status }
      }));
    }
  }

  private initializeAudio(): void {
    if (typeof window !== 'undefined' && window.AudioContext) {
      this.soundContext = new AudioContext();
    }
  }

  private playSound(type: 'preview' | 'success' | 'error' | 'cancel'): void {
    if (!this.soundContext) return;

    const frequencies = {
      preview: [800, 1000],
      success: [600, 800, 1000],
      error: [400, 300],
      cancel: [300, 200]
    };

    const freq = frequencies[type];
    const oscillator = this.soundContext.createOscillator();
    const gainNode = this.soundContext.createGain();

    oscillator.connect(gainNode);
    gainNode.connect(this.soundContext.destination);

    oscillator.frequency.setValueAtTime(freq[0], this.soundContext.currentTime);
    if (freq[1]) {
      oscillator.frequency.exponentialRampToValueAtTime(freq[1], this.soundContext.currentTime + 0.1);
    }

    gainNode.gain.setValueAtTime(0.1, this.soundContext.currentTime);
    gainNode.gain.exponentialRampToValueAtTime(0.01, this.soundContext.currentTime + 0.2);

    oscillator.start();
    oscillator.stop(this.soundContext.currentTime + 0.2);
  }

  private announceAction(action: OverlayAction): void {
    if ('speechSynthesis' in window) {
      const utterance = new SpeechSynthesisUtterance(
        `Luna will ${action.type} ${action.target.description || 'element'} in ${Math.ceil(action.action.delay! / 1000)} seconds`
      );
      utterance.volume = 0.3;
      utterance.rate = 1.2;
      speechSynthesis.speak(utterance);
    }
  }

  private setupEventListeners(): void {
    // Listen for user cancellation
    window.addEventListener('luna:action:cancel', (event: CustomEvent) => {
      const { id } = event.detail;
      this.pendingActions.delete(id);
      
      if (this.config.soundEnabled && this.settings.sound) {
        this.playSound('cancel');
      }
    });

    // Load saved settings
    const savedSettings = localStorage.getItem('luna-accessibility-settings');
    if (savedSettings) {
      try {
        this.settings = { ...this.settings, ...JSON.parse(savedSettings) };
      } catch (error) {
        console.warn('Failed to load accessibility settings:', error);
      }
    }
  }
}