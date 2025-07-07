/*!
 * Luna Build Script - Prepares the portable executable
 */

use std::env;
use std::path::PathBuf;

fn main() {
    // Add Windows resource file for executable metadata
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icons/luna.ico"); // Will use a default if not found
        res.set("FileDescription", "Luna Visual AI Assistant");
        res.set("ProductName", "Luna Visual AI");
        res.set("CompanyName", "Luna Team");
        res.set("LegalCopyright", "© 2024 Luna Team");
        res.set("FileVersion", "1.0.0");
        res.set("ProductVersion", "1.0.0");
        res.set("InternalName", "luna");
        res.set("OriginalFilename", "luna.exe");
        
        // Ignore errors if icon file doesn't exist during development
        if let Err(e) = res.compile() {
            println!("cargo:warning=Could not compile Windows resources: {}", e);
        }
    }
    
    // Generate build information
    println!("cargo:rustc-env=BUILD_TARGET={}", env::var("TARGET").unwrap_or_default());
    println!("cargo:rustc-env=BUILD_PROFILE={}", env::var("PROFILE").unwrap_or_default());
    
    // Set build timestamp
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", chrono::Utc::now().to_rfc3339());
    
    // Link Windows libraries
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=gdi32");
        println!("cargo:rustc-link-lib=kernel32");
        println!("cargo:rustc-link-lib=psapi");
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=shell32");
        println!("cargo:rustc-link-lib=ole32");
    }
    
    // Create output directories
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Create models directory structure
    let models_dir = out_dir.join("models");
    std::fs::create_dir_all(&models_dir).unwrap_or_default();
    
    // Create assets directory structure
    let assets_dir = out_dir.join("assets");
    std::fs::create_dir_all(assets_dir.join("icons")).unwrap_or_default();
    std::fs::create_dir_all(assets_dir.join("sounds")).unwrap_or_default();
    
    // In a real implementation, we would:
    // 1. Download or embed AI model files
    // 2. Compress and embed assets
    // 3. Generate embedded resource files
    // 4. Set up code signing preparation
    
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/");
    
    // Output build information
    println!("🔨 Luna build script completed");
    println!("   Target: {}", env::var("TARGET").unwrap_or("unknown".to_string()));
    println!("   Profile: {}", env::var("PROFILE").unwrap_or("unknown".to_string()));
    println!("   Out dir: {}", out_dir.display());
}