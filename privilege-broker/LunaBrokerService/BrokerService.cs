using System;
using System.ServiceProcess;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Serilog;

namespace LunaBrokerService
{
    public partial class BrokerService : ServiceBase
    {
        private readonly ILogger<BrokerService> _logger;
        private CancellationTokenSource _cancellationTokenSource;
        private Task _serviceTask;
        private NamedPipeServer _namedPipeServer;
        private SecurityValidator _securityValidator;

        public BrokerService()
        {
            InitializeComponent();
            
            // Configure logging
            var loggerFactory = LoggerFactory.Create(builder =>
                builder.AddSerilog(Log.Logger));
            _logger = loggerFactory.CreateLogger<BrokerService>();
            
            _cancellationTokenSource = new CancellationTokenSource();
            _securityValidator = new SecurityValidator(_logger);
            _namedPipeServer = new NamedPipeServer(_securityValidator, _logger);
        }

        protected override void OnStart(string[] args)
        {
            _logger.LogInformation("Luna Broker Service starting...");
            
            try
            {
                _serviceTask = RunServiceAsync(_cancellationTokenSource.Token);
                _logger.LogInformation("Luna Broker Service started successfully");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to start Luna Broker Service");
                throw;
            }
        }

        protected override void OnStop()
        {
            _logger.LogInformation("Luna Broker Service stopping...");
            
            try
            {
                _cancellationTokenSource.Cancel();
                _serviceTask?.Wait(TimeSpan.FromSeconds(30));
                _namedPipeServer?.Dispose();
                _logger.LogInformation("Luna Broker Service stopped");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error while stopping Luna Broker Service");
            }
        }

        public void StartDebug()
        {
            _logger.LogInformation("Luna Broker Service starting in debug mode...");
            _serviceTask = RunServiceAsync(_cancellationTokenSource.Token);
        }

        public void StopDebug()
        {
            _logger.LogInformation("Luna Broker Service stopping debug mode...");
            _cancellationTokenSource.Cancel();
            _serviceTask?.Wait(TimeSpan.FromSeconds(30));
            _namedPipeServer?.Dispose();
        }

        private async Task RunServiceAsync(CancellationToken cancellationToken)
        {
            _logger.LogInformation("Luna Broker Service main loop starting");

            try
            {
                // Start the named pipe server
                await _namedPipeServer.StartAsync(cancellationToken);

                // Keep the service running
                while (!cancellationToken.IsCancellationRequested)
                {
                    await Task.Delay(1000, cancellationToken);
                }
            }
            catch (OperationCanceledException)
            {
                _logger.LogInformation("Luna Broker Service was cancelled");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Unexpected error in Luna Broker Service main loop");
                throw;
            }
        }

        protected override void Dispose(bool disposing)
        {
            if (disposing)
            {
                _cancellationTokenSource?.Dispose();
                _namedPipeServer?.Dispose();
            }
            base.Dispose(disposing);
        }
    }
}