use anyhow::Result;
use std::thread;
use std::time::Duration;

use super::applescript::{run_applescript, type_char_script};

pub fn typewriter_to_applescript(
    text: &str,
    speed_ms: u64,
    variance_ms: u64,
) -> Vec<(String, u64)> {
    text.chars()
        .map(|ch| {
            let script = type_char_script(ch);
            let delay = if variance_ms > 0 {
                speed_ms + fastrand::u64(0..=variance_ms)
            } else {
                speed_ms
            };
            (script, delay)
        })
        .collect()
}

pub fn execute_typewriter(text: &str, speed_ms: u64, variance_ms: u64) -> Result<()> {
    for (script, delay) in typewriter_to_applescript(text, speed_ms, variance_ms) {
        run_applescript(&script)?;
        thread::sleep(Duration::from_millis(delay));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typewriter_generates_per_char_scripts() {
        let pairs = typewriter_to_applescript("hello", 40, 0);
        assert_eq!(pairs.len(), 5);
        for (script, delay) in &pairs {
            assert!(script.contains("keystroke"));
            assert_eq!(*delay, 40);
        }
    }

    #[test]
    fn test_typewriter_empty_string() {
        let pairs = typewriter_to_applescript("", 40, 0);
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_typewriter_special_chars() {
        let pairs = typewriter_to_applescript("a b!", 40, 0);
        assert_eq!(pairs.len(), 4);
    }

    #[test]
    fn test_typewriter_variance_range() {
        let pairs = typewriter_to_applescript("test", 40, 10);
        for (_, delay) in &pairs {
            assert!(*delay >= 40 && *delay <= 50, "delay {delay} out of range");
        }
    }
}
