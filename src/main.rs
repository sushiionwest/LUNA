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

/// Main Luna Visual AI Application - Crystal Clear User Interface
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
    /// Whether this is the first launch (show tutorial)
    first_launch: bool,
    /// Whether to show the help panel
    show_help: bool,
    /// Auto-complete suggestions
    suggestions: Vec<String>,
    /// Whether Luna is in countdown mode
    countdown: Option<u8>,
    /// Current analysis results for preview
    analysis_preview: Option<String>,
}

impl LunaApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Setup custom fonts and styling for beautiful UI
        Self::setup_custom_style(&cc.egui_ctx);
        
        let mut app = Self::default();
        app.status = "🌙 Luna Visual AI Ready!".to_string();
        app.first_launch = true; // Show welcome tutorial
        
        // Pre-load common command suggestions
        app.suggestions = vec![
            "Close all browser tabs".to_string(),
            "Click the Save button".to_string(),
            "Open Control Panel".to_string(),
            "Take a screenshot".to_string(),
            "Type 'Hello World'".to_string(),
            "Press Ctrl+C".to_string(),
            "Scroll down".to_string(),
            "Find and click Submit".to_string(),
        ];
        
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
        
        // Luna Visual AI Brand Theme - Modern Dark with Blue Accents
        style.visuals.dark_mode = true;
        style.visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 220));
        style.visuals.window_fill = egui::Color32::from_rgb(25, 25, 35);
        style.visuals.panel_fill = egui::Color32::from_rgb(30, 30, 40);
        style.visuals.faint_bg_color = egui::Color32::from_rgb(40, 40, 50);
        
        // Luna signature colors
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(100, 149, 237); // Cornflower blue
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(50, 50, 60);
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(70, 70, 80);
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(100, 149, 237);
        
        // Rounded corners for modern appearance
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(8.0);
        
        ctx.set_style(style);
    }

    async fn execute_command(&mut self, command: &str) -> Result<()> {
        self.is_processing = true;
        self.status = format!("🔍 Processing: {}", command);
        
        // Add to history for reuse
        if !command.is_empty() && !self.command_history.contains(&command.to_string()) {
            self.command_history.push(command.to_string());
            if self.command_history.len() > 10 {
                self.command_history.remove(0); // Keep only last 10
            }
        }

        // Get core reference
        let core_guard = self.core.lock().await;
        let core = match core_guard.as_ref() {
            Some(core) => core,
            None => {
                self.status = "⏳ Luna core not ready yet...".to_string();
                self.is_processing = false;
                return Ok(());
            }
        };

        // Execute the command with full safety checks
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
        ctx.send_viewport_cmd(egui::ViewportCommand::Title("🌙 Luna Visual AI".to_string()));
        
        // Main application panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // First launch welcome tutorial
            if self.first_launch {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    
                    // Welcome tutorial box
                    ui.group(|ui| {
                        ui.set_min_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            ui.heading("🎉 Welcome to Luna Visual AI!");
                            ui.label("Your AI assistant that sees and clicks for you");
                            ui.add_space(10.0);
                            
                            ui.horizontal(|ui| {
                                ui.label("🎯 Try clicking:");
                                if ui.small_button("'Close all browser tabs'").clicked() {
                                    self.command_input = "Close all browser tabs".to_string();
                                }
                                if ui.small_button("'Click the Save button'").clicked() {
                                    self.command_input = "Click the Save button".to_string();
                                }
                            });
                            
                            ui.add_space(5.0);
                            if ui.button("🚀 Got it! Let's start").clicked() {
                                self.first_launch = false;
                            }
                        });
                    });
                    
                    ui.add_space(20.0);
                });
            }
            
            // Main header with Luna branding
            ui.vertical_centered(|ui| {
                ui.add_space(if self.first_launch { 10.0 } else { 20.0 });
                
                // Luna logo and title
                ui.heading("🌙 Luna Visual AI");
                ui.label("Your One-Click Computer Assistant");
                ui.add_space(10.0);
                
                // Enhanced status indicator with visual feedback
                let status_color = if self.is_processing {
                    egui::Color32::from_rgb(255, 165, 0) // Orange for processing
                } else if self.status.contains("✅") {
                    egui::Color32::from_rgb(50, 205, 50) // Green for success
                } else if self.status.contains("❌") {
                    egui::Color32::from_rgb(255, 69, 0) // Red for error
                } else {
                    egui::Color32::from_rgb(100, 149, 237) // Blue for ready
                };
                
                // Show processing spinner when active
                if self.is_processing {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.colored_label(status_color, &self.status);
                    });
                } else {
                    ui.colored_label(status_color, &self.status);
                }
                
                ui.add_space(20.0);
            });

            // Command input section - the heart of the interface
            ui.group(|ui| {
                ui.set_min_height(120.0);
                ui.vertical(|ui| {
                    ui.label("💬 Tell Luna what to do:");
                    
                    // Main command input box - large and obvious
                    let response = ui.add_sized(
                        [ui.available_width(), 40.0],
                        egui::TextEdit::singleline(&mut self.command_input)
                            .hint_text("e.g., 'Close all browser tabs' or 'Click the Save button'")
                            .font(egui::TextStyle::Body)
                    );
                    
                    // Smart auto-complete suggestions
                    if !self.command_input.is_empty() {
                        let matching_suggestions: Vec<_> = self.suggestions.iter()
                            .filter(|s| s.to_lowercase().contains(&self.command_input.to_lowercase()))
                            .take(3)
                            .collect();
                        
                        if !matching_suggestions.is_empty() {
                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                ui.label("💡 Suggestions:");
                                for suggestion in matching_suggestions {
                                    if ui.small_button(suggestion).clicked() {
                                        self.command_input = suggestion.clone();
                                    }
                                }
                            });
                        }
                    }
                    
                    // Handle Enter key for quick execution
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let command = self.command_input.clone();
                        if !command.is_empty() {
                            self.command_input.clear();
                            
                            // Execute command asynchronously
                            let rt = tokio::runtime::Handle::current();
                            let mut app_clone = self.clone();
                            rt.spawn(async move {
                                let _ = app_clone.execute_command(&command).await;
                            });
                        }
                    }
                    
                    ui.add_space(10.0);
                    
                    // Control buttons - clear and prominent
                    ui.horizontal(|ui| {
                        // Execute button - made large and obvious
                        let execute_btn = ui.add_enabled(
                            !self.is_processing && !self.command_input.is_empty(),
                            egui::Button::new("🚀 Execute").min_size(egui::vec2(100.0, 35.0))
                        );
                        
                        if execute_btn.clicked() {
                            let command = self.command_input.clone();
                            self.command_input.clear();
                            
                            // Execute command asynchronously
                            let rt = tokio::runtime::Handle::current();
                            let mut app_clone = self.clone();
                            rt.spawn(async move {
                                let _ = app_clone.execute_command(&command).await;
                            });
                        }
                        
                        // Voice toggle with clear feedback
                        let voice_btn = ui.add(egui::Button::new(if self.voice_enabled { "🔊 Voice On" } else { "🔇 Voice Off" }));
                        if voice_btn.clicked() {
                            self.voice_enabled = !self.voice_enabled;
                            self.status = if self.voice_enabled { 
                                "🎤 Voice input enabled - say your command!".to_string() 
                            } else { 
                                "🔇 Voice input disabled".to_string() 
                            };
                        }
                        
                        // Emergency stop - always visible and prominent
                        let stop_btn = ui.add(egui::Button::new("🛑 STOP").fill(egui::Color32::from_rgb(139, 0, 0)));
                        if stop_btn.clicked() {
                            self.is_processing = false;
                            self.countdown = None;
                            self.status = "🛑 Stopped by user".to_string();
                        }
                        
                        // Help button
                        if ui.add(egui::Button::new("❓ Help")).clicked() {
                            self.show_help = !self.show_help;
                        }
                    });
                });
            });
            
            ui.add_space(10.0);
            
            // Command history for easy reuse
            if !self.command_history.is_empty() {
                ui.group(|ui| {
                    ui.label("📜 Recent Commands:");
                    egui::ScrollArea::vertical()
                        .max_height(100.0)
                        .show(ui, |ui| {
                            for cmd in self.command_history.iter().rev() {
                                let response = ui.selectable_label(false, cmd);
                                if response.clicked() {
                                    self.command_input = cmd.clone();
                                }
                            }
                        });
                });
                
                ui.add_space(10.0);
            }
            
            // Comprehensive help panel
            if self.show_help {
                ui.group(|ui| {
                    ui.set_min_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.heading("🎓 How to Use Luna");
                        ui.separator();
                        
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label("📝 Text Commands:");
                                ui.small("• Type what you want Luna to do");
                                ui.small("• Use natural language");
                                ui.small("• Press Enter or click Execute");
                            });
                            
                            ui.separator();
                            
                            ui.vertical(|ui| {
                                ui.label("🎤 Voice Commands:");
                                ui.small("• Click 'Voice On' to enable");
                                ui.small("• Speak your command clearly");
                                ui.small("• Luna will confirm what it heard");
                            });
                        });
                        
                        ui.separator();
                        
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label("🛡️ Safety Features:");
                                ui.small("• 3-second countdown before actions");
                                ui.small("• Visual preview of what Luna will click");
                                ui.small("• Press ESC or Stop to cancel");
                            });
                            
                            ui.separator();
                            
                            ui.vertical(|ui| {
                                ui.label("⌨️ Keyboard Shortcuts:");
                                ui.small("• Enter: Execute command");
                                ui.small("• Ctrl+L: Focus command input");
                                ui.small("• Escape: Cancel current action");
                            });
                        });
                        
                        ui.separator();
                        
                        if ui.button("❌ Close Help").clicked() {
                            self.show_help = false;
                        }
                    });
                });
                
                ui.add_space(10.0);
            }
            
            // Feature showcase organized by categories
            ui.group(|ui| {
                ui.label("🎯 What Luna Can Do - Click to Try:");
                
                ui.horizontal_wrapped(|ui| {
                    let categories = [
                        ("🌐 Browser", vec!["Close all browser tabs", "Refresh page", "Open new tab"]),
                        ("💾 Files", vec!["Click the Save button", "Open Control Panel", "Screenshot this window"]),
                        ("⌨️ Input", vec!["Type 'Hello World'", "Press Ctrl+C", "Scroll down"]),
                        ("🔍 Actions", vec!["Find and click Submit", "Click OK button", "Select all text"]),
                    ];
                    
                    for (category, examples) in &categories {
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.small(*category);
                                for example in examples {
                                    if ui.small_button(format!("💡 {}", example)).clicked() {
                                        self.command_input = example.to_string();
                                    }
                                }
                            });
                        });
                    }
                });
            });
            
            // Debug info and footer
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
                    ui.small(format!("First Launch: {}", self.first_launch));
                });
            }
        });
        
        // Request repaint if processing to keep UI responsive
        if self.is_processing {
            ctx.request_repaint();
        }
    }
}

// Implement Clone for async operations
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
            first_launch: false,
            show_help: false,
            suggestions: self.suggestions.clone(),
            countdown: self.countdown,
            analysis_preview: self.analysis_preview.clone(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize comprehensive logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("🌙 Luna Visual AI starting...");

    // Native window configuration for optimal user experience
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_min_inner_size([700.0, 500.0])
            .with_icon(load_icon())
            .with_resizable(true)
            .with_title("Luna Visual AI"),
        centered: true,
        ..Default::default()
    };

    // Launch the Luna Visual AI application
    eframe::run_native(
        "Luna Visual AI",
        options,
        Box::new(|cc| Box::new(LunaApp::new(cc))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to start Luna GUI: {}", e))?;

    Ok(())
}

fn load_icon() -> egui::IconData {
    // Luna icon data (replace with actual icon)
    egui::IconData {
        rgba: vec![0; 32 * 32 * 4], // Placeholder - replace with actual Luna icon
        width: 32,
        height: 32,
    }
}