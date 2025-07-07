/*!
 * Luna Smart Bootstrap Installer
 * 
 * Lightweight 5MB installer that downloads and installs Luna automatically
 */

#include <windows.h>
#include <wininet.h>
#include <shellapi.h>
#include <shlobj.h>
#include <commdlg.h>
#include <commctrl.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <thread>
#include <mutex>

#pragma comment(lib, "wininet.lib")
#pragma comment(lib, "shell32.lib")
#pragma comment(lib, "comctl32.lib")
#pragma comment(lib, "shlwapi.lib")

// Constants
const wchar_t* APP_NAME = L"Luna Visual AI Installer";
const wchar_t* DOWNLOAD_URL = L"https://github.com/sushiionwest/LUNA/releases/latest/download/Luna-Setup.msi";
const wchar_t* COMPATIBILITY_URL = L"https://api.github.com/repos/sushiionwest/LUNA/releases/latest";
const DWORD DOWNLOAD_BUFFER_SIZE = 8192;
const DWORD MIN_DISK_SPACE_MB = 200;
const DWORD MIN_RAM_MB = 512;

// Global variables
HWND g_hMainWindow = nullptr;
HWND g_hProgressBar = nullptr;
HWND g_hStatusText = nullptr;
HWND g_hCancelButton = nullptr;
bool g_bCancelDownload = false;
std::mutex g_StatusMutex;

// Forward declarations
LRESULT CALLBACK WindowProc(HWND hwnd, UINT uMsg, WPARAM wParam, LPARAM lParam);
bool CheckSystemCompatibility();
bool HasAdminRights();
bool RequestAdminRights();
void DownloadAndInstall();
void UpdateStatus(const wchar_t* status);
void UpdateProgress(int percentage);
bool DownloadFile(const wchar_t* url, const wchar_t* localPath);
bool RunInstaller(const wchar_t* installerPath);
void ShowError(const wchar_t* message);
void ShowSuccess();
std::wstring GetTempPath();
std::wstring GetDownloadPath();

// Entry point
int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    // Initialize common controls
    INITCOMMONCONTROLSEX icex;
    icex.dwSize = sizeof(INITCOMMONCONTROLSEX);
    icex.dwICC = ICC_PROGRESS_CLASS | ICC_STANDARD_CLASSES;
    InitCommonControlsEx(&icex);

    // Register window class
    WNDCLASSEX wcex = {};
    wcex.cbSize = sizeof(WNDCLASSEX);
    wcex.style = CS_HREDRAW | CS_VREDRAW;
    wcex.lpfnWndProc = WindowProc;
    wcex.hInstance = hInstance;
    wcex.hIcon = LoadIcon(hInstance, MAKEINTRESOURCE(101));
    wcex.hCursor = LoadCursor(nullptr, IDC_ARROW);
    wcex.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1);
    wcex.lpszClassName = L"LunaInstallerClass";
    wcex.hIconSm = LoadIcon(hInstance, MAKEINTRESOURCE(101));

    if (!RegisterClassEx(&wcex)) {
        ShowError(L"Failed to register window class");
        return 1;
    }

    // Create main window
    g_hMainWindow = CreateWindowEx(
        WS_EX_APPWINDOW,
        L"LunaInstallerClass",
        APP_NAME,
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        CW_USEDEFAULT, CW_USEDEFAULT, 500, 300,
        nullptr, nullptr, hInstance, nullptr
    );

    if (!g_hMainWindow) {
        ShowError(L"Failed to create window");
        return 1;
    }

    ShowWindow(g_hMainWindow, nCmdShow);
    UpdateWindow(g_hMainWindow);

    // Check system compatibility first
    if (!CheckSystemCompatibility()) {
        return 1;
    }

    // Start download and installation in background thread
    std::thread installThread(DownloadAndInstall);
    installThread.detach();

    // Message loop
    MSG msg;
    while (GetMessage(&msg, nullptr, 0, 0)) {
        TranslateMessage(&msg);
        DispatchMessage(&msg);
    }

    return (int)msg.wParam;
}

// Window procedure
LRESULT CALLBACK WindowProc(HWND hwnd, UINT uMsg, WPARAM wParam, LPARAM lParam) {
    switch (uMsg) {
    case WM_CREATE:
        {
            // Create Luna branding
            CreateWindow(L"STATIC", L"🌙 Luna Visual AI",
                        WS_VISIBLE | WS_CHILD | SS_CENTER,
                        50, 20, 400, 40,
                        hwnd, nullptr, GetModuleHandle(nullptr), nullptr);

            // Create description
            CreateWindow(L"STATIC", L"Installing your AI-powered computer assistant...\nThis will take about 30 seconds.",
                        WS_VISIBLE | WS_CHILD | SS_CENTER,
                        50, 70, 400, 40,
                        hwnd, nullptr, GetModuleHandle(nullptr), nullptr);

            // Create progress bar
            g_hProgressBar = CreateWindow(PROGRESS_CLASS, nullptr,
                                        WS_VISIBLE | WS_CHILD | PBS_SMOOTH,
                                        50, 130, 400, 25,
                                        hwnd, nullptr, GetModuleHandle(nullptr), nullptr);
            SendMessage(g_hProgressBar, PBM_SETRANGE, 0, MAKELPARAM(0, 100));

            // Create status text
            g_hStatusText = CreateWindow(L"STATIC", L"Checking system compatibility...",
                                       WS_VISIBLE | WS_CHILD | SS_CENTER,
                                       50, 170, 400, 20,
                                       hwnd, nullptr, GetModuleHandle(nullptr), nullptr);

            // Create cancel button
            g_hCancelButton = CreateWindow(L"BUTTON", L"Cancel",
                                         WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON,
                                         200, 210, 100, 30,
                                         hwnd, (HMENU)IDCANCEL, GetModuleHandle(nullptr), nullptr);
        }
        break;

    case WM_COMMAND:
        if (LOWORD(wParam) == IDCANCEL) {
            g_bCancelDownload = true;
            PostQuitMessage(0);
        }
        break;

    case WM_CLOSE:
        g_bCancelDownload = true;
        PostQuitMessage(0);
        break;

    case WM_DESTROY:
        PostQuitMessage(0);
        break;

    default:
        return DefWindowProc(hwnd, uMsg, wParam, lParam);
    }
    return 0;
}

// Check if system meets requirements
bool CheckSystemCompatibility() {
    UpdateStatus(L"Checking system requirements...");
    UpdateProgress(10);

    // Check Windows version
    OSVERSIONINFOEX osvi = {};
    osvi.dwOSVersionInfoSize = sizeof(OSVERSIONINFOEX);
    if (!GetVersionEx((OSVERSIONINFO*)&osvi)) {
        ShowError(L"Unable to determine Windows version");
        return false;
    }

    if (osvi.dwMajorVersion < 10) {
        ShowError(L"Luna requires Windows 10 or later");
        return false;
    }

    // Check architecture
    SYSTEM_INFO si;
    GetSystemInfo(&si);
    if (si.wProcessorArchitecture != PROCESSOR_ARCHITECTURE_AMD64) {
        ShowError(L"Luna requires 64-bit Windows");
        return false;
    }

    // Check memory
    MEMORYSTATUSEX memInfo = {};
    memInfo.dwLength = sizeof(MEMORYSTATUSEX);
    GlobalMemoryStatusEx(&memInfo);
    
    DWORD totalMemoryMB = (DWORD)(memInfo.ullTotalPhys / (1024 * 1024));
    if (totalMemoryMB < MIN_RAM_MB) {
        wchar_t errorMsg[256];
        swprintf_s(errorMsg, L"Luna requires at least %d MB of RAM. You have %d MB.", MIN_RAM_MB, totalMemoryMB);
        ShowError(errorMsg);
        return false;
    }

    // Check disk space
    ULARGE_INTEGER freeBytesAvailable, totalNumberOfBytes, totalNumberOfFreeBytes;
    wchar_t windowsDir[MAX_PATH];
    GetWindowsDirectory(windowsDir, MAX_PATH);
    windowsDir[3] = L'\0'; // Keep only drive letter

    if (GetDiskFreeSpaceEx(windowsDir, &freeBytesAvailable, &totalNumberOfBytes, &totalNumberOfFreeBytes)) {
        DWORD freeSpaceMB = (DWORD)(freeBytesAvailable.QuadPart / (1024 * 1024));
        if (freeSpaceMB < MIN_DISK_SPACE_MB) {
            wchar_t errorMsg[256];
            swprintf_s(errorMsg, L"Luna requires at least %d MB of free disk space. You have %d MB.", MIN_DISK_SPACE_MB, freeSpaceMB);
            ShowError(errorMsg);
            return false;
        }
    }

    // Check admin rights
    if (!HasAdminRights()) {
        UpdateStatus(L"Requesting administrator privileges...");
        if (!RequestAdminRights()) {
            ShowError(L"Administrator privileges are required to install Luna");
            return false;
        }
        // If we requested admin rights, the process will restart
        return false;
    }

    UpdateStatus(L"System compatibility check passed ✓");
    UpdateProgress(20);
    return true;
}

// Check if running as administrator
bool HasAdminRights() {
    BOOL isAdmin = FALSE;
    PSID adminGroup = nullptr;
    SID_IDENTIFIER_AUTHORITY ntAuthority = SECURITY_NT_AUTHORITY;

    if (AllocateAndInitializeSid(&ntAuthority, 2, SECURITY_BUILTIN_DOMAIN_RID,
                                DOMAIN_ALIAS_RID_ADMINS, 0, 0, 0, 0, 0, 0, &adminGroup)) {
        CheckTokenMembership(nullptr, adminGroup, &isAdmin);
        FreeSid(adminGroup);
    }

    return isAdmin == TRUE;
}

// Request administrator privileges
bool RequestAdminRights() {
    wchar_t exePath[MAX_PATH];
    GetModuleFileName(nullptr, exePath, MAX_PATH);

    SHELLEXECUTEINFO sei = {};
    sei.cbSize = sizeof(SHELLEXECUTEINFO);
    sei.lpVerb = L"runas";
    sei.lpFile = exePath;
    sei.hwnd = g_hMainWindow;
    sei.nShow = SW_NORMAL;

    return ShellExecuteEx(&sei) == TRUE;
}

// Main download and installation function
void DownloadAndInstall() {
    try {
        UpdateStatus(L"Preparing download...");
        UpdateProgress(30);

        // Get download path
        std::wstring downloadPath = GetDownloadPath();
        
        UpdateStatus(L"Downloading Luna Visual AI...");
        UpdateProgress(40);

        // Download the main installer
        if (!DownloadFile(DOWNLOAD_URL, downloadPath.c_str())) {
            if (!g_bCancelDownload) {
                ShowError(L"Failed to download Luna installer");
            }
            return;
        }

        if (g_bCancelDownload) return;

        UpdateStatus(L"Download complete. Installing...");
        UpdateProgress(90);

        // Run the installer
        if (!RunInstaller(downloadPath.c_str())) {
            ShowError(L"Failed to run Luna installer");
            return;
        }

        UpdateStatus(L"Installation complete!");
        UpdateProgress(100);

        // Clean up downloaded file
        DeleteFile(downloadPath.c_str());

        ShowSuccess();
    }
    catch (...) {
        if (!g_bCancelDownload) {
            ShowError(L"An unexpected error occurred during installation");
        }
    }
}

// Update status text
void UpdateStatus(const wchar_t* status) {
    std::lock_guard<std::mutex> lock(g_StatusMutex);
    if (g_hStatusText) {
        SetWindowText(g_hStatusText, status);
    }
}

// Update progress bar
void UpdateProgress(int percentage) {
    if (g_hProgressBar) {
        SendMessage(g_hProgressBar, PBM_SETPOS, percentage, 0);
    }
}

// Download file with progress updates
bool DownloadFile(const wchar_t* url, const wchar_t* localPath) {
    HINTERNET hInternet = InternetOpen(L"Luna Installer", INTERNET_OPEN_TYPE_PRECONFIG, nullptr, nullptr, 0);
    if (!hInternet) return false;

    HINTERNET hUrl = InternetOpenUrl(hInternet, url, nullptr, 0, INTERNET_FLAG_RELOAD, 0);
    if (!hUrl) {
        InternetCloseHandle(hInternet);
        return false;
    }

    // Get file size
    DWORD fileSize = 0;
    DWORD bufferSize = sizeof(fileSize);
    HttpQueryInfo(hUrl, HTTP_QUERY_CONTENT_LENGTH | HTTP_QUERY_FLAG_NUMBER, &fileSize, &bufferSize, nullptr);

    // Create local file
    HANDLE hFile = CreateFile(localPath, GENERIC_WRITE, 0, nullptr, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (hFile == INVALID_HANDLE_VALUE) {
        InternetCloseHandle(hUrl);
        InternetCloseHandle(hInternet);
        return false;
    }

    // Download with progress updates
    BYTE buffer[DOWNLOAD_BUFFER_SIZE];
    DWORD bytesRead, bytesWritten;
    DWORD totalBytesRead = 0;
    int lastProgress = 40;

    while (InternetReadFile(hUrl, buffer, DOWNLOAD_BUFFER_SIZE, &bytesRead) && bytesRead > 0) {
        if (g_bCancelDownload) break;

        WriteFile(hFile, buffer, bytesRead, &bytesWritten, nullptr);
        totalBytesRead += bytesRead;

        // Update progress (40% to 85% of total progress)
        if (fileSize > 0) {
            int progress = 40 + (int)((double)totalBytesRead / fileSize * 45);
            if (progress > lastProgress) {
                UpdateProgress(progress);
                lastProgress = progress;
            }
        }

        // Update status with download size
        wchar_t statusText[256];
        swprintf_s(statusText, L"Downloaded %d MB / %d MB", totalBytesRead / (1024*1024), fileSize / (1024*1024));
        UpdateStatus(statusText);
    }

    CloseHandle(hFile);
    InternetCloseHandle(hUrl);
    InternetCloseHandle(hInternet);

    return !g_bCancelDownload && totalBytesRead > 0;
}

// Run the downloaded installer
bool RunInstaller(const wchar_t* installerPath) {
    SHELLEXECUTEINFO sei = {};
    sei.cbSize = sizeof(SHELLEXECUTEINFO);
    sei.fMask = SEE_MASK_NOCLOSEPROCESS;
    sei.lpVerb = L"open";
    sei.lpFile = installerPath;
    sei.lpParameters = L"/quiet AUTOSTART=1"; // Silent install with auto-start
    sei.nShow = SW_HIDE;

    if (!ShellExecuteEx(&sei)) {
        return false;
    }

    // Wait for installer to complete
    if (sei.hProcess) {
        WaitForSingleObject(sei.hProcess, INFINITE);
        
        DWORD exitCode;
        GetExitCodeProcess(sei.hProcess, &exitCode);
        CloseHandle(sei.hProcess);
        
        return exitCode == 0;
    }

    return false;
}

// Show error message
void ShowError(const wchar_t* message) {
    MessageBox(g_hMainWindow, message, L"Luna Installer Error", MB_OK | MB_ICONERROR);
    PostQuitMessage(1);
}

// Show success message and exit
void ShowSuccess() {
    EnableWindow(g_hCancelButton, FALSE);
    SetWindowText(g_hCancelButton, L"Close");
    
    UpdateStatus(L"🎉 Luna Visual AI installed successfully!");
    
    MessageBox(g_hMainWindow, 
               L"Luna Visual AI has been installed successfully!\n\n"
               L"You can now:\n"
               L"• Find Luna in your Start Menu\n"
               L"• Use the desktop shortcut\n"
               L"• Luna will start automatically next time you boot\n\n"
               L"Try saying: 'Click the Start button' or 'Open Control Panel'",
               L"Installation Complete", 
               MB_OK | MB_ICONINFORMATION);
    
    PostQuitMessage(0);
}

// Get temporary download path
std::wstring GetDownloadPath() {
    wchar_t tempPath[MAX_PATH];
    GetTempPath(MAX_PATH, tempPath);
    
    std::wstring downloadPath = tempPath;
    downloadPath += L"Luna-Setup.msi";
    
    return downloadPath;
}