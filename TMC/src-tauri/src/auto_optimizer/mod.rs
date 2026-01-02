/// Automatic memory optimization module.
///
/// This module provides scheduled memory optimization functionality,
/// allowing the application to automatically clean memory at configured
/// intervals to maintain system performance.
pub mod scheduler;

pub use scheduler::start_auto_optimizer;
