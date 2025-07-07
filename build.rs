/*!
 * Luna Visual AI Build Script
 * 
 * Handles build-time configuration for Windows deployment:
 * - Windows resource embedding (icons, version info)
 * - Code signing preparation
 * - Feature flag configuration
 * - Conditional compilation setup
 */

use std::env;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Windows-specific build configuration
    #[cfg(target_os = "windows")]
    {
        configure_windows_build();
    }

    // Feature flag configuration
    configure_features();
    
    // Version information
    configure_version_info();
    
    // Model path configuration
    configure_model_paths();
}

#[cfg(target_os = "windows")]
fn configure_windows_build() {
    // Embed Windows resources (icon, version info, manifest)
    let mut res = winres::WindowsResource::new();
    
    // Set application icon
    if Path::new("assets/icons/luna.ico").exists() {
        res.set_icon("assets/icons/luna.ico");
        println!("cargo:rustc-env=LUNA_ICON_PATH=assets/icons/luna.ico");
    } else {
        println!("cargo:warning=Application icon not found at assets/icons/luna.ico");
    }
    
    // Set version information
    res.set("ProductName", "Luna Visual AI");
    res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
    res.set("FileDescription", "AI-Powered Computer Vision Assistant");
    res.set("FileVersion", env!("CARGO_PKG_VERSION"));
    res.set("CompanyName", "Luna AI Team");
    res.set("LegalCopyright", "Copyright © 2024 Luna AI Team");
    res.set("OriginalFilename", "luna.exe");
    
    // Embed application manifest for proper Windows integration
    if Path::new("assets/luna.manifest").exists() {
        res.set_manifest_file("assets/luna.manifest");
    } else {
        // Use default manifest
        res.set_manifest(r#"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    name="Luna.Visual.AI"
    version="1.0.0.0"
    type="win32"/>
  <description>Luna Visual AI - Computer Vision Assistant</description>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"/>
    </dependentAssembly>
  </dependency>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10/11 -->
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
</assembly>
"#);
    }
    
    // Compile resources
    if let Err(e) = res.compile() {
        println!("cargo:warning=Failed to compile Windows resources: {}", e);
    } else {
        println!("cargo:rustc-env=LUNA_WINDOWS_RESOURCES=embedded");
    }
    
    // Link Windows libraries
    println!("cargo:rustc-link-lib=user32");
    println!("cargo:rustc-link-lib=gdi32");
    println!("cargo:rustc-link-lib=shell32");
    println!("cargo:rustc-link-lib=ole32");
    println!("cargo:rustc-link-lib=oleaut32");
    println!("cargo:rustc-link-lib=advapi32");
    println!("cargo:rustc-link-lib=winmm");
    
    // Enable Windows-specific features
    println!("cargo:rustc-cfg=windows_platform");
}

fn configure_features() {
    // Configure GPU features based on available libraries
    if cfg!(feature = "cuda") {
        if let Ok(cuda_path) = env::var("CUDA_PATH") {
            println!("cargo:rustc-link-search=native={}/lib/x64", cuda_path);
            println!("cargo:rustc-link-lib=cuda");
            println!("cargo:rustc-link-lib=cudart");
            println!("cargo:rustc-cfg=cuda_available");
            println!("cargo:rustc-env=LUNA_CUDA_ENABLED=1");
        } else {
            println!("cargo:warning=CUDA feature enabled but CUDA_PATH not found");
            println!("cargo:rustc-env=LUNA_CUDA_ENABLED=0");
        }
    } else {
        println!("cargo:rustc-env=LUNA_CUDA_ENABLED=0");
    }
    
    // Configure voice command features
    if cfg!(feature = "voice-commands") {
        println!("cargo:rustc-cfg=voice_commands_enabled");
        println!("cargo:rustc-env=LUNA_VOICE_ENABLED=1");
    } else {
        println!("cargo:rustc-env=LUNA_VOICE_ENABLED=0");
    }
    
    // Configure visual overlay features
    if cfg!(feature = "visual-overlay") {
        println!("cargo:rustc-cfg=visual_overlay_enabled");
        println!("cargo:rustc-env=LUNA_OVERLAY_ENABLED=1");
    } else {
        println!("cargo:rustc-env=LUNA_OVERLAY_ENABLED=0");
    }
    
    // Debug mode configuration
    if cfg!(feature = "debug-mode") {
        println!("cargo:rustc-cfg=debug_mode_enabled");
        println!("cargo:rustc-env=LUNA_DEBUG_MODE=1");
    } else {
        println!("cargo:rustc-env=LUNA_DEBUG_MODE=0");
    }
    
    // Memory profiling configuration
    if cfg!(feature = "memory-profiling") {
        println!("cargo:rustc-cfg=memory_profiling_enabled");
        println!("cargo:rustc-env=LUNA_MEMORY_PROFILING=1");
    } else {
        println!("cargo:rustc-env=LUNA_MEMORY_PROFILING=0");
    }
}

fn configure_version_info() {
    // Set version environment variables for runtime access
    println!("cargo:rustc-env=LUNA_VERSION={}", env!("CARGO_PKG_VERSION"));
    println!("cargo:rustc-env=LUNA_NAME={}", env!("CARGO_PKG_NAME"));
    println!("cargo:rustc-env=LUNA_DESCRIPTION={}", env!("CARGO_PKG_DESCRIPTION"));
    println!("cargo:rustc-env=LUNA_AUTHORS={}", env!("CARGO_PKG_AUTHORS"));
    println!("cargo:rustc-env=LUNA_REPOSITORY={}", env!("CARGO_PKG_REPOSITORY"));
    
    // Build timestamp
    let build_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("cargo:rustc-env=LUNA_BUILD_TIMESTAMP={}", build_time);
    
    // Build profile
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=LUNA_BUILD_PROFILE={}", profile);
    
    // Git information (if available)
    if let Ok(output) = std::process::Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
    {
        if output.status.success() {
            let git_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=LUNA_GIT_HASH={}", git_hash);
        }
    }
    
    if let Ok(output) = std::process::Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
    {
        if output.status.success() {
            let git_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=LUNA_GIT_BRANCH={}", git_branch);
        }
    }
}

fn configure_model_paths() {
    // Set default model cache directory
    let default_model_dir = if cfg!(windows) {
        r"C:\Users\%USERNAME%\AppData\Local\Luna Visual AI\models"
    } else {
        "~/.cache/luna-visual-ai/models"
    };
    
    println!("cargo:rustc-env=LUNA_DEFAULT_MODEL_DIR={}", default_model_dir);
    
    // Model configuration
    let models = [
        ("FLORENCE2", "florence-2-base"),
        ("CLIP", "clip-vit-base-patch32"),
        ("TROCR", "trocr-base-printed"),
        ("SAM", "sam-vit-base"),
    ];
    
    for (model_name, model_path) in &models {
        println!("cargo:rustc-env=LUNA_MODEL_{}={}", model_name, model_path);
    }
    
    // Model URLs for downloading
    println!("cargo:rustc-env=LUNA_MODEL_BASE_URL=https://huggingface.co");
    
    // Check if models directory exists and warn if empty
    let models_dir = env::var("LUNA_MODELS_DIR")
        .unwrap_or_else(|_| "./models".to_string());
    
    if !Path::new(&models_dir).exists() {
        println!("cargo:warning=Models directory not found: {}. Models will be downloaded on first run.", models_dir);
    }
}

// Additional build utilities
fn print_build_info() {
    println!("cargo:warning=Building Luna Visual AI");
    println!("cargo:warning=Version: {}", env!("CARGO_PKG_VERSION"));
    println!("cargo:warning=Target: {}", env::var("TARGET").unwrap_or_else(|_| "unknown".to_string()));
    println!("cargo:warning=Profile: {}", env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string()));
    
    #[cfg(feature = "cuda")]
    println!("cargo:warning=CUDA support: enabled");
    
    #[cfg(not(feature = "cuda"))]
    println!("cargo:warning=CUDA support: disabled");
    
    #[cfg(feature = "voice-commands")]
    println!("cargo:warning=Voice commands: enabled");
    
    #[cfg(feature = "visual-overlay")]
    println!("cargo:warning=Visual overlay: enabled");
}