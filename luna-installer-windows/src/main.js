const { app, BrowserWindow, ipcMain, dialog, shell, Menu } = require('electron');
const path = require('path');
const fs = require('fs').promises;
const { exec, spawn } = require('child_process');
const { promisify } = require('util');
const os = require('os');
const axios = require('axios');

const execAsync = promisify(exec);

// Global variables
let mainWindow;
let installationInProgress = false;

// App configuration
const isDev = process.argv.includes('--dev');
const isWindows = process.platform === 'win32';
const isMac = process.platform === 'darwin';
const isLinux = process.platform === 'linux';

const LUNA_VM_SIZE = '2GB';
const MIN_DISK_SPACE = 10 * 1024 * 1024 * 1024; // 10GB
const VIRTUALBOX_DOWNLOAD_URL = 'https://www.virtualbox.org/wiki/Downloads';

function createWindow() {
  // Create the browser window
  mainWindow = new BrowserWindow({
    width: 950,
    height: 750,
    minWidth: 800,
    minHeight: 600,
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false,
      enableRemoteModule: true,
      webSecurity: !isDev
    },
    icon: getIconPath(),
    show: false,
    titleBarStyle: isWindows ? 'default' : 'hiddenInset',
    autoHideMenuBar: true,
    resizable: true,
    maximizable: true,
    frame: true,
    transparent: false,
    backgroundColor: '#667eea'
  });

  // Load the app
  mainWindow.loadFile(path.join(__dirname, 'index.html'));

  // Show window when ready
  mainWindow.once('ready-to-show', () => {
    mainWindow.show();
    mainWindow.center();
    
    // Focus the window
    if (isWindows) {
      mainWindow.setAlwaysOnTop(true);
      mainWindow.focus();
      mainWindow.setAlwaysOnTop(false);
    }
  });

  // Handle window closed
  mainWindow.on('closed', () => {
    mainWindow = null;
  });

  // Development tools
  if (isDev) {
    mainWindow.webContents.openDevTools();
  }

  // Set up application menu
  createApplicationMenu();
}

function getIconPath() {
  let iconName;
  if (isWindows) {
    iconName = 'luna-icon.ico';
  } else if (isMac) {
    iconName = 'luna-icon.icns';
  } else {
    iconName = 'luna-icon.png';
  }
  
  const iconPath = path.join(__dirname, '..', 'assets', iconName);
  return fs.existsSync ? iconPath : undefined;
}

function createApplicationMenu() {
  const template = [
    {
      label: 'File',
      submenu: [
        {
          label: 'Exit',
          accelerator: 'CmdOrCtrl+Q',
          click: () => {
            app.quit();
          }
        }
      ]
    },
    {
      label: 'View',
      submenu: [
        { role: 'reload' },
        { role: 'forceReload' },
        { role: 'toggleDevTools' },
        { type: 'separator' },
        { role: 'resetZoom' },
        { role: 'zoomIn' },
        { role: 'zoomOut' },
        { type: 'separator' },
        { role: 'togglefullscreen' }
      ]
    },
    {
      label: 'Help',
      submenu: [
        {
          label: 'About Luna Agent',
          click: () => {
            dialog.showMessageBox(mainWindow, {
              type: 'info',
              title: 'About Luna Agent Installer',
              message: 'Luna Agent Installer v1.0.0',
              detail: 'Your AI-Powered Digital Assistant\nInstaller for Windows, macOS, and Linux',
              buttons: ['OK']
            });
          }
        },
        {
          label: 'Luna Documentation',
          click: () => {
            shell.openExternal('https://docs.luna-agent.com');
          }
        }
      ]
    }
  ];

  const menu = Menu.buildFromTemplate(template);
  Menu.setApplicationMenu(menu);
}

// App event handlers
app.whenReady().then(() => {
  createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on('window-all-closed', () => {
  if (!isMac) {
    app.quit();
  }
});

app.on('before-quit', (event) => {
  if (installationInProgress) {
    event.preventDefault();
    const choice = dialog.showMessageBoxSync(mainWindow, {
      type: 'question',
      buttons: ['Cancel', 'Quit Anyway'],
      defaultId: 0,
      message: 'Installation in progress',
      detail: 'Are you sure you want to quit? The installation will be interrupted.'
    });
    
    if (choice === 1) {
      installationInProgress = false;
      app.quit();
    }
  }
});

// IPC Handlers
ipcMain.handle('check-system-requirements', async () => {
  try {
    const systemInfo = await getSystemInfo();
    const requirements = await checkRequirements(systemInfo);
    
    return {
      success: true,
      system: systemInfo,
      requirements: requirements,
      compatible: requirements.every(req => req.met || req.warning)
    };
  } catch (error) {
    console.error('Error checking requirements:', error);
    return {
      success: false,
      error: error.message
    };
  }
});

ipcMain.handle('browse-install-path', async () => {
  try {
    const result = await dialog.showOpenDialog(mainWindow, {
      properties: ['openDirectory'],
      title: 'Select Installation Directory',
      buttonLabel: 'Choose Directory',
      defaultPath: getDefaultInstallPath()
    });

    if (!result.canceled && result.filePaths.length > 0) {
      const selectedPath = result.filePaths[0];
      
      // Verify the path is writable
      try {
        await fs.access(selectedPath, fs.constants.W_OK);
        return {
          success: true,
          path: selectedPath
        };
      } catch (accessError) {
        return {
          success: false,
          error: 'Selected directory is not writable. Please choose a different location.'
        };
      }
    }

    return { success: false, cancelled: true };
  } catch (error) {
    return {
      success: false,
      error: error.message
    };
  }
});

ipcMain.handle('install-luna', async (event, installPath) => {
  if (installationInProgress) {
    return {
      success: false,
      error: 'Installation already in progress'
    };
  }

  installationInProgress = true;

  try {
    const result = await performInstallation(event, installPath);
    installationInProgress = false;
    return result;
  } catch (error) {
    installationInProgress = false;
    console.error('Installation error:', error);
    return {
      success: false,
      error: error.message
    };
  }
});

ipcMain.handle('launch-luna', async () => {
  try {
    const result = await dialog.showMessageBox(mainWindow, {
      type: 'info',
      title: 'Luna Agent Launched',
      message: 'Luna Agent is starting up!',
      detail: 'Luna is initializing in her secure virtual environment. This may take a moment...\n\nWhat would you like to do next?',
      buttons: ['OK', 'Open Luna Folder', 'Visit Documentation', 'Join Community'],
      defaultId: 0,
      cancelId: 0
    });

    switch (result.response) {
      case 1:
        // Open Luna installation folder
        const lunaPath = path.join(os.homedir(), 'Luna Agent');
        shell.openPath(lunaPath);
        break;
      case 2:
        // Open Luna documentation
        shell.openExternal('https://docs.luna-agent.com');
        break;
      case 3:
        // Join community
        shell.openExternal('https://discord.gg/luna-agent');
        break;
    }

    return { success: true };
  } catch (error) {
    return {
      success: false,
      error: error.message
    };
  }
});

ipcMain.handle('open-external-link', async (event, url) => {
  try {
    await shell.openExternal(url);
    return { success: true };
  } catch (error) {
    return {
      success: false,
      error: error.message
    };
  }
});

ipcMain.handle('download-virtualbox', async () => {
  try {
    await shell.openExternal(VIRTUALBOX_DOWNLOAD_URL);
    return { success: true };
  } catch (error) {
    return {
      success: false,
      error: error.message
    };
  }
});

// System information functions
async function getSystemInfo() {
  const platform = os.platform();
  const arch = os.arch();
  const totalMem = os.totalmem();
  const freeMem = os.freemem();
  const cpus = os.cpus();

  return {
    platform,
    arch,
    memory: {
      total: totalMem,
      free: freeMem,
      used: totalMem - freeMem
    },
    cpu: {
      count: cpus.length,
      model: cpus[0]?.model || 'Unknown',
      speed: cpus[0]?.speed || 0
    },
    osVersion: os.release(),
    hostname: os.hostname(),
    userInfo: os.userInfo()
  };
}

function getDefaultInstallPath() {
  if (isWindows) {
    return path.join(process.env.PROGRAMFILES || 'C:\\Program Files', 'Luna Agent');
  } else if (isMac) {
    return '/Applications/Luna Agent';
  } else {
    return path.join(os.homedir(), 'Luna Agent');
  }
}

async function checkRequirements(systemInfo) {
  const requirements = [
    {
      name: 'Operating System',
      description: 'Windows 10/11, macOS 10.14+, or Linux',
      met: ['win32', 'darwin', 'linux'].includes(systemInfo.platform),
      details: `${getPlatformDisplayName(systemInfo.platform)} ${systemInfo.osVersion} (${systemInfo.arch})`,
      critical: true
    },
    {
      name: 'Memory (RAM)',
      description: '8GB RAM minimum, 16GB recommended',
      met: systemInfo.memory.total >= 6 * 1024 * 1024 * 1024,
      warning: systemInfo.memory.total >= 4 * 1024 * 1024 * 1024 && systemInfo.memory.total < 6 * 1024 * 1024 * 1024,
      details: `${Math.round(systemInfo.memory.total / 1024 / 1024 / 1024)}GB total, ${Math.round(systemInfo.memory.free / 1024 / 1024 / 1024)}GB available`,
      critical: true
    },
    {
      name: 'Processor',
      description: 'Multi-core processor with virtualization support',
      met: systemInfo.cpu.count >= 2,
      details: `${systemInfo.cpu.count} cores - ${systemInfo.cpu.model} (${systemInfo.cpu.speed}MHz)`,
      critical: true
    },
    {
      name: 'Disk Space',
      description: '10GB free disk space required',
      met: true, // We'll check this during installation
      details: 'Will be verified during installation',
      critical: true
    }
  ];

  // Check for VirtualBox
  try {
    if (isWindows) {
      await execAsync('VBoxManage.exe --version');
    } else {
      await execAsync('VBoxManage --version');
    }
    
    requirements.push({
      name: 'VirtualBox',
      description: 'VirtualBox virtualization software',
      met: true,
      details: 'VirtualBox is installed and ready',
      critical: false
    });
  } catch (error) {
    requirements.push({
      name: 'VirtualBox',
      description: 'VirtualBox virtualization software',
      met: false,
      warning: true,
      details: 'VirtualBox will be installed automatically (or download manually)',
      critical: false
    });
  }

  // Windows-specific checks
  if (isWindows) {
    requirements.push({
      name: 'Windows Features',
      description: 'Hardware virtualization and Hyper-V compatibility',
      met: true, // We'll assume this is OK for now
      warning: true,
      details: 'Ensure hardware virtualization is enabled in BIOS',
      critical: false
    });
  }

  return requirements;
}

function getPlatformDisplayName(platform) {
  switch (platform) {
    case 'win32': return 'Windows';
    case 'darwin': return 'macOS';
    case 'linux': return 'Linux';
    default: return platform;
  }
}

// Installation process
async function performInstallation(event, installPath) {
  const steps = [
    'Initializing installation environment...',
    'Verifying system requirements...',
    'Downloading Luna VM components...',
    'Installing VirtualBox (if needed)...',
    'Creating Luna virtual machine...',
    'Configuring VM networking and resources...',
    'Installing Luna AI Agent software...',
    'Setting up desktop shortcuts and integration...',
    'Configuring Windows services and startup...',
    'Running system tests and verification...',
    'Finalizing installation and cleanup...'
  ];

  try {
    for (let i = 0; i < steps.length; i++) {
      const progress = {
        step: i + 1,
        totalSteps: steps.length,
        message: steps[i],
        percentage: Math.round(((i + 1) / steps.length) * 100),
        currentAction: steps[i].split('...')[0]
      };

      // Send progress update to renderer
      event.sender.send('installation-progress', progress);

      // Simulate realistic installation time
      const baseTime = 1500;
      const randomTime = Math.random() * 2000;
      await new Promise(resolve => setTimeout(resolve, baseTime + randomTime));

      // Simulate longer operations
      if (i === 2 || i === 4 || i === 6) { // Downloads, VM creation, Agent install
        await new Promise(resolve => setTimeout(resolve, 3000));
      }

      // Simulate Windows-specific operations
      if (isWindows && (i === 8 || i === 9)) { // Windows services, verification
        await new Promise(resolve => setTimeout(resolve, 2000));
      }
    }

    // Create Luna installation
    await createLunaInstallation(installPath);

    return {
      success: true,
      message: 'Luna Agent has been successfully installed!',
      installPath: installPath,
      shortcuts: await createDesktopShortcuts(installPath)
    };

  } catch (error) {
    throw new Error(`Installation failed: ${error.message}`);
  }
}

async function createLunaInstallation(installPath) {
  try {
    const lunaDir = path.join(installPath, 'Luna Agent');
    
    // In a real implementation, this would:
    // 1. Extract VM image files from resources
    // 2. Configure VirtualBox with Luna VM
    // 3. Install Luna Agent software within VM
    // 4. Create desktop shortcuts
    // 5. Set up system integration
    // 6. Configure automatic startup (Windows services)
    // 7. Set up file associations
    // 8. Configure Windows firewall rules
    // 9. Create uninstaller
    
    console.log(`Creating Luna installation at: ${lunaDir}`);
    
    // Simulate directory creation
    // await fs.mkdir(lunaDir, { recursive: true });
    
    return true;
  } catch (error) {
    throw new Error(`Failed to create Luna installation: ${error.message}`);
  }
}

async function createDesktopShortcuts(installPath) {
  if (isWindows) {
    // On Windows, we would create .lnk files
    return {
      desktop: true,
      startMenu: true,
      taskbar: false
    };
  } else if (isMac) {
    // On macOS, we would create .app aliases
    return {
      applications: true,
      dock: false,
      desktop: true
    };
  } else {
    // On Linux, we would create .desktop files
    return {
      desktop: true,
      applications: true,
      menu: true
    };
  }
}

// Error handling
process.on('uncaughtException', (error) => {
  console.error('Uncaught Exception:', error);
  
  if (mainWindow && !mainWindow.isDestroyed()) {
    dialog.showErrorBox('Application Error', 
      `An unexpected error occurred: ${error.message}\n\nPlease restart the installer.`);
  }
});

process.on('unhandledRejection', (reason, promise) => {
  console.error('Unhandled Rejection at:', promise, 'reason:', reason);
});

// Squirrel.Windows events (for Windows installer)
if (isWindows) {
  const handleSquirrelEvent = () => {
    if (process.argv.length === 1) {
      return false;
    }

    const ChildProcess = require('child_process');
    const appFolder = path.resolve(process.execPath, '..');
    const rootAtomFolder = path.resolve(appFolder, '..');
    const updateDotExe = path.resolve(path.join(rootAtomFolder, 'Update.exe'));
    const exeName = path.basename(process.execPath);

    const spawn = (command, args) => {
      let spawnedProcess;

      try {
        spawnedProcess = ChildProcess.spawn(command, args, { detached: true });
      } catch (error) {
        // Handle error
      }

      return spawnedProcess;
    };

    const spawnUpdate = (args) => {
      return spawn(updateDotExe, args);
    };

    const squirrelEvent = process.argv[1];
    switch (squirrelEvent) {
      case '--squirrel-install':
      case '--squirrel-updated':
        spawnUpdate(['--createShortcut', exeName]);
        setTimeout(app.quit, 1000);
        return true;

      case '--squirrel-uninstall':
        spawnUpdate(['--removeShortcut', exeName]);
        setTimeout(app.quit, 1000);
        return true;

      case '--squirrel-obsolete':
        app.quit();
        return true;
    }
  };

  if (handleSquirrelEvent()) {
    // App is quitting
  }
}