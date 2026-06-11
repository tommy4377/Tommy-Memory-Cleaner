/// Security utilities for input validation and sanitization

/// Sanitize a string by removing potentially dangerous characters
pub fn sanitize_string(input: &str, max_length: usize) -> String {
    // Remove null bytes and control characters except newlines and tabs
    let cleaned = input
        .chars()
        .filter(|c| !matches!(c, '\0' | '\x01'..='\x1F' | '\x7F'))
        .collect::<String>();

    // Truncate to max length
    if cleaned.len() > max_length {
        cleaned.chars().take(max_length).collect()
    } else {
        cleaned
    }
}

/// Sanitize a process name (more restrictive)
pub fn sanitize_process_name(input: &str) -> String {
    let cleaned = input.trim();

    // Allow only alphanumeric, dots, hyphens, underscores, and .exe extension
    cleaned
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect::<String>()
        .trim()
        .to_lowercase()
}

/// Validate and sanitize a hotkey string
pub fn sanitize_hotkey(input: &str) -> String {
    let cleaned = sanitize_string(input, 50);

    // Only allow valid hotkey characters
    cleaned
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '+' | ' '))
        .collect()
}

/// Validate hex color string
pub fn is_valid_hex_color(color: &str) -> bool {
    let cleaned = color.trim().trim_start_matches('#');
    cleaned.len() == 6 && cleaned.chars().all(|c| c.is_ascii_hexdigit())
}

/// Check for potential injection patterns
/// Only checks for actual injection syntax, not legitimate process/file names
pub fn contains_injection_patterns(input: &str) -> bool {
    let lower = input.to_lowercase();
    // Only include patterns that are actual injection indicators, not common legitimate strings
    let patterns = [
        "<script",
        "</script",
        "javascript:",
        "vbscript:",
        "onload=",
        "onerror=",
        "onclick=",
        "onmouseover=",
        "onfocus=",
        "onblur=",
        "onchange=",
        "eval(",
        "expression(",
        "url(",
        "import(",
        "require(",
        "\\x",
        "\\u",
        "\\n",
        "\\r",
        "\\t",
        "--",
        "/*",
        "*/",
        "xp_",
        "sp_",
        // Removed: "exec", "system", "shell", "cmd", "bash", "sh"
        // These are too broad and match legitimate process names like:
        // cmd.exe, shell.exe, system.exe, bash shell, sh script, etc.
        // Only include truly dangerous indicators above
    ];

    patterns.iter().any(|pattern| lower.contains(pattern))
}

/// Rate limiter for API calls
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    requests: HashMap<String, Vec<Instant>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: HashMap::new(),
            max_requests,
            window,
        }
    }

    pub fn check_rate_limit(&mut self, identifier: &str) -> bool {
        let now = Instant::now();
        let entry = self
            .requests
            .entry(identifier.to_string())
            .or_insert_with(Vec::new);

        // Remove old requests outside the window
        entry.retain(|&time| now.duration_since(time) < self.window);

        // Check if under limit
        if entry.len() < self.max_requests {
            entry.push(now);
            true
        } else {
            false
        }
    }
}
