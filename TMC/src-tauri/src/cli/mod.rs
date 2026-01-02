/// Command-line interface module.
///
/// This module provides console-mode functionality for the application,
/// allowing operation without a graphical user interface. It includes
/// argument parsing and command execution for headless environments.
pub mod parser;

pub use parser::run_console_mode;
