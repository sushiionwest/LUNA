const { ipcRenderer } = require('electron');

// Application state
let currentStep = 'step-welcome';
let systemInfo = null;
let installationPath = null;

// Platform detection
const isWindows = process.platform === 'win32';
const isMac = process.platform === 'darwin';
const isLinux = process.platform === 'linux';

// Initialize platform-specific styling
function initializePlatform() {
    const platformInfo = document.getElementById('platformInfo');
    
    if (isWindows) {
        document.body.className = 'windows';
        platformInfo.textContent = 'Windows';
    } else if (isMac) {
        document.body.className = 'macos';
        platformInfo.textContent = 'macOS';
    } else {
        document.body.className = 'linux';
        platformInfo.textContent = 'Linux';
    }
}

// UI Helper Functions
function showStep(stepId) {
    // Hide all steps with fade effect
    document.querySelectorAll('.installation-step').forEach(step => {
        step.classList.remove('active');
    });
    
    // Show target step with delay for smooth transition
    setTimeout(() => {
        const targetStep = document.getElementById(stepId);
        if (targetStep) {
            targetStep.classList.add('active');
            currentStep = stepId;
            
            // Update window title based on current step
            updateWindowTitle();
        }
    }, 150);
}

function updateWindowTitle() {
    const titles = {
        'step-welcome': 'Luna Agent Installer',
        'step-requirements': 'System Check - Luna Agent',
        'step-path': 'Installation Path - Luna Agent',
        'step-installing': 'Installing - Luna Agent',
        'step-success': 'Installation Complete - Luna Agent',
        'step-error': 'Installation Error - Luna Agent'
    };
    
    document.title = titles[currentStep] || 'Luna Agent Installer';
}

function goToStep(stepId) {
    showStep(stepId);
}

function setButtonLoading(buttonElement, loading = true) {
    if (!buttonElement) return;
    
    const spinner = buttonElement.querySelector('.loading-spinner');
    
    if (loading) {
        buttonElement.disabled = true;
        if (spinner) spinner.style.display = 'inline-block';
        buttonElement.style.opacity = '0.7';
        buttonElement.style.cursor = 'not-allowed';
    } else {
        buttonElement.disabled = false;
        if (spinner) spinner.style.display = 'none';
        buttonElement.style.opacity = '1';
        buttonElement.style.cursor = 'pointer';
    }
}

function showNotification(message, type = 'info', duration = 4000) {
    // Create notification element
    const notification = document.createElement('div');
    notification.className = 'notification';
    
    // Set icon based on type
    const icons = {
        'info': 'ℹ️',
        'success': '✅',
        'warning': '⚠️',
        'error': '❌'
    };
    
    notification.innerHTML = `
        <div style="display: flex; align-items: center;">
            <span style="margin-right: 10px; font-size: 1.2rem;">${icons[type] || icons.info}</span>
            <span>${message}</span>
        </div>
    `;
    
    // Apply type-specific styling
    if (type === 'error') {
        notification.style.background = 'rgba(255, 107, 107, 0.2)';
        notification.style.borderColor = 'rgba(255, 107, 107, 0.4)';
    } else if (type === 'success') {
        notification.style.background = 'rgba(76, 175, 80, 0.2)';
        notification.style.borderColor = 'rgba(76, 175, 80, 0.4)';
    } else if (type === 'warning') {
        notification.style.background = 'rgba(255, 152, 0, 0.2)';
        notification.style.borderColor = 'rgba(255, 152, 0, 0.4)';
    }
    
    document.body.appendChild(notification);
    
    // Auto-remove after specified duration
    setTimeout(() => {
        if (notification.parentElement) {
            notification.style.animation = 'slideOutRight 0.3s ease';
            setTimeout(() => notification.remove(), 300);
        }
    }, duration);
}

// Enhanced platform-specific error handling
function handlePlatformSpecificError(error) {
    if (isWindows) {
        if (error.includes('VirtualBox')) {
            return 'VirtualBox installation failed. Please ensure Hyper-V is disabled and hardware virtualization is enabled in your BIOS settings.';
        } else if (error.includes('permission')) {
            return 'Permission denied. Please run the installer as Administrator by right-clicking and selecting "Run as administrator".';
        } else if (error.includes('disk space')) {
            return 'Insufficient disk space. Please free up at least 10GB of space on your system drive.';
        }
    }
    return error;
}

// Step 1: Welcome Screen
async function checkRequirements() {
    const welcomeBtn = document.querySelector('#step-welcome .btn');
    const spinner = document.getElementById('welcome-spinner');
    
    setButtonLoading(welcomeBtn, true);
    showNotification('Starting system compatibility check...', 'info');
    
    showStep('step-requirements');
    
    try {
        const result = await ipcRenderer.invoke('check-system-requirements');
        
        if (result.success) {
            systemInfo = result;
            displaySystemRequirements(result);
            showNotification('System check completed', 'success');
        } else {
            showError('Failed to check system requirements: ' + result.error);
        }
    } catch (error) {
        console.error('Requirements check error:', error);
        showError('Error checking system requirements: ' + error.message);
    } finally {
        setButtonLoading(welcomeBtn, false);
    }
}

// Step 2: System Requirements Display
function displaySystemRequirements(systemData) {
    const container = document.getElementById('requirements-container');
    const continueBtn = document.getElementById('requirements-continue-btn');
    
    if (!systemData.requirements || systemData.requirements.length === 0) {
        container.innerHTML = `
            <div class="requirement-item">
                <span class="requirement-status">❌</span>
                <div class="requirement-content">
                    <div class="requirement-name">Unable to check requirements</div>
                    <div class="requirement-details">Please try again or contact support</div>
                </div>
            </div>
        `;
        continueBtn.disabled = true;
        return;
    }

    let html = '';
    let criticalIssues = 0;
    let warnings = 0;
    let passed = 0;

    systemData.requirements.forEach(req => {
        let statusIcon, statusColor;
        
        if (req.met) {
            statusIcon = '✅';
            statusColor = '#4caf50';
            passed++;
        } else if (req.warning) {
            statusIcon = '⚠️';
            statusColor = '#ff9800';
            warnings++;
        } else {
            statusIcon = '❌';
            statusColor = '#f44336';
            if (req.critical) criticalIssues++;
        }
        
        html += `
            <div class="requirement-item" style="border-left: 4px solid ${statusColor};">
                <span class="requirement-status">${statusIcon}</span>
                <div class="requirement-content">
                    <div class="requirement-name">${req.name}</div>
                    <div class="requirement-details">${req.details}</div>
                    ${req.description && !req.met ? `<div style="font-size: 0.85rem; opacity: 0.7; margin-top: 6px; font-style: italic;">${req.description}</div>` : ''}
                </div>
            </div>
        `;
    });

    // Add detailed system summary
    if (systemData.system) {
        const memoryGB = Math.round(systemData.system.memory.total / 1024 / 1024 / 1024);
        const freeMemoryGB = Math.round(systemData.system.memory.free / 1024 / 1024 / 1024);
        const platformName = getPlatformDisplayName(systemData.system.platform);
        
        html += `
            <div style="margin-top: 28px; padding: 22px; background: rgba(255, 255, 255, 0.05); border-radius: 14px;">
                <h4 style="margin-bottom: 14px; opacity: 0.9; font-size: 1.15rem; text-align: center;">🖥️ System Information</h4>
                <div style="font-size: 0.95rem; opacity: 0.8; line-height: 1.6;">
                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px; text-align: left;">
                        <div><strong>Operating System:</strong><br>${platformName} ${systemData.system.osVersion}</div>
                        <div><strong>Architecture:</strong><br>${systemData.system.arch}</div>
                        <div><strong>Memory:</strong><br>${memoryGB}GB total (${freeMemoryGB}GB free)</div>
                        <div><strong>Processor:</strong><br>${systemData.system.cpu.count} cores</div>
                    </div>
                    <div style="margin-top: 12px; text-align: center;">
                        <strong>Computer:</strong> ${systemData.system.hostname} | <strong>User:</strong> ${systemData.system.userInfo?.username || 'Unknown'}
                    </div>
                </div>
            </div>
        `;
    }

    // Add compatibility summary
    html += `
        <div style="margin-top: 20px; padding: 18px; background: rgba(255, 255, 255, 0.08); border-radius: 12px; text-align: center;">
            <strong>Compatibility Summary:</strong> 
            <span style="color: #4caf50;">✅ ${passed} Passed</span> | 
            <span style="color: #ff9800;">⚠️ ${warnings} Warnings</span> | 
            <span style="color: #f44336;">❌ ${criticalIssues} Issues</span>
        </div>
    `;

    container.innerHTML = html;

    // Update continue button based on requirements
    if (criticalIssues === 0) {
        continueBtn.disabled = false;
        
        if (warnings > 0) {
            continueBtn.textContent = `Continue (${warnings} warning${warnings > 1 ? 's' : ''})`;
            continueBtn.className = 'btn btn-warning';
            showNotification(`System check passed with ${warnings} warning${warnings > 1 ? 's' : ''}`, 'warning');
        } else {
            continueBtn.textContent = '✅ Continue Installation';
            continueBtn.className = 'btn btn-success';
            showNotification('All system requirements met!', 'success');
        }
    } else {
        continueBtn.disabled = true;
        continueBtn.textContent = `❌ ${criticalIssues} Critical Issue${criticalIssues > 1 ? 's' : ''}`;
        continueBtn.className = 'btn';
        continueBtn.style.background = 'rgba(244, 67, 54, 0.6)';
        showNotification(`${criticalIssues} critical issue${criticalIssues > 1 ? 's' : ''} must be resolved`, 'error');
    }
}

function getPlatformDisplayName(platform) {
    switch (platform) {
        case 'win32': return 'Windows';
        case 'darwin': return 'macOS';
        case 'linux': return 'Linux';
        default: return platform;
    }
}

// Step 3: Installation Path Selection
function proceedToInstall() {
    showStep('step-path');
    
    // Set default installation path based on OS
    const pathInput = document.getElementById('installPath');
    const startBtn = document.getElementById('start-install-btn');
    
    if (systemInfo && systemInfo.system) {
        const platform = systemInfo.system.platform;
        let defaultPath;
        
        if (platform === 'win32') {
            defaultPath = 'C:\\Program Files\\Luna Agent';
        } else if (platform === 'darwin') {
            defaultPath = '/Applications/Luna Agent';
        } else {
            // Linux and other Unix-like systems
            defaultPath = `${process.env.HOME || '/home/user'}/Luna Agent`;
        }
        
        pathInput.value = defaultPath;
        installationPath = defaultPath;
        startBtn.disabled = false;
        
        showNotification(`Default installation path set: ${defaultPath}`, 'info');
    }
}

async function browseInstallPath() {
    const browseBtn = document.querySelector('#step-path .btn-secondary');
    setButtonLoading(browseBtn, true);
    
    try {
        const result = await ipcRenderer.invoke('browse-install-path');
        
        if (result.success && result.path) {
            const pathInput = document.getElementById('installPath');
            const startBtn = document.getElementById('start-install-btn');
            
            pathInput.value = result.path;
            installationPath = result.path;
            startBtn.disabled = false;
            
            showNotification('Installation path selected successfully', 'success');
        } else if (result.error) {
            showNotification('Error: ' + result.error, 'error');
        }
        // If cancelled, do nothing (no notification)
    } catch (error) {
        console.error('Browse path error:', error);
        showError('Error selecting installation path: ' + error.message);
    } finally {
        setButtonLoading(browseBtn, false);
    }
}

// Step 4: Installation Process
async function startInstallation() {
    if (!installationPath) {
        showError('Please select an installation path first.');
        return;
    }
    
    showStep('step-installing');
    showNotification('Starting Luna installation...', 'info', 6000);
    
    // Listen for progress updates
    const progressUpdateHandler = (event, progress) => {
        updateInstallationProgress(progress);
    };
    
    ipcRenderer.on('installation-progress', progressUpdateHandler);
    
    try {
        const result = await ipcRenderer.invoke('install-luna', installationPath);
        
        // Remove progress listener
        ipcRenderer.removeListener('installation-progress', progressUpdateHandler);
        
        if (result.success) {
            showNotification('Installation completed successfully!', 'success', 8000);
            
            // Add installation details to success page
            if (result.shortcuts) {
                updateSuccessPage(result);
            }
            
            showStep('step-success');
        } else {
            const errorMessage = handlePlatformSpecificError(result.error || 'Unknown error');
            showError('Installation failed: ' + errorMessage);
        }
    } catch (error) {
        console.error('Installation error:', error);
        ipcRenderer.removeListener('installation-progress', progressUpdateHandler);
        
        const errorMessage = handlePlatformSpecificError(error.message);
        showError('Installation error: ' + errorMessage);
    }
}

function updateInstallationProgress(progress) {
    const progressBar = document.getElementById('progress-bar');
    const progressText = document.getElementById('progress-text');
    const progressDetails = document.getElementById('progress-details');
    
    if (progressBar) {
        progressBar.style.width = progress.percentage + '%';
    }
    
    if (progressText) {
        progressText.textContent = `Step ${progress.step} of ${progress.totalSteps} (${progress.percentage}%)`;
    }
    
    if (progressDetails) {
        progressDetails.textContent = progress.message;
    }
    
    // Update document title to show progress
    document.title = `Installing Luna - ${progress.percentage}%`;
    
    // Show progress notifications for major milestones
    if (progress.percentage === 25 || progress.percentage === 50 || progress.percentage === 75) {
        showNotification(`Installation ${progress.percentage}% complete`, 'info', 2000);
    }
}

function updateSuccessPage(installResult) {
    // Add platform-specific success information
    const successInfo = document.querySelector('#step-success .info-box ul');
    
    if (isWindows && installResult.shortcuts) {
        if (installResult.shortcuts.desktop) {
            const li = document.createElement('li');
            li.innerHTML = '🖥️ Desktop shortcut created for quick access';
            successInfo.appendChild(li);
        }
        
        if (installResult.shortcuts.startMenu) {
            const li = document.createElement('li');
            li.innerHTML = '📋 Start Menu entry added to Programs folder';
            successInfo.appendChild(li);
        }
    }
}

// Step 5: Success Actions
async function launchLuna() {
    const launchBtn = document.querySelector('#step-success .btn-success');
    setButtonLoading(launchBtn, true);
    
    try {
        const result = await ipcRenderer.invoke('launch-luna');
        
        if (result.success) {
            showNotification('Luna is starting up! 🚀', 'success');
            
            // Close installer after a brief delay
            setTimeout(() => {
                closeInstaller();
            }, 4000);
        } else {
            showError('Failed to launch Luna: ' + (result.error || 'Unknown error'));
        }
    } catch (error) {
        console.error('Launch error:', error);
        showError('Error launching Luna: ' + error.message);
    } finally {
        setButtonLoading(launchBtn, false);
    }
}

function closeInstaller() {
    showNotification('Thank you for installing Luna! 🌙✨', 'success', 2000);
    setTimeout(() => {
        window.close();
    }, 2000);
}

// Error Handling
function retryInstallation() {
    showStep('step-path');
    showNotification('Ready to retry installation', 'info');
}

function showError(message) {
    showStep('step-error');
    const errorDetails = document.getElementById('error-details');
    if (errorDetails) {
        errorDetails.textContent = message;
    }
    showNotification('Installation error occurred', 'error', 8000);
}

// External Links Handler
async function openExternalLink(url) {
    try {
        await ipcRenderer.invoke('open-external-link', url);
    } catch (error) {
        console.error('Error opening external link:', error);
    }
}

// VirtualBox download helper
async function downloadVirtualBox() {
    try {
        await ipcRenderer.invoke('download-virtualbox');
        showNotification('Opening VirtualBox download page...', 'info');
    } catch (error) {
        console.error('Error opening VirtualBox download:', error);
    }
}

// Easter egg - Luna logo click
function setupEasterEgg() {
    const logo = document.querySelector('.luna-logo');
    let clickCount = 0;
    
    logo.addEventListener('click', () => {
        clickCount++;
        
        if (clickCount === 5) {
            showNotification('🌙 Luna appreciates your enthusiasm! 🌙', 'info', 3000);
            logo.style.animation = 'none';
            setTimeout(() => {
                logo.style.animation = 'gradientShift 2s ease infinite';
            }, 100);
        } else if (clickCount === 10) {
            showNotification('🎉 You\'ve discovered Luna\'s secret! You\'re clearly ready for automation! 🎉', 'success', 5000);
            clickCount = 0;
        }
    });
}

// Accessibility enhancements
function setupAccessibility() {
    // Add keyboard navigation
    document.addEventListener('keydown', (event) => {
        // ESC key handling
        if (event.key === 'Escape') {
            if (currentStep === 'step-success' || currentStep === 'step-error') {
                closeInstaller();
            }
        }
        
        // Enter key handling
        if (event.key === 'Enter' && !event.target.matches('input')) {
            const activeStep = document.querySelector('.installation-step.active');
            if (activeStep) {
                const primaryBtn = activeStep.querySelector('.btn:not(.btn-secondary):not(:disabled)');
                if (primaryBtn && document.activeElement !== primaryBtn) {
                    primaryBtn.click();
                }
            }
        }
        
        // F1 for help
        if (event.key === 'F1') {
            event.preventDefault();
            openExternalLink('https://docs.luna-agent.com');
        }
    });
    
    // Add ARIA labels
    document.querySelectorAll('.btn').forEach(btn => {
        if (!btn.getAttribute('aria-label')) {
            btn.setAttribute('aria-label', btn.textContent.trim());
        }
    });
}

// Event Listeners
document.addEventListener('DOMContentLoaded', () => {
    console.log('Luna Installer UI loaded for', process.platform);
    
    // Initialize platform-specific features
    initializePlatform();
    
    // Set up accessibility features
    setupAccessibility();
    
    // Set up easter egg
    setupEasterEgg();
    
    // Handle external links
    document.addEventListener('click', (event) => {
        if (event.target.tagName === 'A' && event.target.href && event.target.href.startsWith('http')) {
            event.preventDefault();
            openExternalLink(event.target.href);
        }
    });
    
    // Handle window focus events
    window.addEventListener('focus', () => {
        console.log('Luna Installer focused');
    });
    
    window.addEventListener('blur', () => {
        console.log('Luna Installer blurred');
    });
    
    // Initial window title
    updateWindowTitle();
    
    // Show welcome notification
    setTimeout(() => {
        showNotification('Welcome to Luna Agent! 🌙', 'info', 3000);
    }, 1000);
});

// Window lifecycle
window.addEventListener('beforeunload', (event) => {
    // Clean up any listeners
    ipcRenderer.removeAllListeners('installation-progress');
    
    // Show confirmation if installation is in progress
    if (currentStep === 'step-installing') {
        event.returnValue = 'Installation is in progress. Are you sure you want to close?';
        return 'Installation is in progress. Are you sure you want to close?';
    }
});

// Add CSS for additional animations
const additionalStyles = document.createElement('style');
additionalStyles.textContent = `
    @keyframes slideOutRight {
        from {
            transform: translateX(0);
            opacity: 1;
        }
        to {
            transform: translateX(100%);
            opacity: 0;
        }
    }
    
    .requirement-item {
        transition: all 0.3s ease;
    }
    
    .requirement-item:hover {
        transform: translateX(4px);
        background: rgba(255, 255, 255, 0.1) !important;
    }
    
    .luna-logo {
        transition: all 0.3s ease;
    }
    
    .luna-logo:active {
        transform: scale(0.95);
    }
`;
document.head.appendChild(additionalStyles);

// Expose functions to global scope for onclick handlers
window.checkRequirements = checkRequirements;
window.proceedToInstall = proceedToInstall;
window.browseInstallPath = browseInstallPath;
window.startInstallation = startInstallation;
window.launchLuna = launchLuna;
window.closeInstaller = closeInstaller;
window.retryInstallation = retryInstallation;
window.goToStep = goToStep;
window.showError = showError;
window.showNotification = showNotification;
window.openExternalLink = openExternalLink;
window.downloadVirtualBox = downloadVirtualBox;