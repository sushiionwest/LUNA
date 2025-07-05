using System;
using System.Diagnostics;
using System.Drawing;
using System.Windows.Forms;
using System.Threading;
using System.Threading.Tasks;
using System.Management;
using System.IO;
using System.Text.Json;

namespace LunaPerformanceTray
{
    public partial class PerformanceTrayApp : Form
    {
        private NotifyIcon trayIcon;
        private ContextMenuStrip trayMenu;
        private Timer updateTimer;
        private PerformanceCounter cpuCounter;
        private PerformanceCounter ramCounter;
        private Process lunaProcess;
        
        // Performance data
        private float currentCpuUsage = 0;
        private long currentMemoryUsage = 0;
        private long totalMemory = 0;
        private DateTime lastUpdateTime = DateTime.Now;
        
        // Thresholds
        private const float CPU_WARNING_THRESHOLD = 70.0f;
        private const float CPU_CRITICAL_THRESHOLD = 90.0f;
        private const float MEMORY_WARNING_THRESHOLD = 80.0f;
        private const float MEMORY_CRITICAL_THRESHOLD = 95.0f;
        
        // UI state
        private bool showDetailedInfo = false;
        private Form detailWindow = null;

        public PerformanceTrayApp()
        {
            InitializeComponent();
            SetupTrayIcon();
            SetupPerformanceCounters();
            StartMonitoring();
        }

        private void InitializeComponent()
        {
            this.WindowState = FormWindowState.Minimized;
            this.ShowInTaskbar = false;
            this.Visible = false;
        }

        private void SetupTrayIcon()
        {
            // Create tray menu
            trayMenu = new ContextMenuStrip();
            
            var showMenuItem = new ToolStripMenuItem("Show Details", null, ShowDetails);
            var separatorMenuItem = new ToolStripSeparator();
            var settingsMenuItem = new ToolStripMenuItem("Settings", null, OpenSettings);
            var resetMenuItem = new ToolStripMenuItem("Reset Counters", null, ResetCounters);
            var exitMenuItem = new ToolStripMenuItem("Exit", null, Exit);
            
            trayMenu.Items.AddRange(new ToolStripItem[]
            {
                showMenuItem, separatorMenuItem, settingsMenuItem, resetMenuItem, exitMenuItem
            });

            // Create tray icon
            trayIcon = new NotifyIcon()
            {
                Text = "Luna Performance Monitor",
                Icon = CreateDynamicIcon(0, 0), // Start with 0% CPU and memory
                ContextMenuStrip = trayMenu,
                Visible = true
            };

            trayIcon.MouseDoubleClick += ShowDetails;
        }

        private void SetupPerformanceCounters()
        {
            try
            {
                cpuCounter = new PerformanceCounter("Processor", "% Processor Time", "_Total");
                ramCounter = new PerformanceCounter("Memory", "Available MBytes");
                
                // Get total physical memory
                var computerInfo = new Microsoft.VisualBasic.Devices.ComputerInfo();
                totalMemory = (long)computerInfo.TotalPhysicalMemory;
                
                // Initial reading (first reading is often 0)
                cpuCounter.NextValue();
                ramCounter.NextValue();
            }
            catch (Exception ex)
            {
                MessageBox.Show($"Failed to initialize performance counters: {ex.Message}", 
                    "Luna Performance Monitor", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            }
        }

        private void StartMonitoring()
        {
            updateTimer = new Timer();
            updateTimer.Interval = 2000; // Update every 2 seconds
            updateTimer.Tick += UpdatePerformanceData;
            updateTimer.Start();
            
            // Find Luna process
            Task.Run(FindLunaProcess);
        }

        private async void FindLunaProcess()
        {
            while (lunaProcess == null || lunaProcess.HasExited)
            {
                try
                {
                    var processes = Process.GetProcessesByName("luna-agent");
                    if (processes.Length > 0)
                    {
                        lunaProcess = processes[0];
                    }
                    else
                    {
                        // Try alternative process names
                        processes = Process.GetProcessesByName("node");
                        foreach (var proc in processes)
                        {
                            try
                            {
                                if (proc.MainModule?.FileName?.Contains("luna") == true)
                                {
                                    lunaProcess = proc;
                                    break;
                                }
                            }
                            catch { /* Access denied for some processes */ }
                        }
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"Error finding Luna process: {ex.Message}");
                }

                await Task.Delay(5000); // Check every 5 seconds
            }
        }

        private void UpdatePerformanceData(object sender, EventArgs e)
        {
            try
            {
                // Update CPU usage
                currentCpuUsage = cpuCounter.NextValue();
                
                // Update memory usage
                var availableMemoryMB = ramCounter.NextValue();
                var availableMemoryBytes = (long)(availableMemoryMB * 1024 * 1024);
                currentMemoryUsage = totalMemory - availableMemoryBytes;
                
                var memoryUsagePercent = (float)(currentMemoryUsage * 100.0 / totalMemory);
                
                // Update tray icon
                trayIcon.Icon = CreateDynamicIcon(currentCpuUsage, memoryUsagePercent);
                
                // Update tooltip
                var tooltipText = $"Luna Performance Monitor\n" +
                                 $"CPU: {currentCpuUsage:F1}%\n" +
                                 $"Memory: {memoryUsagePercent:F1}% ({FormatBytes(currentMemoryUsage)})";
                
                if (lunaProcess != null && !lunaProcess.HasExited)
                {
                    try
                    {
                        var lunaMemory = lunaProcess.WorkingSet64;
                        tooltipText += $"\nLuna Process: {FormatBytes(lunaMemory)}";
                    }
                    catch { /* Process might have exited */ }
                }
                
                trayIcon.Text = tooltipText.Length > 127 ? tooltipText.Substring(0, 127) : tooltipText;
                
                // Check for alerts
                CheckPerformanceAlerts(currentCpuUsage, memoryUsagePercent);
                
                // Update detail window if open
                UpdateDetailWindow();
                
                lastUpdateTime = DateTime.Now;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error updating performance data: {ex.Message}");
            }
        }

        private Icon CreateDynamicIcon(float cpuPercent, float memoryPercent)
        {
            var bitmap = new Bitmap(16, 16);
            using (var graphics = Graphics.FromImage(bitmap))
            {
                // Clear background
                graphics.Clear(Color.Transparent);
                
                // Determine color based on usage levels
                var cpuColor = GetUsageColor(cpuPercent);
                var memoryColor = GetUsageColor(memoryPercent);
                
                // Draw CPU bar (left half)
                var cpuHeight = (int)(14 * cpuPercent / 100);
                if (cpuHeight > 0)
                {
                    graphics.FillRectangle(new SolidBrush(cpuColor), 1, 14 - cpuHeight, 6, cpuHeight);
                }
                
                // Draw memory bar (right half)
                var memoryHeight = (int)(14 * memoryPercent / 100);
                if (memoryHeight > 0)
                {
                    graphics.FillRectangle(new SolidBrush(memoryColor), 9, 14 - memoryHeight, 6, memoryHeight);
                }
                
                // Draw border
                graphics.DrawRectangle(Pens.Gray, 0, 0, 7, 15);
                graphics.DrawRectangle(Pens.Gray, 8, 0, 7, 15);
            }
            
            return Icon.FromHandle(bitmap.GetHicon());
        }

        private Color GetUsageColor(float percent)
        {
            if (percent >= CPU_CRITICAL_THRESHOLD) return Color.Red;
            if (percent >= CPU_WARNING_THRESHOLD) return Color.Orange;
            if (percent >= 50) return Color.Yellow;
            return Color.Green;
        }

        private void CheckPerformanceAlerts(float cpuPercent, float memoryPercent)
        {
            // Only show alerts once per minute to avoid spam
            if ((DateTime.Now - lastUpdateTime).TotalMinutes < 1) return;
            
            if (cpuPercent >= CPU_CRITICAL_THRESHOLD)
            {
                ShowBalloonTip("Critical CPU Usage", 
                    $"CPU usage is critically high: {cpuPercent:F1}%", ToolTipIcon.Error);
            }
            else if (cpuPercent >= CPU_WARNING_THRESHOLD)
            {
                ShowBalloonTip("High CPU Usage", 
                    $"CPU usage is high: {cpuPercent:F1}%", ToolTipIcon.Warning);
            }
            
            if (memoryPercent >= MEMORY_CRITICAL_THRESHOLD)
            {
                ShowBalloonTip("Critical Memory Usage", 
                    $"Memory usage is critically high: {memoryPercent:F1}%", ToolTipIcon.Error);
            }
            else if (memoryPercent >= MEMORY_WARNING_THRESHOLD)
            {
                ShowBalloonTip("High Memory Usage", 
                    $"Memory usage is high: {memoryPercent:F1}%", ToolTipIcon.Warning);
            }
        }

        private void ShowBalloonTip(string title, string text, ToolTipIcon icon)
        {
            trayIcon.ShowBalloonTip(5000, title, text, icon);
        }

        private void ShowDetails(object sender, EventArgs e)
        {
            if (detailWindow == null || detailWindow.IsDisposed)
            {
                detailWindow = new PerformanceDetailWindow(this);
            }
            
            detailWindow.Show();
            detailWindow.BringToFront();
        }

        private void OpenSettings(object sender, EventArgs e)
        {
            var settingsForm = new SettingsForm();
            settingsForm.ShowDialog();
        }

        private void ResetCounters(object sender, EventArgs e)
        {
            try
            {
                // Reset performance counters
                cpuCounter?.Dispose();
                ramCounter?.Dispose();
                SetupPerformanceCounters();
                
                ShowBalloonTip("Counters Reset", "Performance counters have been reset.", ToolTipIcon.Info);
            }
            catch (Exception ex)
            {
                MessageBox.Show($"Failed to reset counters: {ex.Message}", 
                    "Error", MessageBoxButtons.OK, MessageBoxIcon.Error);
            }
        }

        private void Exit(object sender, EventArgs e)
        {
            Application.Exit();
        }

        private void UpdateDetailWindow()
        {
            if (detailWindow is PerformanceDetailWindow detailWin && !detailWin.IsDisposed)
            {
                detailWin.UpdateData(currentCpuUsage, currentMemoryUsage, totalMemory, lunaProcess);
            }
        }

        public PerformanceData GetCurrentData()
        {
            return new PerformanceData
            {
                CpuUsage = currentCpuUsage,
                MemoryUsage = currentMemoryUsage,
                TotalMemory = totalMemory,
                LastUpdate = lastUpdateTime,
                LunaProcessMemory = lunaProcess?.WorkingSet64 ?? 0
            };
        }

        private string FormatBytes(long bytes)
        {
            string[] sizes = { "B", "KB", "MB", "GB", "TB" };
            double len = bytes;
            int order = 0;
            while (len >= 1024 && order < sizes.Length - 1)
            {
                order++;
                len = len / 1024;
            }
            return $"{len:0.##} {sizes[order]}";
        }

        protected override void Dispose(bool disposing)
        {
            if (disposing)
            {
                updateTimer?.Dispose();
                cpuCounter?.Dispose();
                ramCounter?.Dispose();
                trayIcon?.Dispose();
                detailWindow?.Dispose();
            }
            base.Dispose(disposing);
        }

        protected override void SetVisibleCore(bool value)
        {
            base.SetVisibleCore(false); // Never show the main form
        }
    }

    public class PerformanceData
    {
        public float CpuUsage { get; set; }
        public long MemoryUsage { get; set; }
        public long TotalMemory { get; set; }
        public DateTime LastUpdate { get; set; }
        public long LunaProcessMemory { get; set; }
    }

    public class PerformanceDetailWindow : Form
    {
        private readonly PerformanceTrayApp parentApp;
        private Label cpuLabel;
        private ProgressBar cpuProgressBar;
        private Label memoryLabel;
        private ProgressBar memoryProgressBar;
        private Label lunaProcessLabel;
        private ListView detailsList;
        private Timer refreshTimer;

        public PerformanceDetailWindow(PerformanceTrayApp parent)
        {
            parentApp = parent;
            InitializeDetailWindow();
            SetupRefreshTimer();
        }

        private void InitializeDetailWindow()
        {
            this.Text = "Luna Performance Details";
            this.Size = new Size(400, 300);
            this.StartPosition = FormStartPosition.CenterScreen;
            this.FormBorderStyle = FormBorderStyle.FixedSingle;
            this.MaximizeBox = false;

            var panel = new TableLayoutPanel
            {
                Dock = DockStyle.Fill,
                ColumnCount = 2,
                RowCount = 6,
                Padding = new Padding(10)
            };

            // CPU section
            panel.Controls.Add(new Label { Text = "CPU Usage:", TextAlign = ContentAlignment.MiddleLeft }, 0, 0);
            cpuProgressBar = new ProgressBar { Dock = DockStyle.Fill };
            panel.Controls.Add(cpuProgressBar, 1, 0);
            
            cpuLabel = new Label { Text = "0%", TextAlign = ContentAlignment.MiddleLeft };
            panel.Controls.Add(cpuLabel, 1, 1);

            // Memory section
            panel.Controls.Add(new Label { Text = "Memory Usage:", TextAlign = ContentAlignment.MiddleLeft }, 0, 2);
            memoryProgressBar = new ProgressBar { Dock = DockStyle.Fill };
            panel.Controls.Add(memoryProgressBar, 1, 2);
            
            memoryLabel = new Label { Text = "0%", TextAlign = ContentAlignment.MiddleLeft };
            panel.Controls.Add(memoryLabel, 1, 3);

            // Luna process section
            panel.Controls.Add(new Label { Text = "Luna Process:", TextAlign = ContentAlignment.MiddleLeft }, 0, 4);
            lunaProcessLabel = new Label { Text = "Not found", TextAlign = ContentAlignment.MiddleLeft };
            panel.Controls.Add(lunaProcessLabel, 1, 4);

            // Details list
            detailsList = new ListView
            {
                View = View.Details,
                FullRowSelect = true,
                GridLines = true,
                Dock = DockStyle.Fill
            };
            detailsList.Columns.Add("Property", 120);
            detailsList.Columns.Add("Value", 150);
            
            panel.SetColumnSpan(detailsList, 2);
            panel.Controls.Add(detailsList, 0, 5);

            this.Controls.Add(panel);
        }

        private void SetupRefreshTimer()
        {
            refreshTimer = new Timer { Interval = 1000 };
            refreshTimer.Tick += (s, e) => RefreshData();
            refreshTimer.Start();
        }

        private void RefreshData()
        {
            var data = parentApp.GetCurrentData();
            UpdateData(data.CpuUsage, data.MemoryUsage, data.TotalMemory, null);
        }

        public void UpdateData(float cpuUsage, long memoryUsage, long totalMemory, Process lunaProcess)
        {
            if (this.InvokeRequired)
            {
                this.Invoke(new Action<float, long, long, Process>(UpdateData), cpuUsage, memoryUsage, totalMemory, lunaProcess);
                return;
            }

            // Update CPU
            cpuProgressBar.Value = Math.Min(100, (int)cpuUsage);
            cpuLabel.Text = $"{cpuUsage:F1}%";
            cpuLabel.ForeColor = GetUsageColor(cpuUsage);

            // Update Memory
            var memoryPercent = (float)(memoryUsage * 100.0 / totalMemory);
            memoryProgressBar.Value = Math.Min(100, (int)memoryPercent);
            memoryLabel.Text = $"{memoryPercent:F1}% ({FormatBytes(memoryUsage)} / {FormatBytes(totalMemory)})";
            memoryLabel.ForeColor = GetUsageColor(memoryPercent);

            // Update Luna process
            if (lunaProcess != null && !lunaProcess.HasExited)
            {
                try
                {
                    var lunaMemory = lunaProcess.WorkingSet64;
                    lunaProcessLabel.Text = $"PID {lunaProcess.Id}: {FormatBytes(lunaMemory)}";
                    lunaProcessLabel.ForeColor = Color.Green;
                }
                catch
                {
                    lunaProcessLabel.Text = "Process access denied";
                    lunaProcessLabel.ForeColor = Color.Orange;
                }
            }
            else
            {
                lunaProcessLabel.Text = "Not running";
                lunaProcessLabel.ForeColor = Color.Red;
            }

            // Update details list
            UpdateDetailsList(cpuUsage, memoryUsage, totalMemory, lunaProcess);
        }

        private void UpdateDetailsList(float cpuUsage, long memoryUsage, long totalMemory, Process lunaProcess)
        {
            detailsList.Items.Clear();

            detailsList.Items.Add(new ListViewItem(new[] { "CPU Cores", Environment.ProcessorCount.ToString() }));
            detailsList.Items.Add(new ListViewItem(new[] { "System Uptime", GetSystemUptime() }));
            
            if (lunaProcess != null && !lunaProcess.HasExited)
            {
                try
                {
                    var startTime = lunaProcess.StartTime;
                    var uptime = DateTime.Now - startTime;
                    detailsList.Items.Add(new ListViewItem(new[] { "Luna Uptime", $"{uptime.Days}d {uptime.Hours}h {uptime.Minutes}m" }));
                    detailsList.Items.Add(new ListViewItem(new[] { "Luna CPU Time", FormatTimeSpan(lunaProcess.TotalProcessorTime) }));
                }
                catch { }
            }

            var availableMemory = totalMemory - memoryUsage;
            detailsList.Items.Add(new ListViewItem(new[] { "Available Memory", FormatBytes(availableMemory) }));
        }

        private Color GetUsageColor(float percent)
        {
            if (percent >= 90) return Color.Red;
            if (percent >= 70) return Color.Orange;
            if (percent >= 50) return Color.Yellow;
            return Color.Green;
        }

        private string FormatBytes(long bytes)
        {
            string[] sizes = { "B", "KB", "MB", "GB", "TB" };
            double len = bytes;
            int order = 0;
            while (len >= 1024 && order < sizes.Length - 1)
            {
                order++;
                len = len / 1024;
            }
            return $"{len:0.##} {sizes[order]}";
        }

        private string GetSystemUptime()
        {
            var uptime = TimeSpan.FromMilliseconds(Environment.TickCount);
            return $"{uptime.Days}d {uptime.Hours}h {uptime.Minutes}m";
        }

        private string FormatTimeSpan(TimeSpan timeSpan)
        {
            return $"{timeSpan.Hours}h {timeSpan.Minutes}m {timeSpan.Seconds}s";
        }

        protected override void Dispose(bool disposing)
        {
            if (disposing)
            {
                refreshTimer?.Dispose();
            }
            base.Dispose(disposing);
        }
    }

    public class SettingsForm : Form
    {
        // Settings form implementation would go here
        public SettingsForm()
        {
            this.Text = "Performance Monitor Settings";
            this.Size = new Size(300, 200);
            this.StartPosition = FormStartPosition.CenterParent;
            
            var label = new Label
            {
                Text = "Settings panel (to be implemented)",
                Dock = DockStyle.Fill,
                TextAlign = ContentAlignment.MiddleCenter
            };
            
            this.Controls.Add(label);
        }
    }

    static class Program
    {
        [STAThread]
        static void Main()
        {
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);
            Application.Run(new PerformanceTrayApp());
        }
    }
}