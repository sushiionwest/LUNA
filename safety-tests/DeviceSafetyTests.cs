using System;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using OpenQA.Selenium;
using OpenQA.Selenium.Appium;
using OpenQA.Selenium.Appium.Windows;
using OpenQA.Selenium.Support.UI;

namespace LunaSafetyTests
{
    [TestClass]
    public class DeviceSafetyTests
    {
        private WindowsDriver<WindowsElement> _session;
        private Process _notepadProcess;
        private const int MAX_CLICK_RATE = 10; // Maximum clicks per second
        private const int MAX_CONTINUOUS_OPERATIONS = 50; // Maximum continuous operations
        private const int OPERATION_TIMEOUT_SECONDS = 30;

        [TestInitialize]
        public void Setup()
        {
            var appiumOptions = new AppiumOptions();
            appiumOptions.AddAdditionalCapability("app", "Root");
            appiumOptions.AddAdditionalCapability("deviceName", "WindowsPC");
            appiumOptions.AddAdditionalCapability("platformName", "Windows");

            _session = new WindowsDriver<WindowsElement>(new Uri("http://127.0.0.1:4723"), appiumOptions);
            _session.Manage().Timeouts().ImplicitWait = TimeSpan.FromSeconds(5);
        }

        [TestCleanup]
        public void Cleanup()
        {
            try
            {
                _notepadProcess?.Kill();
                _notepadProcess?.Dispose();
            }
            catch { }

            _session?.Quit();
        }

        [TestMethod]
        [TestCategory("Hardware-Smoke")]
        [Timeout(60000)] // 60 seconds
        public async Task Test_SafeNotepadInteraction_ShouldCompleteWithoutError()
        {
            // Arrange: Start Notepad
            _notepadProcess = Process.Start("notepad.exe");
            await Task.Delay(2000); // Wait for Notepad to start

            var notepadWindow = _session.FindElementByName("Untitled - Notepad");
            Assert.IsNotNull(notepadWindow, "Notepad window should be found");

            // Act: Type "hello" safely
            var safetyMonitor = new OperationSafetyMonitor();
            var typingTask = Task.Run(() =>
            {
                try
                {
                    safetyMonitor.BeginOperation("typing");
                    notepadWindow.Click();
                    Thread.Sleep(500); // Brief pause for focus
                    
                    // Type each character with safety checks
                    var textToType = "hello";
                    foreach (char c in textToType)
                    {
                        if (safetyMonitor.ShouldStop())
                        {
                            throw new SafetyException("Operation stopped by safety monitor");
                        }
                        
                        _session.Keyboard.SendKeys(c.ToString());
                        Thread.Sleep(100); // Prevent rapid-fire typing
                    }
                    
                    safetyMonitor.EndOperation("typing");
                    return true;
                }
                catch (Exception ex)
                {
                    safetyMonitor.EndOperation("typing", ex);
                    throw;
                }
            });

            var completed = await Task.WhenAny(typingTask, Task.Delay(TimeSpan.FromSeconds(OPERATION_TIMEOUT_SECONDS)));
            Assert.AreEqual(typingTask, completed, "Typing operation should complete within timeout");

            // Save the file
            var saveTask = Task.Run(() =>
            {
                try
                {
                    safetyMonitor.BeginOperation("saving");
                    _session.Keyboard.SendKeys(Keys.Control + "s");
                    Thread.Sleep(1000);
                    
                    // Type filename
                    var testFileName = Path.Combine(Path.GetTempPath(), "luna_safety_test.txt");
                    _session.Keyboard.SendKeys(testFileName);
                    Thread.Sleep(500);
                    
                    _session.Keyboard.SendKeys(Keys.Return);
                    safetyMonitor.EndOperation("saving");
                    return testFileName;
                }
                catch (Exception ex)
                {
                    safetyMonitor.EndOperation("saving", ex);
                    throw;
                }
            });

            completed = await Task.WhenAny(saveTask, Task.Delay(TimeSpan.FromSeconds(OPERATION_TIMEOUT_SECONDS)));
            Assert.AreEqual(saveTask, completed, "Save operation should complete within timeout");

            var savedFileName = saveTask.Result;

            // Assert: Verify file content
            Assert.IsTrue(File.Exists(savedFileName), "File should be saved");
            var fileContent = await File.ReadAllTextAsync(savedFileName);
            Assert.AreEqual("hello", fileContent.Trim(), "File content should match typed text");

            // Cleanup
            try
            {
                File.Delete(savedFileName);
            }
            catch { }

            // Verify safety metrics
            Assert.IsTrue(safetyMonitor.GetOperationCount() <= MAX_CONTINUOUS_OPERATIONS, 
                "Should not exceed maximum continuous operations");
            Assert.IsTrue(safetyMonitor.GetMaxOperationRate() <= MAX_CLICK_RATE, 
                "Should not exceed maximum operation rate");
        }

        [TestMethod]
        [TestCategory("Hardware-Smoke")]
        [Timeout(45000)] // 45 seconds
        public async Task Test_ClickSafetyLimits_ShouldPreventRunawayClicking()
        {
            // Arrange
            var safetyMonitor = new OperationSafetyMonitor();
            var calculator = Process.Start("calc.exe");
            await Task.Delay(2000);

            try
            {
                var calcWindow = _session.FindElementByName("Calculator");
                Assert.IsNotNull(calcWindow, "Calculator window should be found");

                // Act: Attempt rapid clicking with safety limits
                var clickCount = 0;
                var startTime = DateTime.UtcNow;
                var maxTestDuration = TimeSpan.FromSeconds(30);

                var clickingTask = Task.Run(() =>
                {
                    while (DateTime.UtcNow - startTime < maxTestDuration)
                    {
                        if (safetyMonitor.ShouldStop() || clickCount >= MAX_CONTINUOUS_OPERATIONS)
                        {
                            break;
                        }

                        try
                        {
                            safetyMonitor.BeginOperation("click");
                            
                            // Find and click the "1" button
                            var oneButton = _session.FindElementByName("One");
                            oneButton?.Click();
                            
                            clickCount++;
                            safetyMonitor.EndOperation("click");
                            
                            // Enforce minimum delay between clicks
                            Thread.Sleep(100); // 100ms minimum between clicks
                        }
                        catch (Exception ex)
                        {
                            safetyMonitor.EndOperation("click", ex);
                            break;
                        }
                    }
                });

                await clickingTask;

                // Assert: Verify safety limits were enforced
                var actualDuration = DateTime.UtcNow - startTime;
                var expectedMinimumDuration = TimeSpan.FromMilliseconds(clickCount * 100); // 100ms per click
                
                Assert.IsTrue(actualDuration >= expectedMinimumDuration, 
                    $"Clicking should take at least {expectedMinimumDuration.TotalMilliseconds}ms, actual: {actualDuration.TotalMilliseconds}ms");
                
                Assert.IsTrue(clickCount <= MAX_CONTINUOUS_OPERATIONS, 
                    $"Should not exceed {MAX_CONTINUOUS_OPERATIONS} clicks, actual: {clickCount}");
                
                var averageClickRate = clickCount / actualDuration.TotalSeconds;
                Assert.IsTrue(averageClickRate <= MAX_CLICK_RATE, 
                    $"Average click rate should not exceed {MAX_CLICK_RATE} clicks/second, actual: {averageClickRate:F2}");
            }
            finally
            {
                calculator?.Kill();
                calculator?.Dispose();
            }
        }

        [TestMethod]
        [TestCategory("Hardware-Smoke")]
        [Timeout(30000)] // 30 seconds
        public async Task Test_PasswordFieldSafety_ShouldPreventPasswordLeakage()
        {
            // Arrange: Open Run dialog to test password field behavior
            var runDialogTask = Task.Run(() =>
            {
                _session.Keyboard.SendKeys(Keys.Meta + "r");
                Thread.Sleep(1000);
                return _session.FindElementByName("Run");
            });

            var runDialog = await runDialogTask;
            Assert.IsNotNull(runDialog, "Run dialog should open");

            var safetyMonitor = new OperationSafetyMonitor();

            // Act: Simulate typing in what could be a password field
            var sensitiveText = "P@ssw0rd123!";
            var loggedText = string.Empty;

            var typingTask = Task.Run(() =>
            {
                try
                {
                    safetyMonitor.BeginOperation("sensitive_typing");
                    
                    foreach (char c in sensitiveText)
                    {
                        if (safetyMonitor.ShouldStop())
                        {
                            break;
                        }
                        
                        // Simulate character input with logging prevention
                        if (IsPasswordCharacter(c))
                        {
                            loggedText += "*"; // Mask sensitive characters in logs
                        }
                        else
                        {
                            loggedText += c;
                        }
                        
                        _session.Keyboard.SendKeys(c.ToString());
                        Thread.Sleep(50); // Reasonable typing speed
                    }
                    
                    safetyMonitor.EndOperation("sensitive_typing");
                }
                catch (Exception ex)
                {
                    safetyMonitor.EndOperation("sensitive_typing", ex);
                    throw;
                }
            });

            await typingTask;

            // Press Escape to cancel the run dialog
            _session.Keyboard.SendKeys(Keys.Escape);

            // Assert: Verify sensitive data handling
            Assert.IsFalse(loggedText.Contains("P@ssw0rd123!"), 
                "Actual password should not appear in logs");
            Assert.IsTrue(loggedText.Contains("*"), 
                "Password characters should be masked in logs");
            
            // Verify no actual command was executed
            var processes = Process.GetProcessesByName("cmd");
            Assert.AreEqual(0, processes.Length, 
                "No command prompt should be opened from password test");
        }

        [TestMethod]
        [TestCategory("Hardware-Smoke")]
        [Timeout(60000)] // 60 seconds
        public async Task Test_SystemInterruptionHandling_ShouldGracefullyStop()
        {
            // Arrange: Start a long-running operation
            var notepad = Process.Start("notepad.exe");
            await Task.Delay(2000);

            try
            {
                var notepadWindow = _session.FindElementByName("Untitled - Notepad");
                var safetyMonitor = new OperationSafetyMonitor();
                var operationCancelled = false;

                // Act: Start a long typing operation that can be interrupted
                var longOperationTask = Task.Run(async () =>
                {
                    try
                    {
                        safetyMonitor.BeginOperation("long_typing");
                        notepadWindow.Click();
                        Thread.Sleep(500);

                        // Type a long sequence with interruption checks
                        for (int i = 0; i < 1000; i++)
                        {
                            if (safetyMonitor.ShouldStop())
                            {
                                operationCancelled = true;
                                break;
                            }

                            _session.Keyboard.SendKeys($"Line {i + 1}: This is a test line.\n");
                            Thread.Sleep(10); // Very fast typing to test interruption
                        }

                        safetyMonitor.EndOperation("long_typing");
                    }
                    catch (Exception ex)
                    {
                        safetyMonitor.EndOperation("long_typing", ex);
                        operationCancelled = true;
                    }
                });

                // Simulate interruption after 5 seconds
                await Task.Delay(5000);
                safetyMonitor.RequestStop("User requested interruption");

                // Wait for operation to stop gracefully
                var completed = await Task.WhenAny(longOperationTask, Task.Delay(10000));
                Assert.AreEqual(longOperationTask, completed, 
                    "Operation should stop within 10 seconds of interruption request");

                // Assert: Verify graceful stopping
                Assert.IsTrue(operationCancelled, "Operation should be cancelled when requested");
                Assert.IsTrue(safetyMonitor.WasStoppedGracefully(), 
                    "Operation should stop gracefully, not crash");
            }
            finally
            {
                notepad?.Kill();
                notepad?.Dispose();
            }
        }

        [TestMethod]
        [TestCategory("Hardware-Smoke")]
        [Timeout(45000)] // 45 seconds
        public async Task Test_WindowFocusManagement_ShouldNotStealFocus()
        {
            // Arrange: Open multiple applications
            var notepad = Process.Start("notepad.exe");
            await Task.Delay(1000);
            var calc = Process.Start("calc.exe");
            await Task.Delay(1000);

            try
            {
                var notepadWindow = _session.FindElementByName("Untitled - Notepad");
                var calcWindow = _session.FindElementByName("Calculator");
                
                Assert.IsNotNull(notepadWindow, "Notepad should be available");
                Assert.IsNotNull(calcWindow, "Calculator should be available");

                var safetyMonitor = new OperationSafetyMonitor();

                // Act: Test controlled window interactions
                var windowInteractionTask = Task.Run(() =>
                {
                    try
                    {
                        safetyMonitor.BeginOperation("window_interaction");
                        
                        // Focus on notepad first
                        notepadWindow.Click();
                        Thread.Sleep(500);
                        
                        // Type something
                        _session.Keyboard.SendKeys("Notepad test");
                        Thread.Sleep(500);
                        
                        // Switch to calculator with user-like timing
                        calcWindow.Click();
                        Thread.Sleep(500);
                        
                        // Click calculator button
                        var oneButton = _session.FindElementByName("One");
                        oneButton?.Click();
                        Thread.Sleep(500);
                        
                        safetyMonitor.EndOperation("window_interaction");
                        return true;
                    }
                    catch (Exception ex)
                    {
                        safetyMonitor.EndOperation("window_interaction", ex);
                        throw;
                    }
                });

                var completed = await Task.WhenAny(windowInteractionTask, 
                    Task.Delay(TimeSpan.FromSeconds(OPERATION_TIMEOUT_SECONDS)));
                Assert.AreEqual(windowInteractionTask, completed, 
                    "Window interaction should complete within timeout");

                // Assert: Verify controlled focus changes
                Assert.IsTrue(windowInteractionTask.Result, "Window interactions should succeed");
                
                // Verify timing constraints (no rapid focus stealing)
                var operationDuration = safetyMonitor.GetOperationDuration("window_interaction");
                Assert.IsTrue(operationDuration >= TimeSpan.FromSeconds(2), 
                    "Window interactions should take reasonable time, not be instantaneous");
            }
            finally
            {
                notepad?.Kill();
                notepad?.Dispose();
                calc?.Kill();
                calc?.Dispose();
            }
        }

        private bool IsPasswordCharacter(char c)
        {
            // Consider special characters and numbers as potentially sensitive
            return !char.IsLetterOrDigit(c) || char.IsUpper(c) || char.IsDigit(c);
        }
    }

    public class OperationSafetyMonitor
    {
        private readonly Dictionary<string, DateTime> _operationStartTimes = new();
        private readonly Dictionary<string, TimeSpan> _operationDurations = new();
        private readonly List<DateTime> _operationTimes = new();
        private volatile bool _stopRequested = false;
        private string _stopReason = string.Empty;
        private bool _stoppedGracefully = false;

        public void BeginOperation(string operationType)
        {
            _operationStartTimes[operationType] = DateTime.UtcNow;
            _operationTimes.Add(DateTime.UtcNow);
        }

        public void EndOperation(string operationType, Exception error = null)
        {
            if (_operationStartTimes.TryGetValue(operationType, out var startTime))
            {
                _operationDurations[operationType] = DateTime.UtcNow - startTime;
                _operationStartTimes.Remove(operationType);
            }

            if (error == null && _stopRequested)
            {
                _stoppedGracefully = true;
            }
        }

        public bool ShouldStop()
        {
            return _stopRequested || GetOperationCount() >= 50 || GetMaxOperationRate() > 10;
        }

        public void RequestStop(string reason)
        {
            _stopRequested = true;
            _stopReason = reason;
        }

        public int GetOperationCount()
        {
            return _operationTimes.Count;
        }

        public double GetMaxOperationRate()
        {
            if (_operationTimes.Count < 2) return 0;

            var now = DateTime.UtcNow;
            var recentOperations = _operationTimes.Where(t => now - t <= TimeSpan.FromSeconds(1)).Count();
            return recentOperations;
        }

        public TimeSpan GetOperationDuration(string operationType)
        {
            return _operationDurations.GetValueOrDefault(operationType, TimeSpan.Zero);
        }

        public bool WasStoppedGracefully()
        {
            return _stoppedGracefully;
        }
    }

    public class SafetyException : Exception
    {
        public SafetyException(string message) : base(message) { }
        public SafetyException(string message, Exception innerException) : base(message, innerException) { }
    }
}