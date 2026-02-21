use super::lexer::ParseError;
use super::types::FrontMatter;

pub fn extract_front_matter(lines: &[&str]) -> Result<(FrontMatter, usize), ParseError> {
    if lines.is_empty() || lines[0].trim() != "---" {
        return Ok((FrontMatter::default(), 0));
    }

    // Find closing ---
    let closing = lines[1..].iter().position(|l| l.trim() == "---");
    let closing_idx = match closing {
        Some(idx) => idx + 1, // offset by 1 because we started searching from index 1
        None => {
            return Err(ParseError {
                line_number: 1,
                line_content: "---".to_string(),
                message: "Front matter opened but never closed with '---'".to_string(),
            });
        }
    };

    let mut fm = FrontMatter::default();

    for (i, line) in lines[1..closing_idx].iter().enumerate() {
        let line_number = i + 2; // 1-indexed, offset by opening ---
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Strip inline comments
        let without_comment = if let Some(hash_pos) = trimmed.find('#') {
            trimmed[..hash_pos].trim()
        } else {
            trimmed
        };

        let Some((key, value)) = without_comment.split_once(':') else {
            return Err(ParseError {
                line_number,
                line_content: line.to_string(),
                message: "Expected 'key: value' format in front matter".to_string(),
            });
        };

        let key = key.trim();
        let value = value.trim();

        match key {
            "title" => {
                fm.title = Some(value.to_string());
            }
            "typing_speed" => {
                fm.typing_speed = value.parse().map_err(|_| ParseError {
                    line_number,
                    line_content: line.to_string(),
                    message: format!("Invalid typing_speed value: '{value}'"),
                })?;
            }
            "typing_variance" => {
                fm.typing_variance = value.parse().map_err(|_| ParseError {
                    line_number,
                    line_content: line.to_string(),
                    message: format!("Invalid typing_variance value: '{value}'"),
                })?;
            }
            "agent_port" => {
                fm.agent_port = value.parse().map_err(|_| ParseError {
                    line_number,
                    line_content: line.to_string(),
                    message: format!("Invalid agent_port value: '{value}'"),
                })?;
            }
            _ => {
                // Unknown keys are silently ignored
            }
        }
    }

    Ok((fm, closing_idx + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_front_matter_basic() {
        let lines: Vec<&str> = "---\ntitle: My Talk\ntyping_speed: 60\n---\n[SAY] hi"
            .lines()
            .collect();
        let (fm, start) = extract_front_matter(&lines).unwrap();
        assert_eq!(fm.title, Some("My Talk".to_string()));
        assert_eq!(fm.typing_speed, 60);
        assert_eq!(fm.typing_variance, 15);
        assert_eq!(fm.agent_port, 9876);
        assert_eq!(start, 4);
    }

    #[test]
    fn test_front_matter_missing() {
        let lines: Vec<&str> = "[SAY] hi".lines().collect();
        let (fm, start) = extract_front_matter(&lines).unwrap();
        assert_eq!(fm, FrontMatter::default());
        assert_eq!(start, 0);
    }

    #[test]
    fn test_front_matter_empty() {
        let lines: Vec<&str> = "---\n---\n[SAY] hi".lines().collect();
        let (fm, start) = extract_front_matter(&lines).unwrap();
        assert_eq!(fm, FrontMatter::default());
        assert_eq!(start, 2);
    }

    #[test]
    fn test_front_matter_unknown_key_warns() {
        let lines: Vec<&str> = "---\nfoo: bar\n---".lines().collect();
        let (fm, _) = extract_front_matter(&lines).unwrap();
        assert_eq!(fm.title, None); // unknown key ignored
    }

    #[test]
    fn test_front_matter_invalid_number() {
        let lines: Vec<&str> = "---\ntyping_speed: abc\n---".lines().collect();
        let err = extract_front_matter(&lines).unwrap_err();
        assert!(err.to_string().contains("typing_speed"));
    }

    #[test]
    fn test_front_matter_all_fields() {
        let lines: Vec<&str> =
            "---\ntitle: Demo\ntyping_speed: 50\ntyping_variance: 20\nagent_port: 4444\n---"
                .lines()
                .collect();
        let (fm, _) = extract_front_matter(&lines).unwrap();
        assert_eq!(fm.title, Some("Demo".to_string()));
        assert_eq!(fm.typing_speed, 50);
        assert_eq!(fm.typing_variance, 20);
        assert_eq!(fm.agent_port, 4444);
    }

    #[test]
    fn test_front_matter_agent_port() {
        let lines: Vec<&str> = "---\nagent_port: 4444\n---".lines().collect();
        let (fm, _) = extract_front_matter(&lines).unwrap();
        assert_eq!(fm.agent_port, 4444);
    }

    #[test]
    fn test_front_matter_with_inline_comments() {
        let lines: Vec<&str> = "---\ntyping_speed: 60  # fast typing\n---"
            .lines()
            .collect();
        let (fm, _) = extract_front_matter(&lines).unwrap();
        assert_eq!(fm.typing_speed, 60);
    }
}
