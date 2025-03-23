// Special command definitions and command handling logic

#[derive(Debug, Clone)]
pub struct SpecialCommand {
    pub name: String,
    pub description: String,
}

impl SpecialCommand {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
        }
    }
}

// List of available special commands
pub fn get_available_commands() -> Vec<SpecialCommand> {
    vec![
        SpecialCommand::new("/help", "Show help and available commands"),
        SpecialCommand::new("/clear", "Clear conversation history and free up context"),
        SpecialCommand::new("/debug", "Toggle debug messages visibility"),
        SpecialCommand::new("/steps", "Toggle showing intermediate tool steps"),
        SpecialCommand::new("/summarize", "Manually summarize conversation history"),
        SpecialCommand::new("/exit", "Exit the TUI"),
    ]
}

pub trait CommandHandler {
    fn check_command_mode(&mut self);
    fn filtered_commands(&self) -> Vec<SpecialCommand>;
    fn select_next_command(&mut self);
    fn select_prev_command(&mut self);
    fn execute_command(&mut self) -> bool;
}
