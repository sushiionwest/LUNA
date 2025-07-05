using System;
using System.IO;
using System.IO.Pipes;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using System.Security.AccessControl;
using System.Security.Principal;

namespace LunaBrokerService
{
    public class NamedPipeServer : IDisposable
    {
        private const string PipeName = "LunaBrokerService";
        private const int MaxConcurrentConnections = 4;
        
        private readonly SecurityValidator _securityValidator;
        private readonly ILogger _logger;
        private readonly CancellationTokenSource _cancellationTokenSource;
        private readonly SemaphoreSlim _connectionSemaphore;
        private bool _disposed;

        public NamedPipeServer(SecurityValidator securityValidator, ILogger logger)
        {
            _securityValidator = securityValidator ?? throw new ArgumentNullException(nameof(securityValidator));
            _logger = logger ?? throw new ArgumentNullException(nameof(logger));
            _cancellationTokenSource = new CancellationTokenSource();
            _connectionSemaphore = new SemaphoreSlim(MaxConcurrentConnections, MaxConcurrentConnections);
        }

        public async Task StartAsync(CancellationToken cancellationToken)
        {
            _logger.LogInformation("Starting Named Pipe Server on pipe: {PipeName}", PipeName);

            // Start accepting connections
            var tasks = new Task[MaxConcurrentConnections];
            for (int i = 0; i < MaxConcurrentConnections; i++)
            {
                tasks[i] = HandleClientConnectionsAsync(cancellationToken);
            }

            await Task.WhenAll(tasks);
        }

        private async Task HandleClientConnectionsAsync(CancellationToken cancellationToken)
        {
            while (!cancellationToken.IsCancellationRequested && !_disposed)
            {
                try
                {
                    await _connectionSemaphore.WaitAsync(cancellationToken);
                    
                    try
                    {
                        // Create named pipe with security restrictions
                        var pipeSecurity = CreatePipeSecurity();
                        using var pipeServer = NamedPipeServerStreamAcl.Create(
                            PipeName,
                            PipeDirection.InOut,
                            MaxConcurrentConnections,
                            PipeTransmissionMode.Message,
                            PipeOptions.Asynchronous,
                            4096, // inBufferSize
                            4096, // outBufferSize
                            pipeSecurity);

                        _logger.LogDebug("Waiting for client connection...");
                        await pipeServer.WaitForConnectionAsync(cancellationToken);
                        
                        _logger.LogInformation("Client connected to named pipe");
                        await HandleClientAsync(pipeServer, cancellationToken);
                    }
                    finally
                    {
                        _connectionSemaphore.Release();
                    }
                }
                catch (OperationCanceledException)
                {
                    _logger.LogInformation("Named pipe server cancelled");
                    break;
                }
                catch (Exception ex)
                {
                    _logger.LogError(ex, "Error in named pipe server connection handler");
                    // Brief delay before retrying to prevent tight loop on persistent errors
                    await Task.Delay(1000, cancellationToken);
                }
            }
        }

        private PipeSecurity CreatePipeSecurity()
        {
            var pipeSecurity = new PipeSecurity();
            
            // Allow the current user (SYSTEM when running as service) full control
            var systemSid = new SecurityIdentifier(WellKnownSidType.LocalSystemSid, null);
            pipeSecurity.AddAccessRule(new PipeAccessRule(systemSid, PipeAccessRights.FullControl, AccessControlType.Allow));
            
            // Allow local administrators full control
            var adminSid = new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null);
            pipeSecurity.AddAccessRule(new PipeAccessRule(adminSid, PipeAccessRights.FullControl, AccessControlType.Allow));
            
            // Allow authenticated users read/write access (this can be restricted further)
            var authenticatedUsersSid = new SecurityIdentifier(WellKnownSidType.AuthenticatedUserSid, null);
            pipeSecurity.AddAccessRule(new PipeAccessRule(authenticatedUsersSid, 
                PipeAccessRights.ReadWrite, AccessControlType.Allow));
            
            return pipeSecurity;
        }

        private async Task HandleClientAsync(NamedPipeServerStream pipeServer, CancellationToken cancellationToken)
        {
            try
            {
                // Get client identity for validation
                var clientIdentity = pipeServer.GetImpersonationUserName();
                _logger.LogInformation("Client identity: {ClientIdentity}", clientIdentity);

                using var reader = new StreamReader(pipeServer, Encoding.UTF8, leaveOpen: true);
                using var writer = new StreamWriter(pipeServer, Encoding.UTF8, leaveOpen: true) { AutoFlush = true };

                while (pipeServer.IsConnected && !cancellationToken.IsCancellationRequested)
                {
                    try
                    {
                        var requestJson = await reader.ReadLineAsync();
                        if (string.IsNullOrEmpty(requestJson))
                            break;

                        _logger.LogDebug("Received request: {Request}", requestJson);

                        var response = await ProcessRequestAsync(requestJson, clientIdentity, cancellationToken);
                        await writer.WriteLineAsync(response);
                    }
                    catch (IOException ex)
                    {
                        _logger.LogWarning(ex, "IO error while handling client request");
                        break;
                    }
                }
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error handling client connection");
            }
            finally
            {
                _logger.LogInformation("Client disconnected");
            }
        }

        private async Task<string> ProcessRequestAsync(string requestJson, string clientIdentity, CancellationToken cancellationToken)
        {
            try
            {
                var request = JsonSerializer.Deserialize<BrokerRequest>(requestJson);
                if (request == null)
                {
                    return CreateErrorResponse("Invalid request format");
                }

                // Validate the request
                var validationResult = await _securityValidator.ValidateRequestAsync(request, clientIdentity);
                if (!validationResult.IsValid)
                {
                    _logger.LogWarning("Request validation failed: {Reason}", validationResult.ErrorMessage);
                    return CreateErrorResponse($"Access denied: {validationResult.ErrorMessage}");
                }

                // Process the validated request
                var result = await ExecutePrivilegedOperationAsync(request, cancellationToken);
                
                return JsonSerializer.Serialize(new BrokerResponse
                {
                    Success = true,
                    Data = result,
                    Timestamp = DateTime.UtcNow
                });
            }
            catch (JsonException ex)
            {
                _logger.LogError(ex, "Failed to parse request JSON");
                return CreateErrorResponse("Invalid JSON format");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error processing request");
                return CreateErrorResponse($"Internal error: {ex.Message}");
            }
        }

        private async Task<object> ExecutePrivilegedOperationAsync(BrokerRequest request, CancellationToken cancellationToken)
        {
            _logger.LogInformation("Executing privileged operation: {Operation}", request.Operation);

            return request.Operation?.ToLowerInvariant() switch
            {
                "uiautomation.click" => await HandleUIAutomationClick(request),
                "uiautomation.sendkeys" => await HandleUIAutomationSendKeys(request),
                "uiautomation.getwindows" => await HandleUIAutomationGetWindows(request),
                "registry.read" => await HandleRegistryRead(request),
                "registry.write" => await HandleRegistryWrite(request),
                "process.start" => await HandleProcessStart(request),
                "process.terminate" => await HandleProcessTerminate(request),
                "file.read" => await HandleFileRead(request),
                "file.write" => await HandleFileWrite(request),
                "system.screenshot" => await HandleSystemScreenshot(request),
                _ => throw new InvalidOperationException($"Unknown operation: {request.Operation}")
            };
        }

        private async Task<object> HandleUIAutomationClick(BrokerRequest request)
        {
            var parameters = JsonSerializer.Deserialize<UIAutomationClickParameters>(request.Parameters?.ToString() ?? "{}");
            
            // Validate click coordinates are within screen bounds
            var screenBounds = System.Windows.Forms.Screen.PrimaryScreen.Bounds;
            if (parameters.X < 0 || parameters.X > screenBounds.Width || 
                parameters.Y < 0 || parameters.Y > screenBounds.Height)
            {
                throw new ArgumentException("Click coordinates are outside screen bounds");
            }

            // Execute the click using Windows API
            var result = WindowsAPI.PerformClick(parameters.X, parameters.Y, parameters.Button);
            
            return new { Success = result, X = parameters.X, Y = parameters.Y };
        }

        private async Task<object> HandleUIAutomationSendKeys(BrokerRequest request)
        {
            var parameters = JsonSerializer.Deserialize<UIAutomationSendKeysParameters>(request.Parameters?.ToString() ?? "{}");
            
            // Validate that we're not sending potentially dangerous key combinations
            if (_securityValidator.IsDangerousKeySequence(parameters.Keys))
            {
                throw new SecurityException("Dangerous key sequence blocked");
            }

            var result = WindowsAPI.SendKeys(parameters.Keys);
            return new { Success = result, Keys = parameters.Keys };
        }

        private async Task<object> HandleUIAutomationGetWindows(BrokerRequest request)
        {
            var windows = WindowsAPI.GetVisibleWindows();
            return new { Windows = windows };
        }

        private async Task<object> HandleRegistryRead(BrokerRequest request)
        {
            var parameters = JsonSerializer.Deserialize<RegistryParameters>(request.Parameters?.ToString() ?? "{}");
            
            // Only allow reading from safe registry paths
            if (!_securityValidator.IsAllowedRegistryPath(parameters.KeyPath, false))
            {
                throw new SecurityException("Registry path not allowed");
            }

            var value = WindowsAPI.ReadRegistryValue(parameters.KeyPath, parameters.ValueName);
            return new { Value = value };
        }

        private async Task<object> HandleRegistryWrite(BrokerRequest request)
        {
            var parameters = JsonSerializer.Deserialize<RegistryParameters>(request.Parameters?.ToString() ?? "{}");
            
            // Only allow writing to safe registry paths
            if (!_securityValidator.IsAllowedRegistryPath(parameters.KeyPath, true))
            {
                throw new SecurityException("Registry path not allowed for writing");
            }

            var success = WindowsAPI.WriteRegistryValue(parameters.KeyPath, parameters.ValueName, parameters.Value);
            return new { Success = success };
        }

        private async Task<object> HandleProcessStart(BrokerRequest request)
        {
            var parameters = JsonSerializer.Deserialize<ProcessParameters>(request.Parameters?.ToString() ?? "{}");
            
            // Validate that the executable is allowed
            if (!_securityValidator.IsAllowedExecutable(parameters.FileName))
            {
                throw new SecurityException("Executable not allowed");
            }

            var processId = WindowsAPI.StartProcess(parameters.FileName, parameters.Arguments);
            return new { ProcessId = processId };
        }

        private async Task<object> HandleProcessTerminate(BrokerRequest request)
        {
            var parameters = JsonSerializer.Deserialize<ProcessParameters>(request.Parameters?.ToString() ?? "{}");
            
            // Additional validation for process termination
            if (!_securityValidator.CanTerminateProcess(parameters.ProcessId))
            {
                throw new SecurityException("Process termination not allowed");
            }

            var success = WindowsAPI.TerminateProcess(parameters.ProcessId);
            return new { Success = success };
        }

        private async Task<object> HandleFileRead(BrokerRequest request)
        {
            var parameters = JsonSerializer.Deserialize<FileParameters>(request.Parameters?.ToString() ?? "{}");
            
            if (!_securityValidator.IsAllowedFilePath(parameters.FilePath, false))
            {
                throw new SecurityException("File path not allowed for reading");
            }

            var content = await File.ReadAllTextAsync(parameters.FilePath);
            return new { Content = content };
        }

        private async Task<object> HandleFileWrite(BrokerRequest request)
        {
            var parameters = JsonSerializer.Deserialize<FileParameters>(request.Parameters?.ToString() ?? "{}");
            
            if (!_securityValidator.IsAllowedFilePath(parameters.FilePath, true))
            {
                throw new SecurityException("File path not allowed for writing");
            }

            await File.WriteAllTextAsync(parameters.FilePath, parameters.Content);
            return new { Success = true };
        }

        private async Task<object> HandleSystemScreenshot(BrokerRequest request)
        {
            var screenshotPath = WindowsAPI.TakeScreenshot();
            return new { ScreenshotPath = screenshotPath };
        }

        private string CreateErrorResponse(string errorMessage)
        {
            return JsonSerializer.Serialize(new BrokerResponse
            {
                Success = false,
                ErrorMessage = errorMessage,
                Timestamp = DateTime.UtcNow
            });
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                _cancellationTokenSource.Cancel();
                _connectionSemaphore?.Dispose();
                _cancellationTokenSource?.Dispose();
                _disposed = true;
            }
        }
    }
}