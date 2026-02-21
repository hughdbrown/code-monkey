pub mod front_matter;
pub mod lexer;
pub mod types;

use lexer::ParseError;
use types::Script;

pub fn parse_script(input: &str) -> Result<Script, ParseError> {
    let lines: Vec<&str> = input.lines().collect();
    let (front_matter, content_start) = front_matter::extract_front_matter(&lines)?;

    let mut parsed_lines = Vec::new();
    for (idx, line) in lines[content_start..].iter().enumerate() {
        let line_number = content_start + idx + 1;
        if let Some(parsed) = lexer::parse_line(line, line_number)? {
            parsed_lines.push(parsed);
        }
    }

    Ok(Script {
        front_matter,
        lines: parsed_lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::FrontMatter;

    #[test]
    fn test_parse_empty_script() {
        let script = parse_script("").unwrap();
        assert!(script.lines.is_empty());
        assert_eq!(script.front_matter, FrontMatter::default());
    }

    #[test]
    fn test_parse_comments_only() {
        let input = "# comment 1\n# comment 2\n";
        let script = parse_script(input).unwrap();
        assert!(script.lines.is_empty());
    }

    #[test]
    fn test_parse_multi_line_script() {
        let input = "[SAY] Hello\n[TYPE] cargo build\n[RUN]\n";
        let script = parse_script(input).unwrap();
        assert_eq!(script.lines.len(), 3);
    }

    #[test]
    fn test_parse_error_includes_line_number() {
        let input = "# comment\n# comment\n# comment\n# comment\n[BOGUS]\n";
        let err = parse_script(input).unwrap_err();
        assert!(
            err.to_string().contains("5"),
            "Error should mention line 5: {err}"
        );
    }

    #[test]
    fn test_parse_full_script_roundtrip() {
        let input = "\
---
title: Test Talk
typing_speed: 60
---

## Section: Intro

[SAY] Welcome everyone.
[SAY] Let me show you something.

[FOCUS] Terminal
[TYPE] echo hello
[RUN]
[PAUSE]

## Section: Demo

[SAY] Now watch this.
[TYPE] ls -la
[RUN]
[PAUSE 3]

[SLIDE next]
";
        let script = parse_script(input).unwrap();
        assert_eq!(script.front_matter.title, Some("Test Talk".to_string()));
        assert_eq!(script.front_matter.typing_speed, 60);
        assert_eq!(script.lines.len(), 13);
    }
}
