using System;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using LunaBrokerService;
using System.Text.Json;

namespace LunaBrokerService.Tests
{
    [TestClass]
    public class SecurityValidationTests
    {
        private SecurityValidator _validator;
        private ILogger _logger;

        [TestInitialize]
        public void Setup()
        {
            var loggerFactory = LoggerFactory.Create(builder => builder.AddConsole());
            _logger = loggerFactory.CreateLogger<SecurityValidationTests>();
            _validator = new SecurityValidator(_logger);
        }

        [TestMethod]
        public async Task ValidateRequest_WithValidUIAutomationClick_ShouldSucceed()
        {
            // Arrange
            var request = new BrokerRequest
            {
                RequestId = Guid.NewGuid().ToString(),
                Operation = "uiautomation.click",
                Parameters = JsonSerializer.SerializeToElement(new UIAutomationClickParameters { X = 100, Y = 100, Button = "left" }),
                Timestamp = DateTime.UtcNow
            };

            // Act
            var result = await _validator.ValidateRequestAsync(request, Environment.UserName);

            // Assert
            Assert.IsTrue(result.IsValid, $"Validation should succeed: {result.ErrorMessage}");
        }

        [TestMethod]
        public async Task ValidateRequest_WithInvalidClickCoordinates_ShouldFail()
        {
            // Arrange
            var request = new BrokerRequest
            {
                RequestId = Guid.NewGuid().ToString(),
                Operation = "uiautomation.click",
                Parameters = JsonSerializer.SerializeToElement(new UIAutomationClickParameters { X = -100, Y = -100, Button = "left" }),
                Timestamp = DateTime.UtcNow
            };

            // Act
            var result = await _validator.ValidateRequestAsync(request, Environment.UserName);

            // Assert
            Assert.IsFalse(result.IsValid, "Validation should fail for invalid coordinates");
            Assert.IsTrue(result.ErrorMessage.Contains("Invalid click coordinates"));
        }

        [TestMethod]
        public async Task ValidateRequest_WithDangerousKeySequence_ShouldFail()
        {
            // Arrange
            var request = new BrokerRequest
            {
                RequestId = Guid.NewGuid().ToString(),
                Operation = "uiautomation.sendkeys",
                Parameters = JsonSerializer.SerializeToElement(new UIAutomationSendKeysParameters { Keys = "ctrl+alt+del" }),
                Timestamp = DateTime.UtcNow
            };

            // Act
            var result = await _validator.ValidateRequestAsync(request, Environment.UserName);

            // Assert
            Assert.IsFalse(result.IsValid, "Validation should fail for dangerous key sequences");
            Assert.IsTrue(result.ErrorMessage.Contains("Dangerous key sequence blocked"));
        }

        [TestMethod]
        public async Task ValidateRequest_WithUnauthorizedRegistryPath_ShouldFail()
        {
            // Arrange
            var request = new BrokerRequest
            {
                RequestId = Guid.NewGuid().ToString(),
                Operation = "registry.write",
                Parameters = JsonSerializer.SerializeToElement(new RegistryParameters 
                { 
                    KeyPath = "HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
                    ValueName = "MaliciousApp",
                    Value = "C:\\malware.exe"
                }),
                Timestamp = DateTime.UtcNow
            };

            // Act
            var result = await _validator.ValidateRequestAsync(request, Environment.UserName);

            // Assert
            Assert.IsFalse(result.IsValid, "Validation should fail for unauthorized registry paths");
        }

        [TestMethod]
        public async Task ValidateRequest_WithAuthorizedLunaRegistryPath_ShouldSucceed()
        {
            // Arrange
            var request = new BrokerRequest
            {
                RequestId = Guid.NewGuid().ToString(),
                Operation = "registry.write",
                Parameters = JsonSerializer.SerializeToElement(new RegistryParameters 
                { 
                    KeyPath = "HKEY_CURRENT_USER\\Software\\Luna\\Settings",
                    ValueName = "TestSetting",
                    Value = "TestValue"
                }),
                Timestamp = DateTime.UtcNow
            };

            // Act
            var result = await _validator.ValidateRequestAsync(request, Environment.UserName);

            // Assert
            Assert.IsTrue(result.IsValid, $"Validation should succeed for Luna registry paths: {result.ErrorMessage}");
        }

        [TestMethod]
        public async Task ValidateRequest_WithUnauthorizedExecutable_ShouldFail()
        {
            // Arrange
            var request = new BrokerRequest
            {
                RequestId = Guid.NewGuid().ToString(),
                Operation = "process.start",
                Parameters = JsonSerializer.SerializeToElement(new ProcessParameters 
                { 
                    FileName = "cmd.exe",
                    Arguments = "/c format c: /y"
                }),
                Timestamp = DateTime.UtcNow
            };

            // Act
            var result = await _validator.ValidateRequestAsync(request, Environment.UserName);

            // Assert
            Assert.IsFalse(result.IsValid, "Validation should fail for unauthorized executables");
        }

        [TestMethod]
        public async Task ValidateRequest_WithAuthorizedExecutable_ShouldSucceed()
        {
            // Arrange
            var request = new BrokerRequest
            {
                RequestId = Guid.NewGuid().ToString(),
                Operation = "process.start",
                Parameters = JsonSerializer.SerializeToElement(new ProcessParameters 
                { 
                    FileName = "notepad.exe"
                }),
                Timestamp = DateTime.UtcNow
            };

            // Act
            var result = await _validator.ValidateRequestAsync(request, Environment.UserName);

            // Assert
            Assert.IsTrue(result.IsValid, $"Validation should succeed for authorized executables: {result.ErrorMessage}");
        }

        [TestMethod]
        public async Task ValidateRequest_WithUnauthorizedFilePath_ShouldFail()
        {
            // Arrange
            var request = new BrokerRequest
            {
                RequestId = Guid.NewGuid().ToString(),
                Operation = "file.write",
                Parameters = JsonSerializer.SerializeToElement(new FileParameters 
                { 
                    FilePath = "C:\\Windows\\System32\\drivers\\etc\\hosts",
                    Content = "127.0.0.1 malicious.com"
                }),
                Timestamp = DateTime.UtcNow
            };

            // Act
            var result = await _validator.ValidateRequestAsync(request, Environment.UserName);

            // Assert
            Assert.IsFalse(result.IsValid, "Validation should fail for unauthorized file paths");
        }

        [TestMethod]
        public async Task ValidateRequest_WithEmptyOperation_ShouldFail()
        {
            // Arrange
            var request = new BrokerRequest
            {
                RequestId = Guid.NewGuid().ToString(),
                Operation = "",
                Parameters = null,
                Timestamp = DateTime.UtcNow
            };

            // Act
            var result = await _validator.ValidateRequestAsync(request, Environment.UserName);

            // Assert
            Assert.IsFalse(result.IsValid, "Validation should fail for empty operation");
            Assert.IsTrue(result.ErrorMessage.Contains("Operation cannot be empty"));
        }

        [TestMethod]
        public async Task ValidateRequest_WithEmptyRequestId_ShouldFail()
        {
            // Arrange
            var request = new BrokerRequest
            {
                RequestId = "",
                Operation = "uiautomation.getwindows",
                Parameters = null,
                Timestamp = DateTime.UtcNow
            };

            // Act
            var result = await _validator.ValidateRequestAsync(request, Environment.UserName);

            // Assert
            Assert.IsFalse(result.IsValid, "Validation should fail for empty request ID");
            Assert.IsTrue(result.ErrorMessage.Contains("RequestId cannot be empty"));
        }

        [TestMethod]
        public void IsDangerousKeySequence_WithDangerousSequence_ShouldReturnTrue()
        {
            // Act & Assert
            Assert.IsTrue(_validator.IsDangerousKeySequence("ctrl+alt+del"));
            Assert.IsTrue(_validator.IsDangerousKeySequence("CTRL+ALT+DEL"));
            Assert.IsTrue(_validator.IsDangerousKeySequence("win+r"));
            Assert.IsTrue(_validator.IsDangerousKeySequence("shift+del"));
        }

        [TestMethod]
        public void IsDangerousKeySequence_WithSafeSequence_ShouldReturnFalse()
        {
            // Act & Assert
            Assert.IsFalse(_validator.IsDangerousKeySequence("ctrl+c"));
            Assert.IsFalse(_validator.IsDangerousKeySequence("ctrl+v"));
            Assert.IsFalse(_validator.IsDangerousKeySequence("alt+tab"));
            Assert.IsFalse(_validator.IsDangerousKeySequence("Hello World"));
        }

        [TestMethod]
        public void IsAllowedRegistryPath_WithLunaPath_ShouldReturnTrue()
        {
            // Act & Assert
            Assert.IsTrue(_validator.IsAllowedRegistryPath("HKEY_CURRENT_USER\\Software\\Luna\\Settings", false));
            Assert.IsTrue(_validator.IsAllowedRegistryPath("HKEY_LOCAL_MACHINE\\Software\\Luna\\Config", false));
        }

        [TestMethod]
        public void IsAllowedRegistryPath_WithSystemPath_ShouldReturnFalse()
        {
            // Act & Assert
            Assert.IsFalse(_validator.IsAllowedRegistryPath("HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion", true));
            Assert.IsFalse(_validator.IsAllowedRegistryPath("HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet", true));
        }

        [TestMethod]
        public void IsAllowedFilePath_WithLunaPath_ShouldReturnTrue()
        {
            // Act & Assert
            Assert.IsTrue(_validator.IsAllowedFilePath("C:\\ProgramData\\Luna\\config.json", false));
            Assert.IsTrue(_validator.IsAllowedFilePath("C:\\Users\\testuser\\AppData\\Local\\Luna\\data.txt", false));
        }

        [TestMethod]
        public void IsAllowedFilePath_WithSystemPath_ShouldReturnFalse()
        {
            // Act & Assert
            Assert.IsFalse(_validator.IsAllowedFilePath("C:\\Windows\\System32\\kernel32.dll", true));
            Assert.IsFalse(_validator.IsAllowedFilePath("C:\\Windows\\explorer.exe", true));
        }

        [TestMethod]
        public void IsAllowedExecutable_WithAuthorizedApp_ShouldReturnTrue()
        {
            // Act & Assert
            Assert.IsTrue(_validator.IsAllowedExecutable("notepad.exe"));
            Assert.IsTrue(_validator.IsAllowedExecutable("calc.exe"));
            Assert.IsTrue(_validator.IsAllowedExecutable("C:\\Windows\\System32\\notepad.exe"));
        }

        [TestMethod]
        public void IsAllowedExecutable_WithUnauthorizedApp_ShouldReturnFalse()
        {
            // Act & Assert
            Assert.IsFalse(_validator.IsAllowedExecutable("malware.exe"));
            Assert.IsFalse(_validator.IsAllowedExecutable("cmd.exe"));
            Assert.IsFalse(_validator.IsAllowedExecutable("powershell.exe"));
        }

        [TestMethod]
        public void CanTerminateProcess_WithSystemProcess_ShouldReturnFalse()
        {
            // Note: This test assumes system processes exist
            // In a real environment, you'd need to identify actual system process IDs
            
            // Act & Assert
            // System process (usually PID 4)
            Assert.IsFalse(_validator.CanTerminateProcess(4));
        }

        [TestMethod]
        public void CanTerminateProcess_WithNonexistentProcess_ShouldReturnFalse()
        {
            // Act & Assert
            Assert.IsFalse(_validator.CanTerminateProcess(999999));
        }
    }
}