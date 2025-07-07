/*!
 * Luna Visual AI - One-Click Computer Assistant
 * 
 * Just double-click luna.exe and start giving voice or text commands!
 * Luna sees your screen and clicks where you want it to click.
 */

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hide console in release

use anyhow::Result;
use eframe::egui;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error};

mod ai;
mod core;
mod input;
mod overlay;
mod utils;
mod vision;

use ai::AIVisionPipeline;
use core::{LunaCore, LunaEvent};
use overlay::VisualOverlay;
use vision::ScreenCapture;

/// Main Luna Visual AI Application
#[derive(Default)]
pub struct LunaApp {
    /// Core AI processing engine
    core: Arc<Mutex<Option<LunaCore>>>,
    /// Current user command input
    command_input: String,
    /// Current status message
    status: String,
    /// Whether Luna is actively processing
    is_processing: bool,
    /// Whether to show debug info
    show_debug: bool,
    /// Recent command history
    command_history: Vec<String>,
    /// Whether voice input is enabled
    voice_enabled: bool,
    /// Current screenshot for preview
    current_screenshot: Option<egui::TextureHandle>,
}

impl LunaApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Setup custom fonts and styling
        Self::setup_custom_style(&cc.egui_ctx);
        
        let mut app = Self::default();
        app.status = "Luna Visual AI Ready! 🤖".to_string();
        
        // Initialize Luna core in background
        let core_clone = app.core.clone();
        tokio::spawn(async move {
            match LunaCore::new().await {
                Ok(core) => {
                    *core_clone.lock().await = Some(core);
                    info!("Luna core initialized successfully");
                }
                Err(e) => {
                    error!("Failed to initialize Luna core: {}", e);
                }
            }
        });
        
        app
    }

    fn setup_custom_style(ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        
        // Modern dark theme with Luna branding
        style.visuals.dark_mode = true;
        style.visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 220));
        style.visuals.window_fill = egui::Color32::from_rgb(25, 25, 35);
        style.visuals.panel_fill = egui::Color32::from_rgb(30, 30, 40);
        style.visuals.faint_bg_color = egui::Color32::from_rgb(40, 40, 50);
        
        // Luna brand colors
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(100, 149, 237); // Cornflower blue
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(50, 50, 60);
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(70, 70, 80);
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(100, 149, 237);
        
        // Rounded corners for modern look
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(8.0);
        
        ctx.set_style(style);
    }

    async fn execute_command(&mut self, command: &str) -> Result<()> {
        self.is_processing = true;
        self.status = format!("Processing: {}", command);
        
        // Add to history
        if !command.is_empty() && !self.command_history.contains(&command.to_string()) {
            self.command_history.push(command.to_string());
            if self.command_history.len() > 10 {
                self.command_history.remove(0);
            }
        }

        // Get core reference
        let core_guard = self.core.lock().await;
        let core = match core_guard.as_ref() {
            Some(core) => core,
            None => {
                self.status = "Luna core not ready yet...".to_string();
                self.is_processing = false;
                return Ok(());
            }
        };

        // Execute the command
        match core.execute_command(command).await {
            Ok(_) => {
                self.status = "✅ Command completed successfully!".to_string();
            }
            Err(e) => {
                self.status = format!("❌ Error: {}", e);
                error!("Command execution failed: {}", e);
            }
        }
        
        self.is_processing = false;
        Ok(())
    }
}

impl eframe::App for LunaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set window title
        ctx.send_viewport_cmd(egui::ViewportCommand::Title("Luna Visual AI".to_string()));
        
        // Main panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // Header with Luna branding
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                
                // Luna logo and title
                ui.heading("🌙 Luna Visual AI");
                ui.label("Your One-Click Computer Assistant");
                ui.add_space(10.0);
                
                // Status indicator
                let status_color = if self.is_processing {
                    egui::Color32::from_rgb(255, 165, 0) // Orange for processing
                } else if self.status.contains("✅") {
                    egui::Color32::from_rgb(50, 205, 50) // Green for success
                } else if self.status.contains("❌") {
                    egui::Color32::from_rgb(255, 69, 0) // Red for error
                } else {
                    egui::Color32::from_rgb(100, 149, 237) // Blue for ready
                };
                
                ui.colored_label(status_color, &self.status);
                ui.add_space(20.0);
            });

            // Command input section
            ui.group(|ui| {
                ui.set_min_height(80.0);
                ui.vertical(|ui| {
                    ui.label("💬 Tell Luna what to do:");
                    
                    let response = ui.add_sized(
                        [ui.available_width(), 40.0],
                        egui::TextEdit::singleline(&mut self.command_input)
                            .hint_text("e.g., 'Close all browser tabs' or 'Click the Save button'")
                            .font(egui::TextStyle::Body)
                    );
                    
                    // Handle Enter key
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let command = self.command_input.clone();
                        if !command.is_empty() {
                            self.command_input.clear();
                            
                            // Execute command asynchronously
                            let rt = tokio::runtime::Handle::current();
                            let mut app_clone = self.clone(); // We'll need to implement Clone for this
                            rt.spawn(async move {
                                let _ = app_clone.execute_command(&command).await;
                            });
                        }
                    }
                    
                    ui.horizontal(|ui| {
                        // Execute button
                        let execute_btn = ui.add_enabled(
                            !self.is_processing && !self.command_input.is_empty(),
                            egui::Button::new("🚀 Execute")
                        );
                        
                        if execute_btn.clicked() {
                            let command = self.command_input.clone();
                            self.command_input.clear();
                            
                            // Execute command asynchronously
                            let rt = tokio::runtime::Handle::current();
                            let mut app_clone = self.clone(); // We'll implement Clone
                            rt.spawn(async move {
                                let _ = app_clone.execute_command(&command).await;
                            });
                        }
                        
                        // Voice toggle
                        if ui.add(egui::Button::new(if self.voice_enabled { "🔊 Voice On" } else { "🔇 Voice Off" })).clicked() {
                            self.voice_enabled = !self.voice_enabled;
                        }
                        
                        // Emergency stop
                        if ui.add(egui::Button::new("🛑 Stop")).clicked() {
                            self.is_processing = false;
                            self.status = "Stopped by user".to_string();
                        }
                    });
                });
            });
            
            ui.add_space(10.0);
            
            // Command history
            if !self.command_history.is_empty() {
                ui.group(|ui| {
                    ui.label("📜 Recent Commands:");
                    egui::ScrollArea::vertical()
                        .max_height(100.0)
                        .show(ui, |ui| {
                            for (i, cmd) in self.command_history.iter().rev().enumerate() {
                                let response = ui.selectable_label(false, cmd);
                                if response.clicked() {
                                    self.command_input = cmd.clone();
                                }
                            }
                        });
                });
            }
            
            ui.add_space(10.0);
            
            // Feature showcase
            ui.group(|ui| {
                ui.label("🎯 What Luna Can Do:");
                ui.horizontal_wrapped(|ui| {
                    let examples = [
                        "Close all browser tabs",
                        "Click the Save button",
                        "Open Control Panel",
                        "Find and click Submit",
                        "Screenshot this window",
                        "Type 'Hello World'",
                        "Press Ctrl+C",
                        "Scroll down",
                    ];
                    
                    for example in &examples {
                        if ui.small_button(format!("💡 {}", example)).clicked() {
                            self.command_input = example.to_string();
                        }
                    }
                });
            });
            
            // Debug info toggle
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if ui.small_button("🔧 Debug Info").clicked() {
                    self.show_debug = !self.show_debug;
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.hyperlink_to("📚 Documentation", "https://github.com/sushiionwest/LUNA");
                    ui.label("•");
                    ui.small("v1.0.0");
                });
            });
            
            if self.show_debug {
                ui.separator();
                ui.group(|ui| {
                    ui.label("🔍 Debug Information:");
                    ui.small(format!("Processing: {}", self.is_processing));
                    ui.small(format!("Voice Enabled: {}", self.voice_enabled));
                    ui.small(format!("Command History Length: {}", self.command_history.len()));
                });
            }
        });
        
        // Request repaint if processing
        if self.is_processing {
            ctx.request_repaint();
        }
    }
}

// We need to implement Clone for async operations
impl Clone for LunaApp {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            command_input: self.command_input.clone(),
            status: self.status.clone(),
            is_processing: self.is_processing,
            show_debug: self.show_debug,
            command_history: self.command_history.clone(),
            voice_enabled: self.voice_enabled,
            current_screenshot: None, // Can't clone TextureHandle
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("🌙 Luna Visual AI starting...");

    // Native window options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_icon(load_icon())
            .with_resizable(true)
            .with_title("Luna Visual AI"),
        centered: true,
        ..Default::default()
    };

    // Launch the native GUI application
    eframe::run_native(
        "Luna Visual AI",
        options,
        Box::new(|cc| Box::new(LunaApp::new(cc))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to start Luna GUI: {}", e))?;

    Ok(())
}

fn load_icon() -> egui::IconData {
    // Embedded Luna icon (you can replace this with actual icon data)
    egui::IconData {
        rgba: vec![0; 32 * 32 * 4], // Placeholder - replace with actual icon
        width: 32,
        height: 32,
    }
}