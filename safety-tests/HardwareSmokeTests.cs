using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace LunaSafetyTests
{
    [TestClass]
    public class HardwareSmokeTests
    {
        private const int MAX_EXECUTION_TIME_MS = 10000; // 10 seconds max per test
        private const int SAFE_OPERATION_DELAY_MS = 100; // Minimum delay between operations

        [DllImport("user32.dll")]
        private static extern bool SetCursorPos(int x, int y);

        [DllImport("user32.dll")]
        private static extern void mouse_event(int dwFlags, int dx, int dy, int cButtons, int dwExtraInfo);

        [DllImport("user32.dll")]
        private static extern IntPtr FindWindow(string lpClassName, string lpWindowName);

        [DllImport("user32.dll")]
        private static extern bool IsWindow(IntPtr hWnd);

        private const int MOUSEEVENTF_LEFTDOWN = 0x0002;
        private const int MOUSEEVENTF_LEFTUP = 0x0004;

        [TestMethod]
        [TestCategory("Hardware-Smoke")]
        [Timeout(15000)] // 15 seconds
        public async Task HardwareSmoke_SafeClickOperations_ShouldCompleteWithinLimits()
        {
            // Arrange
            var safetyMonitor = new SimpleSafetyMonitor();
            var clickCount = 0;
            var startTime = DateTime.UtcNow;
            var maxClicks = 10; // Limited number of clicks for safety

            // Act
            var clickTask = Task.Run(() =>
            {
                try
                {
                    for (int i = 0; i < maxClicks; i++)
                    {
                        if (safetyMonitor.ShouldStop())
                        {
                            break;
                        }

                        // Perform safe click in safe area (middle of screen)
                        var screenCenter = GetSafeScreenCenter();
                        SetCursorPos(screenCenter.X, screenCenter.Y);
                        
                        // Brief delay to prevent rapid-fire clicking
                        Thread.Sleep(SAFE_OPERATION_DELAY_MS);
                        
                        // Simulate mouse click
                        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
                        Thread.Sleep(50);
                        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
                        
                        clickCount++;
                        safetyMonitor.RecordOperation("click");
                    }
                }
                catch (Exception ex)
                {
                    safetyMonitor.RecordError(ex);
                    throw;
                }
            });

            var completed = await Task.WhenAny(clickTask, Task.Delay(MAX_EXECUTION_TIME_MS));
            
            // Assert
            Assert.AreEqual(clickTask, completed, "Click operations should complete within time limit");
            Assert.IsTrue(clickCount <= maxClicks, $"Should not exceed {maxClicks} clicks");
            
            var duration = DateTime.UtcNow - startTime;
            var expectedMinDuration = TimeSpan.FromMilliseconds(maxClicks * SAFE_OPERATION_DELAY_MS);
            Assert.IsTrue(duration >= expectedMinDuration, 
                "Operations should respect minimum delay between clicks");
            
            Assert.IsFalse(safetyMonitor.HasErrors(), "No errors should occur during safe operations");
        }

        [TestMethod]
        [TestCategory("Hardware-Smoke")]
        [Timeout(12000)] // 12 seconds
        public async Task HardwareSmoke_SafeTypingOperations_ShouldNotLeakSensitiveData()
        {
            // Arrange
            var safetyMonitor = new SimpleSafetyMonitor();
            var testText = "Hello World 123!";
            var sensitivePattern = @"[A-Z].*\d.*[!@#$%^&*]"; // Pattern that might indicate password
            var logOutput = new StringWriter();

            // Act
            var typingTask = Task.Run(() =>
            {
                try
                {
                    foreach (char c in testText)
                    {
                        if (safetyMonitor.ShouldStop())
                        {
                            break;
                        }

                        // Simulate safe character logging (mask sensitive patterns)
                        var charToLog = IsPotentiallySensitive(c.ToString()) ? "*" : c.ToString();
                        logOutput.Write(charToLog);
                        
                        // Simulate typing with reasonable delays
                        Thread.Sleep(50); // Human-like typing speed
                        safetyMonitor.RecordOperation("type");
                    }
                }
                catch (Exception ex)
                {
                    safetyMonitor.RecordError(ex);
                    throw;
                }
            });

            var completed = await Task.WhenAny(typingTask, Task.Delay(MAX_EXECUTION_TIME_MS));
            
            // Assert
            Assert.AreEqual(typingTask, completed, "Typing operations should complete within time limit");
            
            var loggedOutput = logOutput.ToString();
            Assert.IsFalse(System.Text.RegularExpressions.Regex.IsMatch(loggedOutput, sensitivePattern),
                "Potentially sensitive patterns should be masked in logs");
            
            Assert.IsTrue(loggedOutput.Contains("*"), "Sensitive characters should be masked");
            Assert.IsFalse(safetyMonitor.HasErrors(), "No errors should occur during safe typing");
        }

        [TestMethod]
        [TestCategory("Hardware-Smoke")]
        [Timeout(8000)] // 8 seconds
        public async Task HardwareSmoke_OperationRateLimiting_ShouldEnforceTimingConstraints()
        {
            // Arrange
            var safetyMonitor = new SimpleSafetyMonitor();
            var operationCount = 0;
            var maxOperationsPerSecond = 10;
            var testDurationSeconds = 3;
            var startTime = DateTime.UtcNow;

            // Act
            var rateLimitTask = Task.Run(() =>
            {
                while ((DateTime.UtcNow - startTime).TotalSeconds < testDurationSeconds)
                {
                    if (safetyMonitor.ShouldStop())
                    {
                        break;
                    }

                    var currentRate = safetyMonitor.GetCurrentOperationRate();
                    if (currentRate >= maxOperationsPerSecond)
                    {
                        Thread.Sleep(100); // Throttle operations
                        continue;
                    }

                    // Simulate operation
                    Thread.Sleep(SAFE_OPERATION_DELAY_MS);
                    operationCount++;
                    safetyMonitor.RecordOperation("rate_limited_op");
                }
            });

            await rateLimitTask;
            var actualDuration = DateTime.UtcNow - startTime;

            // Assert
            var actualRate = operationCount / actualDuration.TotalSeconds;
            Assert.IsTrue(actualRate <= maxOperationsPerSecond * 1.1, // 10% tolerance
                $"Operation rate should not exceed {maxOperationsPerSecond} ops/sec, actual: {actualRate:F2}");
            
            Assert.IsFalse(safetyMonitor.HasErrors(), "No errors should occur during rate limiting");
        }

        [TestMethod]
        [TestCategory("Hardware-Smoke")]
        [Timeout(5000)] // 5 seconds
        public void HardwareSmoke_WindowSafetyChecks_ShouldValidateTargets()
        {
            // Arrange & Act
            var safeWindow = FindWindow("Shell_TrayWnd", null); // Taskbar (always safe)
            var invalidWindow = new IntPtr(0x99999999); // Invalid window handle

            // Assert
            Assert.AreNotEqual(IntPtr.Zero, safeWindow, "Should find taskbar window");
            Assert.IsTrue(IsWindow(safeWindow), "Taskbar should be valid window");
            Assert.IsFalse(IsWindow(invalidWindow), "Invalid window should not be valid");
        }

        [TestMethod]
        [TestCategory("Hardware-Smoke")]
        [Timeout(10000)] // 10 seconds
        public async Task HardwareSmoke_EmergencyStop_ShouldRespondQuickly()
        {
            // Arrange
            var safetyMonitor = new SimpleSafetyMonitor();
            var operationCount = 0;
            var stopRequested = false;

            // Act
            var longRunningTask = Task.Run(() =>
            {
                while (!stopRequested && operationCount < 1000)
                {
                    if (safetyMonitor.ShouldStop())
                    {
                        break;
                    }

                    Thread.Sleep(10); // Fast operations
                    operationCount++;
                    safetyMonitor.RecordOperation("emergency_test");
                }
            });

            // Simulate emergency stop after 2 seconds
            await Task.Delay(2000);
            var stopTime = DateTime.UtcNow;
            safetyMonitor.RequestEmergencyStop("Test emergency stop");
            stopRequested = true;

            // Wait for graceful stop
            var completed = await Task.WhenAny(longRunningTask, Task.Delay(1000));
            var stopDuration = DateTime.UtcNow - stopTime;

            // Assert
            Assert.AreEqual(longRunningTask, completed, "Task should stop within 1 second of emergency stop");
            Assert.IsTrue(stopDuration.TotalMilliseconds < 1000, 
                "Emergency stop should be very fast");
            Assert.IsTrue(safetyMonitor.WasEmergencyStopped(), "Emergency stop should be recorded");
        }

        [TestMethod]
        [TestCategory("Hardware-Smoke")]
        [Timeout(6000)] // 6 seconds
        public void HardwareSmoke_FileSystemSafety_ShouldPreventDangerousOperations()
        {
            // Arrange
            var tempDir = Path.GetTempPath();
            var safeTestFile = Path.Combine(tempDir, "luna_safety_test.txt");
            var dangerousPath = @"C:\Windows\System32\kernel32.dll";

            try
            {
                // Act & Assert: Safe operations should succeed
                File.WriteAllText(safeTestFile, "Test content");
                Assert.IsTrue(File.Exists(safeTestFile), "Safe file operations should work");

                var content = File.ReadAllText(safeTestFile);
                Assert.AreEqual("Test content", content, "File content should be preserved");

                // Assert: Dangerous operations should be prevented by OS permissions
                Assert.ThrowsException<UnauthorizedAccessException>(() =>
                {
                    File.WriteAllText(dangerousPath, "Malicious content");
                }, "Writing to system files should be prevented");
            }
            finally
            {
                // Cleanup
                if (File.Exists(safeTestFile))
                {
                    File.Delete(safeTestFile);
                }
            }
        }

        private static (int X, int Y) GetSafeScreenCenter()
        {
            // Get screen dimensions and return center point (safest click location)
            var screenWidth = System.Windows.Forms.Screen.PrimaryScreen.Bounds.Width;
            var screenHeight = System.Windows.Forms.Screen.PrimaryScreen.Bounds.Height;
            return (screenWidth / 2, screenHeight / 2);
        }

        private static bool IsPotentiallySensitive(string text)
        {
            // Basic heuristic for potentially sensitive characters
            return text.Any(c => !char.IsLetterOrDigit(c) && c != ' ') ||
                   text.Any(char.IsUpper) ||
                   text.Any(char.IsDigit);
        }
    }

    public class SimpleSafetyMonitor
    {
        private readonly List<DateTime> _operationTimes = new();
        private readonly List<Exception> _errors = new();
        private volatile bool _emergencyStopRequested = false;
        private volatile bool _stopRequested = false;

        public void RecordOperation(string operationType)
        {
            lock (_operationTimes)
            {
                _operationTimes.Add(DateTime.UtcNow);
                
                // Clean old entries (keep only last 10 seconds)
                var cutoff = DateTime.UtcNow.AddSeconds(-10);
                _operationTimes.RemoveAll(t => t < cutoff);
            }
        }

        public void RecordError(Exception error)
        {
            lock (_errors)
            {
                _errors.Add(error);
            }
        }

        public bool ShouldStop()
        {
            return _stopRequested || _emergencyStopRequested || GetCurrentOperationRate() > 20;
        }

        public void RequestEmergencyStop(string reason)
        {
            _emergencyStopRequested = true;
            _stopRequested = true;
        }

        public double GetCurrentOperationRate()
        {
            lock (_operationTimes)
            {
                var now = DateTime.UtcNow;
                var recentOps = _operationTimes.Count(t => (now - t).TotalSeconds <= 1);
                return recentOps;
            }
        }

        public bool HasErrors()
        {
            lock (_errors)
            {
                return _errors.Count > 0;
            }
        }

        public bool WasEmergencyStopped()
        {
            return _emergencyStopRequested;
        }
    }
}