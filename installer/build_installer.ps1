# Luna Smart Installer Build Script
# Creates both the bootstrap installer and full MSI package

param(
    [switch]$Release = $false,
    [switch]$Sign = $false,
    [string]$SigningCert = "",
    [switch]$Clean = $false
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# Configuration
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$BuildDir = Join-Path $ProjectRoot "installer\build"
$OutputDir = Join-Path $ProjectRoot "installer\output"
$BootstrapDir = Join-Path $ProjectRoot "installer\bootstrap"
$AssetsDir = Join-Path $ProjectRoot "assets"

# Build configuration
$Configuration = if ($Release) { "Release" } else { "Debug" }
$Platform = "x64"

Write-Host "🔨 Luna Smart Installer Build Script" -ForegroundColor Cyan
Write-Host "Configuration: $Configuration" -ForegroundColor Yellow
Write-Host "Platform: $Platform" -ForegroundColor Yellow

# Clean previous builds if requested
if ($Clean) {
    Write-Host "🧹 Cleaning previous builds..." -ForegroundColor Yellow
    if (Test-Path $BuildDir) { Remove-Item -Recurse -Force $BuildDir }
    if (Test-Path $OutputDir) { Remove-Item -Recurse -Force $OutputDir }
}

# Create directories
New-Item -ItemType Directory -Force -Path $BuildDir | Out-Null
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

# Step 1: Build Luna main executable
Write-Host "🔨 Building Luna main executable..." -ForegroundColor Green

Push-Location $ProjectRoot
try {
    # Build with Cargo
    if ($Release) {
        cargo build --release --bin luna
    } else {
        cargo build --bin luna
    }
    
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build failed"
    }
    
    Write-Host "✅ Luna executable built successfully" -ForegroundColor Green
} finally {
    Pop-Location
}

# Step 2: Build Luna Updater
Write-Host "🔄 Building Luna Updater..." -ForegroundColor Green

Push-Location $ProjectRoot
try {
    if ($Release) {
        cargo build --release --bin luna-updater
    } else {
        cargo build --bin luna-updater
    }
    
    if ($LASTEXITCODE -ne 0) {
        throw "Luna Updater build failed"
    }
    
    Write-Host "✅ Luna Updater built successfully" -ForegroundColor Green
} finally {
    Pop-Location
}

# Step 3: Prepare dependencies
Write-Host "📦 Preparing dependencies..." -ForegroundColor Green

$DepsDir = Join-Path $BuildDir "deps"
New-Item -ItemType Directory -Force -Path $DepsDir | Out-Null

# Download Visual C++ Redistributable
$VCRedistUrl = "https://aka.ms/vs/17/release/vc_redist.x64.exe"
$VCRedistPath = Join-Path $DepsDir "vcredist_x64.exe"

if (-not (Test-Path $VCRedistPath)) {
    Write-Host "📥 Downloading Visual C++ Redistributable..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri $VCRedistUrl -OutFile $VCRedistPath -UseBasicParsing
    Write-Host "✅ VC++ Redistributable downloaded" -ForegroundColor Green
}

# Step 4: Create default configuration
Write-Host "⚙️ Creating default configuration..." -ForegroundColor Green

$ConfigDir = Join-Path $BuildDir "config"
New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null

$DefaultConfig = @"
# Luna Visual AI Default Configuration

[ai]
precision = "f32"
use_gpu = false
max_inference_time_ms = 5000
confidence_threshold = 0.7

[safety]
enabled = true
confirmation_timeout_seconds = 3
max_actions_per_command = 10

[ui]
show_overlay = true
overlay_opacity = 0.8
highlight_color = [100, 149, 237]
enable_sounds = true

[performance]
max_memory_mb = 512
worker_threads = 2
screenshot_cache_size = 5
"@

$DefaultConfig | Out-File -FilePath (Join-Path $ConfigDir "default.toml") -Encoding UTF8

# Step 5: Build WiX MSI Installer
Write-Host "📦 Building WiX MSI installer..." -ForegroundColor Green

# Check if WiX is installed
$WixPath = "${env:ProgramFiles(x86)}\WiX Toolset v3.11\bin"
if (-not (Test-Path $WixPath)) {
    Write-Error "WiX Toolset not found. Please install WiX Toolset v3.11 or later."
    exit 1
}

$env:PATH = "$WixPath;$env:PATH"

# Compile WiX source
$WxsFile = Join-Path $ProjectRoot "installer\luna_installer.wxs"
$WixObjFile = Join-Path $BuildDir "luna_installer.wixobj"
$MsiFile = Join-Path $OutputDir "Luna-Setup.msi"

# Candle (compile)
Write-Host "🕯️ Compiling WiX source..." -ForegroundColor Yellow
& candle.exe -arch x64 -out $WixObjFile $WxsFile
if ($LASTEXITCODE -ne 0) {
    throw "WiX candle compilation failed"
}

# Light (link)
Write-Host "💡 Linking MSI package..." -ForegroundColor Yellow
& light.exe -ext WixUIExtension -ext WixUtilExtension -out $MsiFile $WixObjFile
if ($LASTEXITCODE -ne 0) {
    throw "WiX light linking failed"
}

Write-Host "✅ MSI installer created: $MsiFile" -ForegroundColor Green

# Step 6: Build Bootstrap Installer
Write-Host "🚀 Building bootstrap installer..." -ForegroundColor Green

# Check if Visual Studio Build Tools are available
$VSWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $VSWhere) {
    $VSPath = & $VSWhere -latest -property installationPath
    $MSBuild = Join-Path $VSPath "MSBuild\Current\Bin\MSBuild.exe"
} else {
    # Fallback to older Visual Studio versions
    $MSBuild = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2019\BuildTools\MSBuild\Current\Bin\MSBuild.exe"
    if (-not (Test-Path $MSBuild)) {
        $MSBuild = "${env:ProgramFiles(x86)}\MSBuild\14.0\Bin\MSBuild.exe"
    }
}

if (-not (Test-Path $MSBuild)) {
    Write-Error "MSBuild not found. Please install Visual Studio Build Tools."
    exit 1
}

# Create Visual Studio project for bootstrap
$BootstrapProject = @"
<?xml version="1.0" encoding="utf-8"?>
<Project DefaultTargets="Build" xmlns="http://schemas.microsoft.com/developer/msbuild/2003">
  <PropertyGroup Label="Globals">
    <ProjectGuid>{12345678-1234-5678-9ABC-123456789012}</ProjectGuid>
    <Keyword>Win32Proj</Keyword>
    <RootNamespace>LunaBootstrap</RootNamespace>
    <WindowsTargetPlatformVersion>10.0</WindowsTargetPlatformVersion>
  </PropertyGroup>
  <Import Project="`$(VCTargetsPath)\Microsoft.Cpp.Default.props" />
  <PropertyGroup Condition="'`$(Configuration)|`$(Platform)'=='Debug|x64'" Label="Configuration">
    <ConfigurationType>Application</ConfigurationType>
    <UseDebugLibraries>true</UseDebugLibraries>
    <PlatformToolset>v142</PlatformToolset>
    <CharacterSet>Unicode</CharacterSet>
  </PropertyGroup>
  <PropertyGroup Condition="'`$(Configuration)|`$(Platform)'=='Release|x64'" Label="Configuration">
    <ConfigurationType>Application</ConfigurationType>
    <UseDebugLibraries>false</UseDebugLibraries>
    <PlatformToolset>v142</PlatformToolset>
    <WholeProgramOptimization>true</WholeProgramOptimization>
    <CharacterSet>Unicode</CharacterSet>
  </PropertyGroup>
  <Import Project="`$(VCTargetsPath)\Microsoft.Cpp.props" />
  <PropertyGroup>
    <OutDir>`$(SolutionDir)output\</OutDir>
    <IntDir>`$(SolutionDir)build\bootstrap\</IntDir>
    <TargetName>LunaInstaller</TargetName>
  </PropertyGroup>
  <ItemDefinitionGroup Condition="'`$(Configuration)|`$(Platform)'=='Release|x64'">
    <ClCompile>
      <WarningLevel>Level3</WarningLevel>
      <Optimization>MaxSpeed</Optimization>
      <FunctionLevelLinking>true</FunctionLevelLinking>
      <IntrinsicFunctions>true</IntrinsicFunctions>
      <RuntimeLibrary>MultiThreaded</RuntimeLibrary>
    </ClCompile>
    <Link>
      <EnableCOMDATFolding>true</EnableCOMDATFolding>
      <OptimizeReferences>true</OptimizeReferences>
      <SubSystem>Windows</SubSystem>
      <AdditionalDependencies>wininet.lib;shell32.lib;comctl32.lib;shlwapi.lib;%(AdditionalDependencies)</AdditionalDependencies>
    </Link>
  </ItemDefinitionGroup>
  <ItemGroup>
    <ClCompile Include="bootstrap\luna_bootstrap.cpp" />
  </ItemGroup>
  <Import Project="`$(VCTargetsPath)\Microsoft.Cpp.targets" />
</Project>
"@

$ProjectFile = Join-Path $BuildDir "LunaBootstrap.vcxproj"
$BootstrapProject | Out-File -FilePath $ProjectFile -Encoding UTF8

# Build bootstrap installer
Write-Host "🔨 Compiling bootstrap installer..." -ForegroundColor Yellow
& $MSBuild $ProjectFile /p:Configuration=$Configuration /p:Platform=x64 /p:SolutionDir="$ProjectRoot\installer\"
if ($LASTEXITCODE -ne 0) {
    throw "Bootstrap installer build failed"
}

$BootstrapExe = Join-Path $OutputDir "LunaInstaller.exe"
Write-Host "✅ Bootstrap installer created: $BootstrapExe" -ForegroundColor Green

# Step 7: Code Signing (if requested)
if ($Sign -and $SigningCert) {
    Write-Host "🔏 Code signing binaries..." -ForegroundColor Green
    
    $SignTool = "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.0.19041.0\x64\signtool.exe"
    if (-not (Test-Path $SignTool)) {
        # Try to find signtool in different locations
        $SignTool = Get-ChildItem -Path "${env:ProgramFiles(x86)}\Windows Kits" -Recurse -Name "signtool.exe" | Select-Object -First 1
        if ($SignTool) {
            $SignTool = $SignTool.FullName
        }
    }
    
    if ($SignTool -and (Test-Path $SignTool)) {
        # Sign the MSI
        & $SignTool sign /f $SigningCert /t http://timestamp.digicert.com $MsiFile
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ MSI signed successfully" -ForegroundColor Green
        }
        
        # Sign the bootstrap
        & $SignTool sign /f $SigningCert /t http://timestamp.digicert.com $BootstrapExe
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ Bootstrap installer signed successfully" -ForegroundColor Green
        }
    } else {
        Write-Warning "SignTool not found. Skipping code signing."
    }
}

# Step 8: Create full offline installer
Write-Host "📦 Creating full offline installer..." -ForegroundColor Green

$OfflineDir = Join-Path $BuildDir "offline"
New-Item -ItemType Directory -Force -Path $OfflineDir | Out-Null

# Copy all components to offline directory
Copy-Item -Path $MsiFile -Destination $OfflineDir
Copy-Item -Path (Join-Path $DepsDir "vcredist_x64.exe") -Destination $OfflineDir

# Create offline installer script
$OfflineScript = @"
@echo off
echo 🌙 Luna Visual AI - Offline Installer
echo.

REM Install VC++ Redistributable if needed
echo Installing dependencies...
vcredist_x64.exe /quiet /norestart

REM Install Luna
echo Installing Luna Visual AI...
msiexec /i Luna-Setup.msi /quiet AUTOSTART=1

echo.
echo ✅ Luna Visual AI installed successfully!
echo You can now find Luna in your Start Menu.
pause
"@

$OfflineScript | Out-File -FilePath (Join-Path $OfflineDir "install.bat") -Encoding ASCII

# Create self-extracting archive
$SfxScript = @"
;!@Install@!UTF-8!
Title="Luna Visual AI Installer"
BeginPrompt="Install Luna Visual AI - AI-powered computer assistant?"
RunProgram="install.bat"
;!@InstallEnd@!
"@

$SfxFile = Join-Path $BuildDir "sfx_config.txt"
$SfxScript | Out-File -FilePath $SfxFile -Encoding UTF8

# Use 7-Zip to create self-extracting archive
$SevenZip = "${env:ProgramFiles}\7-Zip\7z.exe"
if (Test-Path $SevenZip) {
    $OfflineInstaller = Join-Path $OutputDir "Luna-Offline-Setup.exe"
    Push-Location $OfflineDir
    try {
        & $SevenZip a -sfx7z.sfx $OfflineInstaller *
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ Offline installer created: $OfflineInstaller" -ForegroundColor Green
        }
    } finally {
        Pop-Location
    }
} else {
    Write-Warning "7-Zip not found. Skipping offline installer creation."
}

# Step 9: Generate checksums
Write-Host "🔍 Generating checksums..." -ForegroundColor Green

$ChecksumFile = Join-Path $OutputDir "checksums.txt"
$Checksums = @()

Get-ChildItem -Path $OutputDir -Filter "*.exe" | ForEach-Object {
    $Hash = Get-FileHash -Path $_.FullName -Algorithm SHA256
    $Checksums += "$($Hash.Hash)  $($_.Name)"
}

Get-ChildItem -Path $OutputDir -Filter "*.msi" | ForEach-Object {
    $Hash = Get-FileHash -Path $_.FullName -Algorithm SHA256
    $Checksums += "$($Hash.Hash)  $($_.Name)"
}

$Checksums | Out-File -FilePath $ChecksumFile -Encoding UTF8

# Step 10: Build summary
Write-Host ""
Write-Host "🎉 Build completed successfully!" -ForegroundColor Green
Write-Host "📁 Output directory: $OutputDir" -ForegroundColor Yellow
Write-Host ""
Write-Host "📦 Created files:" -ForegroundColor Cyan

Get-ChildItem -Path $OutputDir | ForEach-Object {
    $Size = [math]::Round($_.Length / 1MB, 2)
    Write-Host "   $($_.Name) ($Size MB)" -ForegroundColor White
}

Write-Host ""
Write-Host "🚀 Ready for distribution!" -ForegroundColor Green
Write-Host "Users can download LunaInstaller.exe (5MB) for the 1-click experience" -ForegroundColor Yellow