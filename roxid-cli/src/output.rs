// Output formatting helpers for CLI commands

/// Print a status message: "  Status message"
pub fn status(action: &str, message: &str) {
    eprintln!("\x1b[1;36m{:>12}\x1b[0m {}", action, message);
}

/// Print a success message with checkmark
pub fn success(message: &str) {
    eprintln!("\x1b[1;32m  \u{2713}\x1b[0m {}", message);
}

/// Print a failure message with X
pub fn failure(message: &str) {
    eprintln!("\x1b[1;31m  \u{2717}\x1b[0m {}", message);
}

/// Print a check/pass item
pub fn check(message: &str) {
    eprintln!("\x1b[32m  \u{2713}\x1b[0m {}", message);
}

/// Print a warning message
pub fn warning(message: &str) {
    eprintln!("\x1b[33m  !\x1b[0m {}", message);
}

/// Print an error message
pub fn error(message: &str) {
    eprintln!("\x1b[1;31merror:\x1b[0m {}", message);
}

/// Print an info message
pub fn info(message: &str) {
    eprintln!("\x1b[36m  i\x1b[0m {}", message);
}

/// Print a dim/muted message
pub fn dim(message: &str) {
    eprintln!("\x1b[2m{}\x1b[0m", message);
}

/// Print a dim success message
pub fn dim_success(message: &str) {
    eprintln!("\x1b[32m{}\x1b[0m", message);
}

/// Print a dim failure message
pub fn dim_failure(message: &str) {
    eprintln!("\x1b[31m{}\x1b[0m", message);
}

/// Print a stage header
pub fn stage_header(name: &str, total_jobs: usize) {
    eprintln!("\x1b[1;34m  Stage\x1b[0m '{}' ({} jobs)", name, total_jobs);
}

/// Print step output (indented)
pub fn step_output(line: &str) {
    println!("        | {}", line);
}

/// Print step error output (indented, red)
pub fn step_error(line: &str) {
    eprintln!("\x1b[31m        | {}\x1b[0m", line);
}

/// Print a header line
pub fn header(message: &str) {
    eprintln!("\x1b[1m==> {}\x1b[0m", message);
}
