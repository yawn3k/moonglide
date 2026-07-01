pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const RED: &str = "\x1b[31m";
pub const RESET: &str = "\x1b[0m";

pub fn bold(s: &str) -> String { format!("{}{}{}", BOLD, s, RESET) }
pub fn dim(s: &str) -> String { format!("{}{}{}", DIM, s, RESET) }
pub fn green(s: &str) -> String { format!("{}{}{}", GREEN, s, RESET) }
pub fn yellow(s: &str) -> String { format!("{}{}{}", YELLOW, s, RESET) }
pub fn info(s: &str) -> String { format!("{}{}{}{}", BOLD, GREEN, s, RESET) }
pub fn warn(s: &str) -> String { format!("{}{}{}{}", BOLD, YELLOW, s, RESET) }
pub fn err(s: &str) -> String { format!("{}{}{}{}", BOLD, RED, s, RESET) }
pub fn progress(s: &str) -> String { format!("{}{}{}", DIM, s, RESET) }
