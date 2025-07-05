# Luna Performance Monitoring Script
# This script monitors system performance while k6 load tests are running
# and fails if performance thresholds are exceeded

param(
    [Parameter(Mandatory=$false)]
    [int]$DurationMinutes = 10,
    
    [Parameter(Mandatory=$false)]
    [int]$SampleIntervalSeconds = 5,
    
    [Parameter(Mandatory=$false)]
    [double]$MaxAverageCpu = 30.0,
    
    [Parameter(Mandatory=$false)]
    [long]$MaxMemoryUsageBytes = 1GB,
    
    [Parameter(Mandatory=$false)]
    [string]$LunaProcessName = "node",
    
    [Parameter(Mandatory=$false)]
    [string]$OutputFile = "performance-report.json"
)

# Performance monitoring variables
$startTime = Get-Date
$endTime = $startTime.AddMinutes($DurationMinutes)
$samples = @()
$lunaProcess = $null
$performanceCounters = @{}

Write-Host "🔍 Luna Performance Monitor Starting" -ForegroundColor Green
Write-Host "   Duration: $DurationMinutes minutes"
Write-Host "   Sample interval: $SampleIntervalSeconds seconds"
Write-Host "   Max average CPU: $MaxAverageCpu%"
Write-Host "   Max memory usage: $([math]::Round($MaxMemoryUsageBytes / 1GB, 2)) GB"
Write-Host ""

# Initialize performance counters
try {
    $performanceCounters.CPU = Get-Counter "\Processor(_Total)\% Processor Time"
    $performanceCounters.Memory = Get-Counter "\Memory\Available MBytes"
    $performanceCounters.DiskIO = Get-Counter "\PhysicalDisk(_Total)\Disk Read Bytes/sec", "\PhysicalDisk(_Total)\Disk Write Bytes/sec"
    Write-Host "✅ Performance counters initialized" -ForegroundColor Green
} catch {
    Write-Error "❌ Failed to initialize performance counters: $_"
    exit 1
}

# Find Luna process
function Find-LunaProcess {
    $processes = Get-Process -Name $LunaProcessName -ErrorAction SilentlyContinue
    foreach ($proc in $processes) {
        try {
            if ($proc.MainModule.FileName -like "*luna*" -or 
                $proc.ProcessName -eq "luna-agent" -or
                (Get-Process -Id $proc.Id).CommandLine -like "*luna*") {
                return $proc
            }
        } catch {
            # Access denied or process exited
        }
    }
    return $null
}

# Function to collect performance sample
function Get-PerformanceSample {
    $timestamp = Get-Date
    
    try {
        # System CPU usage
        $cpuCounter = Get-Counter "\Processor(_Total)\% Processor Time" -SampleInterval 1 -MaxSamples 1
        $cpuUsage = $cpuCounter.CounterSamples[0].CookedValue
        
        # System memory usage
        $memCounter = Get-Counter "\Memory\Available MBytes" -SampleInterval 1 -MaxSamples 1
        $availableMemoryMB = $memCounter.CounterSamples[0].CookedValue
        $totalMemoryMB = (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1MB
        $usedMemoryMB = $totalMemoryMB - $availableMemoryMB
        $memoryUsagePercent = ($usedMemoryMB / $totalMemoryMB) * 100
        
        # Disk I/O
        $diskReadCounter = Get-Counter "\PhysicalDisk(_Total)\Disk Read Bytes/sec" -SampleInterval 1 -MaxSamples 1
        $diskWriteCounter = Get-Counter "\PhysicalDisk(_Total)\Disk Write Bytes/sec" -SampleInterval 1 -MaxSamples 1
        $diskReadBytesPerSec = $diskReadCounter.CounterSamples[0].CookedValue
        $diskWriteBytesPerSec = $diskWriteCounter.CounterSamples[0].CookedValue
        
        # Luna process specific metrics
        $lunaMetrics = $null
        if ($script:lunaProcess -and !$script:lunaProcess.HasExited) {
            try {
                $script:lunaProcess.Refresh()
                $lunaMetrics = @{
                    ProcessId = $script:lunaProcess.Id
                    ProcessName = $script:lunaProcess.ProcessName
                    WorkingSet = $script:lunaProcess.WorkingSet64
                    PrivateMemory = $script:lunaProcess.PrivateMemorySize64
                    VirtualMemory = $script:lunaProcess.VirtualMemorySize64
                    TotalProcessorTime = $script:lunaProcess.TotalProcessorTime.TotalMilliseconds
                    StartTime = $script:lunaProcess.StartTime
                    Responding = $script:lunaProcess.Responding
                }
            } catch {
                Write-Warning "⚠️ Could not get Luna process metrics: $_"
            }
        }
        
        $sample = @{
            Timestamp = $timestamp
            System = @{
                CpuUsage = [math]::Round($cpuUsage, 2)
                MemoryUsagePercent = [math]::Round($memoryUsagePercent, 2)
                MemoryUsedMB = [math]::Round($usedMemoryMB, 2)
                MemoryAvailableMB = [math]::Round($availableMemoryMB, 2)
                DiskReadBytesPerSec = [math]::Round($diskReadBytesPerSec, 0)
                DiskWriteBytesPerSec = [math]::Round($diskWriteBytesPerSec, 0)
            }
            Luna = $lunaMetrics
        }
        
        return $sample
    } catch {
        Write-Error "❌ Error collecting performance sample: $_"
        return $null
    }
}

# Function to display real-time performance
function Show-PerformanceStatus {
    param($Sample)
    
    $cpu = $Sample.System.CpuUsage
    $memory = $Sample.System.MemoryUsagePercent
    $lunaMemory = if ($Sample.Luna) { [math]::Round($Sample.Luna.WorkingSet / 1MB, 1) } else { "N/A" }
    
    # Color coding based on thresholds
    $cpuColor = if ($cpu -gt $MaxAverageCpu) { "Red" } elseif ($cpu -gt ($MaxAverageCpu * 0.8)) { "Yellow" } else { "Green" }
    $memoryColor = if ($memory -gt 80) { "Red" } elseif ($memory -gt 60) { "Yellow" } else { "Green" }
    
    $status = "$(Get-Date -Format 'HH:mm:ss') | CPU: $($cpu)% | Memory: $($memory)% | Luna: $($lunaMemory)MB"
    Write-Host $status -ForegroundColor $cpuColor
}

# Function to check performance thresholds during monitoring
function Test-PerformanceThresholds {
    param($Samples)
    
    if ($Samples.Count -eq 0) { return $true }
    
    # Calculate averages
    $avgCpu = ($Samples | Measure-Object -Property {$_.System.CpuUsage} -Average).Average
    $maxMemory = ($Samples | Measure-Object -Property {$_.System.MemoryUsedMB} -Maximum).Maximum * 1MB
    $currentMemory = $Samples[-1].System.MemoryUsedMB * 1MB
    
    # Check thresholds
    $cpuExceeded = $avgCpu -gt $MaxAverageCpu
    $memoryExceeded = $currentMemory -gt $MaxMemoryUsageBytes
    
    if ($cpuExceeded) {
        Write-Warning "🚨 CPU threshold exceeded! Average: $([math]::Round($avgCpu, 2))% > $MaxAverageCpu%"
    }
    
    if ($memoryExceeded) {
        $currentMemoryGB = [math]::Round($currentMemory / 1GB, 2)
        $maxMemoryGB = [math]::Round($MaxMemoryUsageBytes / 1GB, 2)
        Write-Warning "🚨 Memory threshold exceeded! Current: ${currentMemoryGB}GB > ${maxMemoryGB}GB"
    }
    
    return -not ($cpuExceeded -or $memoryExceeded)
}

# Main monitoring loop
Write-Host "🔄 Starting performance monitoring..." -ForegroundColor Yellow

try {
    while ((Get-Date) -lt $endTime) {
        # Find Luna process if not found
        if (-not $script:lunaProcess -or $script:lunaProcess.HasExited) {
            $script:lunaProcess = Find-LunaProcess
            if ($script:lunaProcess) {
                Write-Host "✅ Found Luna process (PID: $($script:lunaProcess.Id))" -ForegroundColor Green
            }
        }
        
        # Collect performance sample
        $sample = Get-PerformanceSample
        if ($sample) {
            $samples += $sample
            Show-PerformanceStatus -Sample $sample
            
            # Check thresholds every 10 samples (real-time alerting)
            if ($samples.Count % 10 -eq 0) {
                $thresholdsPassed = Test-PerformanceThresholds -Samples $samples
                if (-not $thresholdsPassed) {
                    Write-Warning "⚠️ Performance thresholds exceeded during monitoring"
                }
            }
        }
        
        Start-Sleep -Seconds $SampleIntervalSeconds
    }
} catch {
    Write-Error "❌ Error during monitoring: $_"
    exit 1
}

Write-Host "⏹️ Performance monitoring completed" -ForegroundColor Green

# Analyze results
Write-Host "`n📊 Performance Analysis:" -ForegroundColor Cyan

if ($samples.Count -eq 0) {
    Write-Error "❌ No performance samples collected"
    exit 1
}

# Calculate statistics
$cpuStats = $samples | Measure-Object -Property {$_.System.CpuUsage} -Average -Maximum -Minimum
$memoryStats = $samples | Measure-Object -Property {$_.System.MemoryUsagePercent} -Average -Maximum -Minimum
$diskReadStats = $samples | Measure-Object -Property {$_.System.DiskReadBytesPerSec} -Average -Maximum
$diskWriteStats = $samples | Measure-Object -Property {$_.System.DiskWriteBytesPerSec} -Average -Maximum

$avgCpu = [math]::Round($cpuStats.Average, 2)
$maxCpu = [math]::Round($cpuStats.Maximum, 2)
$minCpu = [math]::Round($cpuStats.Minimum, 2)

$avgMemory = [math]::Round($memoryStats.Average, 2)
$maxMemory = [math]::Round($memoryStats.Maximum, 2)
$minMemory = [math]::Round($memoryStats.Minimum, 2)

$avgDiskRead = [math]::Round($diskReadStats.Average / 1MB, 2)
$avgDiskWrite = [math]::Round($diskWriteStats.Average / 1MB, 2)

Write-Host "CPU Usage:" -ForegroundColor White
Write-Host "   Average: $avgCpu% (threshold: $MaxAverageCpu%)"
Write-Host "   Maximum: $maxCpu%"
Write-Host "   Minimum: $minCpu%"

Write-Host "Memory Usage:" -ForegroundColor White
Write-Host "   Average: $avgMemory%"
Write-Host "   Maximum: $maxMemory%"
Write-Host "   Minimum: $minMemory%"

Write-Host "Disk I/O:" -ForegroundColor White
Write-Host "   Average Read: $avgDiskRead MB/s"
Write-Host "   Average Write: $avgDiskWrite MB/s"

# Luna process statistics
$lunaWithMetrics = $samples | Where-Object { $_.Luna -ne $null }
if ($lunaWithMetrics.Count -gt 0) {
    $lunaMemoryStats = $lunaWithMetrics | Measure-Object -Property {$_.Luna.WorkingSet} -Average -Maximum -Minimum
    $avgLunaMemoryMB = [math]::Round($lunaMemoryStats.Average / 1MB, 2)
    $maxLunaMemoryMB = [math]::Round($lunaMemoryStats.Maximum / 1MB, 2)
    
    Write-Host "Luna Process:" -ForegroundColor White
    Write-Host "   Average Memory: $avgLunaMemoryMB MB"
    Write-Host "   Peak Memory: $maxLunaMemoryMB MB"
    Write-Host "   Samples with data: $($lunaWithMetrics.Count)/$($samples.Count)"
}

# Generate detailed report
$report = @{
    TestConfiguration = @{
        Duration = $DurationMinutes
        SampleInterval = $SampleIntervalSeconds
        MaxAverageCpu = $MaxAverageCpu
        MaxMemoryUsageBytes = $MaxMemoryUsageBytes
        LunaProcessName = $LunaProcessName
    }
    TestResults = @{
        StartTime = $startTime
        EndTime = Get-Date
        SampleCount = $samples.Count
        ThresholdsPassed = $false
    }
    SystemPerformance = @{
        CPU = @{
            Average = $avgCpu
            Maximum = $maxCpu
            Minimum = $minCpu
            ThresholdExceeded = $avgCpu -gt $MaxAverageCpu
        }
        Memory = @{
            AveragePercent = $avgMemory
            MaximumPercent = $maxMemory
            MinimumPercent = $minMemory
        }
        DiskIO = @{
            AverageReadMBps = $avgDiskRead
            AverageWriteMBps = $avgDiskWrite
        }
    }
    LunaProcess = if ($lunaWithMetrics.Count -gt 0) {
        @{
            AverageMemoryMB = $avgLunaMemoryMB
            PeakMemoryMB = $maxLunaMemoryMB
            SamplesWithData = $lunaWithMetrics.Count
            ProcessFound = $true
        }
    } else {
        @{
            ProcessFound = $false
        }
    }
    RawSamples = $samples
}

# Final threshold check
$finalThresholdsPassed = Test-PerformanceThresholds -Samples $samples
$report.TestResults.ThresholdsPassed = $finalThresholdsPassed

# Save report
try {
    $report | ConvertTo-Json -Depth 10 | Out-File -FilePath $OutputFile -Encoding UTF8
    Write-Host "`n💾 Performance report saved to: $OutputFile" -ForegroundColor Green
} catch {
    Write-Warning "⚠️ Could not save report to file: $_"
}

# Final result
Write-Host "`n🏁 Performance Test Result:" -ForegroundColor Cyan
if ($finalThresholdsPassed) {
    Write-Host "✅ PASS - All performance thresholds met" -ForegroundColor Green
    Write-Host "   Average CPU: $avgCpu% (≤ $MaxAverageCpu%)"
    Write-Host "   Memory usage within limits"
    exit 0
} else {
    Write-Host "❌ FAIL - Performance thresholds exceeded" -ForegroundColor Red
    Write-Host "   Average CPU: $avgCpu% (threshold: $MaxAverageCpu%)"
    if ($avgCpu -gt $MaxAverageCpu) {
        Write-Host "   CPU usage exceeded threshold by $([math]::Round($avgCpu - $MaxAverageCpu, 2))%"
    }
    exit 1
}