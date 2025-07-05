using System;
using System.ServiceProcess;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Serilog;

namespace LunaBrokerService
{
    internal static class Program
    {
        /// <summary>
        /// The main entry point for the application.
        /// </summary>
        private static void Main(string[] args)
        {
            // Configure Serilog for logging
            Log.Logger = new LoggerConfiguration()
                .MinimumLevel.Information()
                .WriteTo.EventLog("Luna Broker Service", manageEventSource: true)
                .WriteTo.File(@"C:\ProgramData\Luna\Logs\broker-service-.log", 
                    rollingInterval: RollingInterval.Day,
                    retainedFileCountLimit: 7)
                .CreateLogger();

            try
            {
                Log.Information("Luna Broker Service starting up");

                if (Environment.UserInteractive)
                {
                    // Running as console application (debug mode)
                    Console.WriteLine("Luna Broker Service - Debug Mode");
                    Console.WriteLine("Press Ctrl+C to stop the service");
                    
                    var service = new BrokerService();
                    service.StartDebug();
                    
                    Console.ReadKey();
                    service.StopDebug();
                }
                else
                {
                    // Running as Windows Service
                    ServiceBase[] ServicesToRun;
                    ServicesToRun = new ServiceBase[]
                    {
                        new BrokerService()
                    };
                    ServiceBase.Run(ServicesToRun);
                }
            }
            catch (Exception ex)
            {
                Log.Fatal(ex, "Luna Broker Service failed to start");
                throw;
            }
            finally
            {
                Log.CloseAndFlush();
            }
        }
    }
}