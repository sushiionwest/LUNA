// LUNA command-line entry point.
//
// Reads a natural-language command, runs it through the pipeline
// (safety check -> screen capture -> CV analysis -> action planning ->
// guarded execution) and reports what happened.
//
// Note: screen capture and input injection are currently placeholder
// stubs (see README), so this exercises the full pipeline against a
// synthetic screen and logs actions instead of performing them.

use std::io::{self, BufRead, Write};

use luna::{Luna, LunaConfig};

fn main() -> anyhow::Result<()> {
    let config = LunaConfig::default();
    config.apply_logging()?;

    let mut luna = Luna::new(config)?;

    println!("LUNA prototype ({})", env!("CARGO_PKG_VERSION"));
    println!("Commands:");
    println!("  analyze            - capture and analyze the screen");
    println!("  stats              - show processing statistics");
    println!("  quit               - exit");
    println!("  anything else      - processed as an automation command,");
    println!("                       e.g. 'click the save button'");
    println!();

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break; // EOF
        }
        let command = line.trim();

        match command {
            "" => continue,
            "quit" | "exit" => break,
            "analyze" => match luna.analyze_current_screen() {
                Ok(analysis) => {
                    println!(
                        "{} elements detected in {}ms (avg confidence {:.2})",
                        analysis.elements.len(),
                        analysis.processing_time_ms,
                        analysis.confidence
                    );
                    for element in &analysis.elements {
                        println!(
                            "  {} at ({}, {}) {}x{} confidence {:.2}",
                            element.element_type,
                            element.bounds.x,
                            element.bounds.y,
                            element.bounds.width,
                            element.bounds.height,
                            element.confidence
                        );
                    }
                }
                Err(e) => eprintln!("Analysis failed: {}", e),
            },
            "stats" => {
                let stats = luna.get_stats();
                println!(
                    "commands: {}, actions: {}, safety blocks: {}, avg time: {:.1}ms",
                    stats.commands_processed,
                    stats.actions_executed,
                    stats.safety_blocks,
                    stats.average_processing_time_ms
                );
            }
            _ => match luna.process_command(command) {
                Ok(actions) => println!("Executed {} action(s): {:?}", actions.len(), actions),
                Err(e) => eprintln!("Command failed: {}", e),
            },
        }
    }

    println!("Bye.");
    Ok(())
}
