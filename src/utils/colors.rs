//! Terminal colors and styling for CLI output.

use colored::Colorize;

/// Message type for different levels of output
#[derive(Debug, Clone, Copy)]
pub enum MessageType {
    Success,
    Error,
    Warning,
    Info,
    Hint,
}

impl MessageType {
    /// Apply color to a message based on its type
    pub fn colorize(&self, message: &str) -> String {
        match self {
            MessageType::Success => message.green().to_string(),
            MessageType::Error => message.red().to_string(),
            MessageType::Warning => message.yellow().to_string(),
            MessageType::Info => message.blue().to_string(),
            MessageType::Hint => message.dimmed().to_string(),
        }
    }

    /// Get the prefix/emoji for this message type
    pub fn prefix(&self) -> &str {
        match self {
            MessageType::Success => "✓",
            MessageType::Error => "✗",
            MessageType::Warning => "⚠",
            MessageType::Info => "ℹ",
            MessageType::Hint => "💡",
        }
    }

    /// Format a message with prefix and color
    pub fn format(&self, message: &str) -> String {
        format!("{} {}", self.prefix(), self.colorize(message))
    }
}

/// Print a success message (green)
pub fn success(message: &str) {
    println!("{}", MessageType::Success.format(message));
}

/// Print an info message (blue)
pub fn info(message: &str) {
    println!("{}", MessageType::Info.format(message));
}

/// Print a hint message (dimmed)
pub fn hint(message: &str) {
    println!("{}", MessageType::Hint.format(message));
}

/// Print a header/title (bold, cyan)
pub fn header(message: &str) {
    println!("\n{}", message.bold().cyan());
}
