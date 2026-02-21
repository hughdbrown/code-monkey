use super::types::{Directive, ParsedLine, SlideAction};

#[derive(Debug, thiserror::Error)]
#[error("Parse error at line {line_number}: {message}\n  | {line_content}")]
pub struct ParseError {
    pub line_number: usize,
    pub line_content: String,
    pub message: String,
}

pub fn parse_line(line: &str, line_number: usize) -> Result<Option<ParsedLine>, ParseError> {
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    // Comments
    if trimmed.starts_with('#') && !trimmed.starts_with("## Section:") {
        return Ok(None);
    }

    // Section headers
    if let Some(rest) = trimmed.strip_prefix("## Section:") {
        let name = rest.trim().to_string();
        return Ok(Some(ParsedLine {
            line_number,
            directive: Directive::Section(name),
        }));
    }

    // Bracket directives: [DIRECTIVE] optional_arg
    if trimmed.starts_with('[') {
        let directive = parse_bracket_directive(trimmed, line_number)?;
        return Ok(Some(ParsedLine {
            line_number,
            directive,
        }));
    }

    Err(ParseError {
        line_number,
        line_content: line.to_string(),
        message: "Unrecognized line format".to_string(),
    })
}

fn parse_bracket_directive(line: &str, line_number: usize) -> Result<Directive, ParseError> {
    // Find the closing bracket
    let close_bracket: usize = line.find(']').ok_or_else(|| ParseError {
        line_number,
        line_content: line.to_string(),
        message: "Missing closing bracket ']'".to_string(),
    })?;

    let inside = &line[1..close_bracket];
    let after = line[close_bracket + 1..].trim();

    // Split inside brackets: tag is the first word, rest is inline arg
    let (tag_str, inline_arg) = match inside.find(' ') {
        Some(pos) => (&inside[..pos], inside[pos + 1..].trim()),
        None => (inside, ""),
    };
    let tag_upper = tag_str.to_uppercase();

    // Combine inline arg and after-bracket arg (prefer after-bracket if both present)
    let arg = if !after.is_empty() { after } else { inline_arg };

    match tag_upper.as_str() {
        "SAY" => Ok(Directive::Say(arg.to_string())),
        "TYPE" => Ok(Directive::Type(arg.to_string())),
        "RUN" => Ok(Directive::Run),
        "PAUSE" => {
            if arg.is_empty() {
                Ok(Directive::Pause(None))
            } else {
                let secs: u64 = arg.parse().map_err(|_| ParseError {
                    line_number,
                    line_content: line.to_string(),
                    message: format!("Invalid PAUSE duration: '{arg}'"),
                })?;
                Ok(Directive::Pause(Some(secs)))
            }
        }
        "FOCUS" => Ok(Directive::Focus(arg.to_string())),
        "SLIDE" => {
            let action = match arg.to_lowercase().as_str() {
                "next" => SlideAction::Next,
                "prev" | "previous" => SlideAction::Prev,
                other => {
                    let n: u32 = other.parse().map_err(|_| ParseError {
                        line_number,
                        line_content: line.to_string(),
                        message: format!(
                            "Invalid SLIDE argument: '{arg}' (expected 'next', 'prev', or a number)"
                        ),
                    })?;
                    SlideAction::GoTo(n)
                }
            };
            Ok(Directive::Slide(action))
        }
        "KEY" => Ok(Directive::Key(arg.to_string())),
        "CLEAR" => Ok(Directive::Clear),
        "WAIT" => {
            let secs: u64 = arg.parse().map_err(|_| ParseError {
                line_number,
                line_content: line.to_string(),
                message: format!("Invalid WAIT duration: '{arg}'"),
            })?;
            Ok(Directive::Wait(secs))
        }
        "EXEC" => Ok(Directive::Exec(arg.to_string())),
        _ => Err(ParseError {
            line_number,
            line_content: line.to_string(),
            message: format!("Unknown directive: [{tag_str}]"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_line() {
        assert!(parse_line("", 1).unwrap().is_none());
        assert!(parse_line("   ", 1).unwrap().is_none());
    }

    #[test]
    fn test_parse_comment() {
        assert!(parse_line("# this is a comment", 1).unwrap().is_none());
    }

    #[test]
    fn test_parse_say() {
        let parsed = parse_line("[SAY] Hello world", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Say("Hello world".into()));
    }

    #[test]
    fn test_parse_type() {
        let parsed = parse_line("[TYPE] cargo build", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Type("cargo build".into()));
    }

    #[test]
    fn test_parse_run() {
        let parsed = parse_line("[RUN]", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Run);
    }

    #[test]
    fn test_parse_pause_no_arg() {
        let parsed = parse_line("[PAUSE]", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Pause(None));
    }

    #[test]
    fn test_parse_pause_with_seconds() {
        let parsed = parse_line("[PAUSE 3]", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Pause(Some(3)));
    }

    #[test]
    fn test_parse_focus() {
        let parsed = parse_line("[FOCUS] Terminal", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Focus("Terminal".into()));
    }

    #[test]
    fn test_parse_slide_next() {
        let parsed = parse_line("[SLIDE next]", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Slide(SlideAction::Next));
    }

    #[test]
    fn test_parse_slide_prev() {
        let parsed = parse_line("[SLIDE prev]", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Slide(SlideAction::Prev));
    }

    #[test]
    fn test_parse_slide_number() {
        let parsed = parse_line("[SLIDE 5]", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Slide(SlideAction::GoTo(5)));
    }

    #[test]
    fn test_parse_key() {
        let parsed = parse_line("[KEY cmd+s]", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Key("cmd+s".into()));
    }

    #[test]
    fn test_parse_clear() {
        let parsed = parse_line("[CLEAR]", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Clear);
    }

    #[test]
    fn test_parse_wait() {
        let parsed = parse_line("[WAIT 2]", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Wait(2));
    }

    #[test]
    fn test_parse_exec() {
        let parsed = parse_line("[EXEC cargo build --release]", 1)
            .unwrap()
            .unwrap();
        assert_eq!(
            parsed.directive,
            Directive::Exec("cargo build --release".into())
        );
    }

    #[test]
    fn test_parse_section() {
        let parsed = parse_line("## Section: Intro", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Section("Intro".into()));
    }

    #[test]
    fn test_parse_unknown_directive() {
        let err = parse_line("[BOGUS]", 5).unwrap_err();
        assert!(err.to_string().contains("5"));
        assert!(err.to_string().contains("BOGUS"));
    }

    #[test]
    fn test_parse_directive_case_insensitive() {
        let parsed = parse_line("[say] hello", 1).unwrap().unwrap();
        assert_eq!(parsed.directive, Directive::Say("hello".into()));
    }

    #[test]
    fn test_parse_say_preserves_whitespace() {
        let parsed = parse_line("[SAY]   spaced out  ", 1).unwrap().unwrap();
        // trim() is applied to the argument
        assert_eq!(parsed.directive, Directive::Say("spaced out".into()));
    }
}
