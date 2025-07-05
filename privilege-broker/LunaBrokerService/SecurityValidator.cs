using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Principal;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using System.Diagnostics;

namespace LunaBrokerService
{
    public class SecurityValidator
    {
        private readonly ILogger _logger;
        private readonly HashSet<string> _allowedExecutables;
        private readonly HashSet<string> _dangerousKeySequences;
        private readonly List<Regex> _allowedRegistryPaths;
        private readonly List<Regex> _allowedWritableRegistryPaths;
        private readonly List<Regex> _allowedFilePaths;
        private readonly List<Regex> _allowedWritableFilePaths;
        private readonly HashSet<string> _trustedCallerHashes;

        public SecurityValidator(ILogger logger)
        {
            _logger = logger ?? throw new ArgumentNullException(nameof(logger));
            
            // Initialize security policies
            InitializeAllowedExecutables();
            InitializeDangerousKeySequences();
            InitializeRegistryPaths();
            InitializeFilePaths();
            InitializeTrustedCallers();
        }

        private void InitializeAllowedExecutables()
        {
            _allowedExecutables = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
            {
                // Windows system utilities
                "notepad.exe",
                "calc.exe",
                "mspaint.exe",
                "explorer.exe",
                
                // Common applications (can be expanded)
                "chrome.exe",
                "firefox.exe",
                "code.exe", // VS Code
                
                // System tools (be very careful with these)
                "taskmgr.exe",
                "regedit.exe" // Only if registry operations are specifically allowed
            };
        }

        private void InitializeDangerousKeySequences()
        {
            _dangerousKeySequences = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
            {
                // System shortcuts that could be dangerous
                "ctrl+alt+del",
                "ctrl+shift+esc",
                "win+l", // Lock screen
                "alt+f4", // Close window
                "win+r", // Run dialog
                
                // Registry/system modification shortcuts
                "win+x",
                
                // Browser dangerous shortcuts
                "ctrl+shift+del", // Clear browsing data
                "ctrl+shift+n", // Incognito/private browsing
                
                // File system shortcuts
                "shift+del", // Permanent delete
                
                // Sequences that include passwords or sensitive data patterns
                // (This would be expanded with more sophisticated detection)
            };
        }

        private void InitializeRegistryPaths()
        {
            // Allowed registry paths for reading
            _allowedRegistryPaths = new List<Regex>
            {
                new Regex(@"^HKEY_CURRENT_USER\\Software\\Luna\\.*", RegexOptions.IgnoreCase),
                new Regex(@"^HKEY_LOCAL_MACHINE\\Software\\Luna\\.*", RegexOptions.IgnoreCase),
                new Regex(@"^HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run$", RegexOptions.IgnoreCase),
                new Regex(@"^HKEY_LOCAL_MACHINE\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\.*", RegexOptions.IgnoreCase),
                
                // System information (read-only)
                new Regex(@"^HKEY_LOCAL_MACHINE\\Hardware\\.*", RegexOptions.IgnoreCase),
                new Regex(@"^HKEY_LOCAL_MACHINE\\System\\CurrentControlSet\\Control\\ComputerName\\.*", RegexOptions.IgnoreCase)
            };

            // Allowed registry paths for writing (much more restrictive)
            _allowedWritableRegistryPaths = new List<Regex>
            {
                new Regex(@"^HKEY_CURRENT_USER\\Software\\Luna\\.*", RegexOptions.IgnoreCase),
                new Regex(@"^HKEY_LOCAL_MACHINE\\Software\\Luna\\.*", RegexOptions.IgnoreCase),
                
                // Autostart entries (only for Luna itself)
                new Regex(@"^HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run\\Luna.*", RegexOptions.IgnoreCase)
            };
        }

        private void InitializeFilePaths()
        {
            // Allowed file paths for reading
            _allowedFilePaths = new List<Regex>
            {
                // Luna application directory
                new Regex(@"^C:\\Program Files\\Luna Agent\\.*", RegexOptions.IgnoreCase),
                new Regex(@"^C:\\ProgramData\\Luna\\.*", RegexOptions.IgnoreCase),
                
                // User's Luna directory
                new Regex(@"^C:\\Users\\[^\\]+\\AppData\\(?:Local|Roaming)\\Luna\\.*", RegexOptions.IgnoreCase),
                
                // Temporary files
                new Regex(@"^C:\\Windows\\Temp\\Luna_.*", RegexOptions.IgnoreCase),
                new Regex(@"^C:\\Users\\[^\\]+\\AppData\\Local\\Temp\\Luna_.*", RegexOptions.IgnoreCase),
                
                // System information files (read-only)
                new Regex(@"^C:\\Windows\\System32\\drivers\\etc\\hosts$", RegexOptions.IgnoreCase),
                
                // Desktop files (for automation)
                new Regex(@"^C:\\Users\\[^\\]+\\Desktop\\.*", RegexOptions.IgnoreCase),
                new Regex(@"^C:\\Users\\[^\\]+\\Documents\\.*", RegexOptions.IgnoreCase)
            };

            // Allowed file paths for writing (more restrictive)
            _allowedWritableFilePaths = new List<Regex>
            {
                // Luna application data only
                new Regex(@"^C:\\ProgramData\\Luna\\.*", RegexOptions.IgnoreCase),
                new Regex(@"^C:\\Users\\[^\\]+\\AppData\\(?:Local|Roaming)\\Luna\\.*", RegexOptions.IgnoreCase),
                
                // Temporary files
                new Regex(@"^C:\\Windows\\Temp\\Luna_.*", RegexOptions.IgnoreCase),
                new Regex(@"^C:\\Users\\[^\\]+\\AppData\\Local\\Temp\\Luna_.*", RegexOptions.IgnoreCase),
                
                // User documents (with specific patterns to avoid system files)
                new Regex(@"^C:\\Users\\[^\\]+\\Documents\\Luna\\.*", RegexOptions.IgnoreCase),
                new Regex(@"^C:\\Users\\[^\\]+\\Desktop\\Luna_.*", RegexOptions.IgnoreCase)
            };
        }

        private void InitializeTrustedCallers()
        {
            // This would be populated with hashes of trusted caller executables
            // In a real implementation, you'd verify the digital signature and hash
            // of the calling process
            _trustedCallerHashes = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
            {
                // Placeholder - would be actual SHA256 hashes of signed Luna executables
                "placeholder_hash_of_luna_controller"
            };
        }

        public async Task<ValidationResult> ValidateRequestAsync(BrokerRequest request, string clientIdentity)
        {
            try
            {
                _logger.LogInformation("Validating request from client: {ClientIdentity}, Operation: {Operation}", 
                    clientIdentity, request.Operation);

                // Basic request validation
                if (string.IsNullOrEmpty(request.Operation))
                {
                    return ValidationResult.Failure("Operation cannot be empty");
                }

                if (string.IsNullOrEmpty(request.RequestId))
                {
                    return ValidationResult.Failure("RequestId cannot be empty");
                }

                // Validate client identity
                if (!IsValidClientIdentity(clientIdentity))
                {
                    return ValidationResult.Failure("Client identity not authorized");
                }

                // Validate the calling process
                if (!await ValidateCallingProcessAsync())
                {
                    return ValidationResult.Failure("Calling process not trusted");
                }

                // Rate limiting check
                if (!CheckRateLimit(clientIdentity, request.Operation))
                {
                    return ValidationResult.Failure("Rate limit exceeded");
                }

                // Operation-specific validation
                var operationValidation = ValidateOperation(request);
                if (!operationValidation.IsValid)
                {
                    return operationValidation;
                }

                _logger.LogInformation("Request validation successful for operation: {Operation}", request.Operation);
                return ValidationResult.Success();
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error during request validation");
                return ValidationResult.Failure("Validation error occurred");
            }
        }

        private bool IsValidClientIdentity(string clientIdentity)
        {
            try
            {
                // Check if the client is running under the current user or an authorized service account
                var currentUser = WindowsIdentity.GetCurrent();
                
                // Allow current user and service accounts
                if (clientIdentity.Contains(currentUser.Name, StringComparison.OrdinalIgnoreCase) ||
                    clientIdentity.Contains("NT AUTHORITY\\SYSTEM", StringComparison.OrdinalIgnoreCase) ||
                    clientIdentity.Contains("NT AUTHORITY\\LOCAL SERVICE", StringComparison.OrdinalIgnoreCase) ||
                    clientIdentity.Contains("NT AUTHORITY\\NETWORK SERVICE", StringComparison.OrdinalIgnoreCase))
                {
                    return true;
                }

                // Check if user is in Administrators group
                using var identity = new WindowsIdentity(clientIdentity);
                var principal = new WindowsPrincipal(identity);
                return principal.IsInRole(WindowsBuiltInRole.Administrator);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error validating client identity: {ClientIdentity}", clientIdentity);
                return false;
            }
        }

        private async Task<bool> ValidateCallingProcessAsync()
        {
            try
            {
                // Get the calling process
                var currentProcess = Process.GetCurrentProcess();
                var parentProcess = GetParentProcess(currentProcess);
                
                if (parentProcess == null)
                {
                    _logger.LogWarning("Could not determine parent process");
                    return false;
                }

                // Verify the parent process is a trusted Luna component
                var processPath = parentProcess.MainModule?.FileName;
                if (string.IsNullOrEmpty(processPath))
                {
                    _logger.LogWarning("Could not determine parent process path");
                    return false;
                }

                // Check if the process is in an allowed location
                if (!processPath.StartsWith(@"C:\Program Files\Luna Agent\", StringComparison.OrdinalIgnoreCase) &&
                    !processPath.StartsWith(@"C:\ProgramData\Luna\", StringComparison.OrdinalIgnoreCase))
                {
                    _logger.LogWarning("Parent process not in trusted location: {ProcessPath}", processPath);
                    return false;
                }

                // TODO: Verify digital signature of the calling process
                // This would check that the executable is properly signed with our certificate

                return true;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error validating calling process");
                return false;
            }
        }

        private Process GetParentProcess(Process process)
        {
            try
            {
                var parentPid = 0;
                var processId = process.Id;
                var o = wmiQueryString.Replace("$PID", processId.ToString());
                var mos = new ManagementObjectSearcher(o);
                var moc = mos.Get();
                foreach (var mo in moc)
                {
                    parentPid = Convert.ToInt32(mo["ParentProcessId"]);
                }
                return Process.GetProcessById(parentPid);
            }
            catch
            {
                return null;
            }
        }

        private static readonly string wmiQueryString = "SELECT ParentProcessId FROM Win32_Process WHERE ProcessId = $PID";

        private readonly Dictionary<string, DateTime> _lastRequestTimes = new();
        private readonly Dictionary<string, int> _requestCounts = new();
        private const int MaxRequestsPerMinute = 100;

        private bool CheckRateLimit(string clientIdentity, string operation)
        {
            var key = $"{clientIdentity}:{operation}";
            var now = DateTime.UtcNow;
            
            lock (_lastRequestTimes)
            {
                if (_lastRequestTimes.TryGetValue(key, out var lastTime))
                {
                    if (now - lastTime < TimeSpan.FromMinutes(1))
                    {
                        _requestCounts[key] = _requestCounts.GetValueOrDefault(key, 0) + 1;
                        if (_requestCounts[key] > MaxRequestsPerMinute)
                        {
                            _logger.LogWarning("Rate limit exceeded for {Key}", key);
                            return false;
                        }
                    }
                    else
                    {
                        _requestCounts[key] = 1;
                    }
                }
                else
                {
                    _requestCounts[key] = 1;
                }
                
                _lastRequestTimes[key] = now;
            }
            
            return true;
        }

        private ValidationResult ValidateOperation(BrokerRequest request)
        {
            return request.Operation?.ToLowerInvariant() switch
            {
                "uiautomation.click" => ValidateUIAutomationClick(request),
                "uiautomation.sendkeys" => ValidateUIAutomationSendKeys(request),
                "uiautomation.getwindows" => ValidationResult.Success(), // Safe operation
                "registry.read" => ValidateRegistryOperation(request, false),
                "registry.write" => ValidateRegistryOperation(request, true),
                "process.start" => ValidateProcessStart(request),
                "process.terminate" => ValidateProcessTerminate(request),
                "file.read" => ValidateFileOperation(request, false),
                "file.write" => ValidateFileOperation(request, true),
                "system.screenshot" => ValidationResult.Success(), // Safe operation
                _ => ValidationResult.Failure($"Unknown operation: {request.Operation}")
            };
        }

        private ValidationResult ValidateUIAutomationClick(BrokerRequest request)
        {
            try
            {
                var parameters = System.Text.Json.JsonSerializer.Deserialize<UIAutomationClickParameters>(
                    request.Parameters?.ToString() ?? "{}");
                
                // Validate coordinates are reasonable
                if (parameters.X < 0 || parameters.Y < 0 || parameters.X > 65535 || parameters.Y > 65535)
                {
                    return ValidationResult.Failure("Invalid click coordinates");
                }

                return ValidationResult.Success();
            }
            catch (Exception ex)
            {
                return ValidationResult.Failure($"Invalid click parameters: {ex.Message}");
            }
        }

        private ValidationResult ValidateUIAutomationSendKeys(BrokerRequest request)
        {
            try
            {
                var parameters = System.Text.Json.JsonSerializer.Deserialize<UIAutomationSendKeysParameters>(
                    request.Parameters?.ToString() ?? "{}");
                
                if (IsDangerousKeySequence(parameters.Keys))
                {
                    return ValidationResult.Failure("Dangerous key sequence blocked");
                }

                return ValidationResult.Success();
            }
            catch (Exception ex)
            {
                return ValidationResult.Failure($"Invalid sendkeys parameters: {ex.Message}");
            }
        }

        public bool IsDangerousKeySequence(string keys)
        {
            if (string.IsNullOrEmpty(keys))
                return false;

            var normalizedKeys = keys.ToLowerInvariant().Replace(" ", "");
            
            return _dangerousKeySequences.Any(dangerous => 
                normalizedKeys.Contains(dangerous.Replace(" ", "")));
        }

        public bool IsAllowedRegistryPath(string keyPath, bool isWrite)
        {
            if (string.IsNullOrEmpty(keyPath))
                return false;

            var allowedPaths = isWrite ? _allowedWritableRegistryPaths : _allowedRegistryPaths;
            return allowedPaths.Any(regex => regex.IsMatch(keyPath));
        }

        public bool IsAllowedFilePath(string filePath, bool isWrite)
        {
            if (string.IsNullOrEmpty(filePath))
                return false;

            var allowedPaths = isWrite ? _allowedWritableFilePaths : _allowedFilePaths;
            return allowedPaths.Any(regex => regex.IsMatch(filePath));
        }

        public bool IsAllowedExecutable(string fileName)
        {
            if (string.IsNullOrEmpty(fileName))
                return false;

            var executableName = Path.GetFileName(fileName);
            return _allowedExecutables.Contains(executableName);
        }

        public bool CanTerminateProcess(int processId)
        {
            try
            {
                var process = Process.GetProcessById(processId);
                
                // Don't allow terminating system processes
                if (process.ProcessName.Equals("System", StringComparison.OrdinalIgnoreCase) ||
                    process.ProcessName.Equals("csrss", StringComparison.OrdinalIgnoreCase) ||
                    process.ProcessName.Equals("winlogon", StringComparison.OrdinalIgnoreCase) ||
                    process.ProcessName.Equals("services", StringComparison.OrdinalIgnoreCase) ||
                    process.ProcessName.Equals("lsass", StringComparison.OrdinalIgnoreCase))
                {
                    return false;
                }

                // Only allow terminating processes that Luna started or user processes
                return true; // Could be made more restrictive
            }
            catch
            {
                return false;
            }
        }

        private ValidationResult ValidateRegistryOperation(BrokerRequest request, bool isWrite)
        {
            try
            {
                var parameters = System.Text.Json.JsonSerializer.Deserialize<RegistryParameters>(
                    request.Parameters?.ToString() ?? "{}");
                
                if (!IsAllowedRegistryPath(parameters.KeyPath, isWrite))
                {
                    return ValidationResult.Failure($"Registry path not allowed: {parameters.KeyPath}");
                }

                return ValidationResult.Success();
            }
            catch (Exception ex)
            {
                return ValidationResult.Failure($"Invalid registry parameters: {ex.Message}");
            }
        }

        private ValidationResult ValidateProcessStart(BrokerRequest request)
        {
            try
            {
                var parameters = System.Text.Json.JsonSerializer.Deserialize<ProcessParameters>(
                    request.Parameters?.ToString() ?? "{}");
                
                if (!IsAllowedExecutable(parameters.FileName))
                {
                    return ValidationResult.Failure($"Executable not allowed: {parameters.FileName}");
                }

                return ValidationResult.Success();
            }
            catch (Exception ex)
            {
                return ValidationResult.Failure($"Invalid process parameters: {ex.Message}");
            }
        }

        private ValidationResult ValidateProcessTerminate(BrokerRequest request)
        {
            try
            {
                var parameters = System.Text.Json.JsonSerializer.Deserialize<ProcessParameters>(
                    request.Parameters?.ToString() ?? "{}");
                
                if (!CanTerminateProcess(parameters.ProcessId))
                {
                    return ValidationResult.Failure($"Cannot terminate process: {parameters.ProcessId}");
                }

                return ValidationResult.Success();
            }
            catch (Exception ex)
            {
                return ValidationResult.Failure($"Invalid process parameters: {ex.Message}");
            }
        }

        private ValidationResult ValidateFileOperation(BrokerRequest request, bool isWrite)
        {
            try
            {
                var parameters = System.Text.Json.JsonSerializer.Deserialize<FileParameters>(
                    request.Parameters?.ToString() ?? "{}");
                
                if (!IsAllowedFilePath(parameters.FilePath, isWrite))
                {
                    return ValidationResult.Failure($"File path not allowed: {parameters.FilePath}");
                }

                return ValidationResult.Success();
            }
            catch (Exception ex)
            {
                return ValidationResult.Failure($"Invalid file parameters: {ex.Message}");
            }
        }
    }

    public class ValidationResult
    {
        public bool IsValid { get; set; }
        public string ErrorMessage { get; set; } = string.Empty;

        public static ValidationResult Success() => new() { IsValid = true };
        public static ValidationResult Failure(string errorMessage) => new() { IsValid = false, ErrorMessage = errorMessage };
    }
}