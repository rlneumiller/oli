use anyhow::Result;
use oli_tui::app::App;

fn main() -> Result<()> {
    // Create a new OLI instance
    let app = App::new();
    
    // Print some basic info about the app
    println!("OLI TUI Demo");
    println!("Available models:");
    
    for (idx, model) in app.available_models.iter().enumerate() {
        println!("{}: {} ({}GB) - {}", 
            idx + 1, 
            model.name, 
            model.size_gb, 
            model.description
        );
    }
    
    println!("\nTo use the full TUI interface, run the application with: cargo run");
    
    Ok(())
}