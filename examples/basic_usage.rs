/*!
 * Basic Luna Usage Example
 * 
 * Shows how to initialize and use Luna Visual AI
 */

use luna::{init, LunaCore};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌙 Luna Visual AI - Basic Usage Example");
    
    // Initialize Luna
    println!("Initializing Luna...");
    let luna = init().await?;
    
    println!("✅ Luna initialized successfully!");
    
    // Check system compatibility
    let compat = luna::check_compatibility();
    println!("🔍 Compatibility check:");
    println!("  - Compatible: {}", compat.compatible);
    println!("  - Windows version: {}", compat.windows_version);
    
    if !compat.issues.is_empty() {
        println!("⚠️  Issues found:");
        for issue in &compat.issues {
            println!("    - {}", issue);
        }
    }
    
    if !compat.recommendations.is_empty() {
        println!("💡 Recommendations:");
        for rec in &compat.recommendations {
            println!("    - {}", rec);
        }
    }
    
    // Execute some basic commands
    println!("\n🚀 Executing example commands...");
    
    // Safe command that just waits
    println!("⏱️  Executing wait command...");
    match luna.execute_command("Wait 1 second").await {
        Ok(()) => println!("✅ Wait command completed"),
        Err(e) => println!("❌ Wait command failed: {}", e),
    }
    
    // Type some text (won't actually type since no input focus)
    println!("⌨️  Executing type command...");
    match luna.execute_command("Type 'Hello Luna!'").await {
        Ok(()) => println!("✅ Type command completed"),
        Err(e) => println!("❌ Type command failed: {}", e),
    }
    
    // Try to find a common UI element (will likely not find anything)
    println!("🔍 Searching for UI elements...");
    match luna.execute_command("Click the OK button").await {
        Ok(()) => println!("✅ Click command completed"),
        Err(e) => println!("ℹ️  Click command info: {}", e),
    }
    
    // Get system status
    let status = luna.get_status()?;
    println!("\n📊 Luna Status:");
    println!("  - AI Ready: {}", status.ai_ready);
    println!("  - Memory Usage: {} MB", status.memory_usage);
    println!("  - Safety Enabled: {}", status.safety_enabled);
    println!("  - Uptime: {} seconds", status.uptime);
    
    println!("\n🏁 Example completed successfully!");
    println!("You can now use Luna by running the main executable and giving it voice or text commands.");
    
    Ok(())
}