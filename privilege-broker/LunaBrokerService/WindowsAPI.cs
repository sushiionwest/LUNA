using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32;

namespace LunaBrokerService
{
    public static class WindowsAPI
    {
        #region Windows API Imports

        [DllImport("user32.dll")]
        private static extern bool SetCursorPos(int x, int y);

        [DllImport("user32.dll")]
        private static extern void mouse_event(int dwFlags, int dx, int dy, int cButtons, int dwExtraInfo);

        [DllImport("user32.dll")]
        private static extern bool SendMessage(IntPtr hWnd, uint Msg, int wParam, int lParam);

        [DllImport("user32.dll")]
        private static extern IntPtr FindWindow(string lpClassName, string lpWindowName);

        [DllImport("user32.dll")]
        private static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);

        [DllImport("user32.dll")]
        private static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);

        [DllImport("user32.dll")]
        private static extern int GetClassName(IntPtr hWnd, StringBuilder lpClassName, int nMaxCount);

        [DllImport("user32.dll")]
        private static extern bool IsWindowVisible(IntPtr hWnd);

        [DllImport("user32.dll")]
        private static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

        [DllImport("user32.dll")]
        private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);

        [DllImport("gdi32.dll")]
        private static extern IntPtr CreateDC(string lpszDriver, string lpszDevice, string lpszOutput, IntPtr lpInitData);

        [DllImport("gdi32.dll")]
        private static extern bool DeleteDC(IntPtr hdc);

        [DllImport("gdi32.dll")]
        private static extern IntPtr CreateCompatibleDC(IntPtr hdc);

        [DllImport("gdi32.dll")]
        private static extern IntPtr CreateCompatibleBitmap(IntPtr hdc, int nWidth, int nHeight);

        [DllImport("gdi32.dll")]
        private static extern IntPtr SelectObject(IntPtr hdc, IntPtr hgdiobj);

        [DllImport("gdi32.dll")]
        private static extern bool BitBlt(IntPtr hdcDest, int nXDest, int nYDest, int nWidth, int nHeight,
            IntPtr hdcSrc, int nXSrc, int nYSrc, int dwRop);

        [DllImport("user32.dll")]
        private static extern IntPtr GetDC(IntPtr hWnd);

        [DllImport("user32.dll")]
        private static extern bool ReleaseDC(IntPtr hWnd, IntPtr hDc);

        [DllImport("kernel32.dll")]
        private static extern bool TerminateProcess(IntPtr hProcess, uint uExitCode);

        [DllImport("kernel32.dll")]
        private static extern IntPtr OpenProcess(uint dwDesiredAccess, bool bInheritHandle, int dwProcessId);

        [DllImport("kernel32.dll")]
        private static extern bool CloseHandle(IntPtr hObject);

        private delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

        #endregion

        #region Constants

        private const int MOUSEEVENTF_LEFTDOWN = 0x0002;
        private const int MOUSEEVENTF_LEFTUP = 0x0004;
        private const int MOUSEEVENTF_RIGHTDOWN = 0x0008;
        private const int MOUSEEVENTF_RIGHTUP = 0x0010;
        private const int MOUSEEVENTF_MIDDLEDOWN = 0x0020;
        private const int MOUSEEVENTF_MIDDLEUP = 0x0040;
        private const int PROCESS_TERMINATE = 0x0001;
        private const int SRCCOPY = 0x00CC0020;

        #endregion

        #region Structures

        [StructLayout(LayoutKind.Sequential)]
        private struct RECT
        {
            public int Left, Top, Right, Bottom;
        }

        #endregion

        #region UI Automation Methods

        public static bool PerformClick(int x, int y, string button = "left")
        {
            try
            {
                // Move cursor to position
                SetCursorPos(x, y);

                // Perform the click based on button type
                switch (button.ToLowerInvariant())
                {
                    case "left":
                        mouse_event(MOUSEEVENTF_LEFTDOWN, x, y, 0, 0);
                        System.Threading.Thread.Sleep(50); // Brief delay
                        mouse_event(MOUSEEVENTF_LEFTUP, x, y, 0, 0);
                        break;

                    case "right":
                        mouse_event(MOUSEEVENTF_RIGHTDOWN, x, y, 0, 0);
                        System.Threading.Thread.Sleep(50);
                        mouse_event(MOUSEEVENTF_RIGHTUP, x, y, 0, 0);
                        break;

                    case "middle":
                        mouse_event(MOUSEEVENTF_MIDDLEDOWN, x, y, 0, 0);
                        System.Threading.Thread.Sleep(50);
                        mouse_event(MOUSEEVENTF_MIDDLEUP, x, y, 0, 0);
                        break;

                    default:
                        return false;
                }

                return true;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error performing click: {ex.Message}");
                return false;
            }
        }

        public static bool SendKeys(string keys)
        {
            try
            {
                // Use System.Windows.Forms.SendKeys for key sending
                // Note: This requires System.Windows.Forms reference
                System.Windows.Forms.SendKeys.SendWait(keys);
                return true;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error sending keys: {ex.Message}");
                return false;
            }
        }

        public static List<WindowInfo> GetVisibleWindows()
        {
            var windows = new List<WindowInfo>();

            try
            {
                EnumWindows((hWnd, lParam) =>
                {
                    if (IsWindowVisible(hWnd))
                    {
                        var title = new StringBuilder(256);
                        var className = new StringBuilder(256);
                        
                        GetWindowText(hWnd, title, title.Capacity);
                        GetClassName(hWnd, className, className.Capacity);
                        
                        GetWindowThreadProcessId(hWnd, out uint processId);
                        GetWindowRect(hWnd, out RECT rect);

                        var processName = string.Empty;
                        try
                        {
                            var process = Process.GetProcessById((int)processId);
                            processName = process.ProcessName;
                        }
                        catch
                        {
                            processName = "Unknown";
                        }

                        windows.Add(new WindowInfo
                        {
                            Handle = hWnd,
                            Title = title.ToString(),
                            ClassName = className.ToString(),
                            ProcessId = (int)processId,
                            ProcessName = processName,
                            IsVisible = true,
                            Bounds = new WindowBounds
                            {
                                X = rect.Left,
                                Y = rect.Top,
                                Width = rect.Right - rect.Left,
                                Height = rect.Bottom - rect.Top
                            }
                        });
                    }

                    return true; // Continue enumeration
                }, IntPtr.Zero);
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error getting windows: {ex.Message}");
            }

            return windows;
        }

        #endregion

        #region Registry Methods

        public static object? ReadRegistryValue(string keyPath, string valueName)
        {
            try
            {
                var parts = keyPath.Split('\\', 2);
                if (parts.Length < 2) return null;

                var hive = GetRegistryHive(parts[0]);
                if (hive == null) return null;

                using var key = hive.OpenSubKey(parts[1]);
                return key?.GetValue(valueName);
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error reading registry: {ex.Message}");
                return null;
            }
        }

        public static bool WriteRegistryValue(string keyPath, string valueName, object value)
        {
            try
            {
                var parts = keyPath.Split('\\', 2);
                if (parts.Length < 2) return false;

                var hive = GetRegistryHive(parts[0]);
                if (hive == null) return false;

                using var key = hive.CreateSubKey(parts[1]);
                key?.SetValue(valueName, value);
                return true;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error writing registry: {ex.Message}");
                return false;
            }
        }

        private static RegistryKey? GetRegistryHive(string hiveName)
        {
            return hiveName.ToUpperInvariant() switch
            {
                "HKEY_CURRENT_USER" => Registry.CurrentUser,
                "HKEY_LOCAL_MACHINE" => Registry.LocalMachine,
                "HKEY_CLASSES_ROOT" => Registry.ClassesRoot,
                "HKEY_USERS" => Registry.Users,
                "HKEY_CURRENT_CONFIG" => Registry.CurrentConfig,
                _ => null
            };
        }

        #endregion

        #region Process Methods

        public static int StartProcess(string fileName, string? arguments = null)
        {
            try
            {
                var startInfo = new ProcessStartInfo
                {
                    FileName = fileName,
                    Arguments = arguments ?? string.Empty,
                    UseShellExecute = false,
                    CreateNoWindow = true
                };

                var process = Process.Start(startInfo);
                return process?.Id ?? -1;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error starting process: {ex.Message}");
                return -1;
            }
        }

        public static bool TerminateProcess(int processId)
        {
            try
            {
                var process = Process.GetProcessById(processId);
                process.Kill();
                return true;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error terminating process: {ex.Message}");
                return false;
            }
        }

        #endregion

        #region Screenshot Methods

        public static string TakeScreenshot()
        {
            try
            {
                var bounds = System.Windows.Forms.Screen.PrimaryScreen.Bounds;
                using var bitmap = new Bitmap(bounds.Width, bounds.Height);
                using var graphics = Graphics.FromImage(bitmap);
                
                graphics.CopyFromScreen(Point.Empty, Point.Empty, bounds.Size);

                var fileName = $"screenshot_{DateTime.Now:yyyyMMdd_HHmmss}.png";
                var filePath = Path.Combine(@"C:\ProgramData\Luna\Screenshots", fileName);
                
                Directory.CreateDirectory(Path.GetDirectoryName(filePath)!);
                bitmap.Save(filePath, ImageFormat.Png);

                return filePath;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error taking screenshot: {ex.Message}");
                return string.Empty;
            }
        }

        #endregion
    }
}