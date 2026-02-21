use anyhow::Result;
use std::process::Command;

pub fn focus_app_script(app_name: &str) -> String {
    let escaped = app_name.replace('\\', "\\\\").replace('"', "\\\"");
    format!("tell application \"{escaped}\" to activate")
}

pub fn slide_next_script() -> String {
    "tell application \"Keynote\"\nshow next\nend tell".to_string()
}

pub fn slide_prev_script() -> String {
    "tell application \"Keynote\"\nshow previous\nend tell".to_string()
}

pub fn slide_goto_script(n: u32) -> String {
    format!(
        "tell application \"Keynote\"\ntell front document\nset current slide to slide {n}\nend tell\nend tell"
    )
}

pub fn keystroke_script(combo: &str) -> String {
    let (modifiers, key) = parse_key_combo(combo);

    // Special keys that need key codes
    let key_code = match key {
        "return" | "enter" => Some(36),
        "tab" => Some(48),
        "space" => Some(49),
        "delete" | "backspace" => Some(51),
        "escape" | "esc" => Some(53),
        "left" => Some(123),
        "right" => Some(124),
        "down" => Some(125),
        "up" => Some(126),
        _ => None,
    };

    let modifier_str = if modifiers.is_empty() {
        String::new()
    } else {
        let mod_list: Vec<&str> = modifiers
            .iter()
            .map(|m| match *m {
                "cmd" | "command" => "command down",
                "ctrl" | "control" => "control down",
                "shift" => "shift down",
                "alt" | "opt" | "option" => "option down",
                other => other,
            })
            .collect();
        if mod_list.len() == 1 {
            format!(" using {}", mod_list[0])
        } else {
            format!(" using {{{}}}", mod_list.join(", "))
        }
    };

    if let Some(code) = key_code {
        format!("tell application \"System Events\" to key code {code}{modifier_str}")
    } else {
        format!("tell application \"System Events\" to keystroke \"{key}\"{modifier_str}")
    }
}

pub fn type_char_script(ch: char) -> String {
    if ch == '"' {
        "tell application \"System Events\" to keystroke \"\\\"\"".to_string()
    } else if ch == '\\' {
        "tell application \"System Events\" to keystroke \"\\\\\"".to_string()
    } else {
        format!("tell application \"System Events\" to keystroke \"{ch}\"")
    }
}

pub fn clear_script() -> String {
    keystroke_script("ctrl+l")
}

pub fn run_applescript(script: &str) -> Result<String> {
    let output = Command::new("osascript").arg("-e").arg(script).output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        anyhow::bail!("osascript error: {err}");
    }
}

fn parse_key_combo(combo: &str) -> (Vec<&str>, &str) {
    let parts: Vec<&str> = combo.split('+').collect();
    if parts.len() == 1 {
        (vec![], parts[0])
    } else {
        let modifiers = parts[..parts.len() - 1].to_vec();
        let key = parts[parts.len() - 1];
        (modifiers, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_app_script() {
        assert_eq!(
            focus_app_script("Terminal"),
            "tell application \"Terminal\" to activate"
        );
    }

    #[test]
    fn test_focus_app_escapes_quotes() {
        let script = focus_app_script("My \"App\"");
        assert!(script.contains("My \\\"App\\\""));
    }

    #[test]
    fn test_slide_next_script() {
        let script = slide_next_script();
        assert!(script.contains("show next"));
        assert!(script.contains("Keynote"));
    }

    #[test]
    fn test_slide_prev_script() {
        let script = slide_prev_script();
        assert!(script.contains("show previous"));
    }

    #[test]
    fn test_slide_goto_script() {
        let script = slide_goto_script(5);
        assert!(script.contains("slide 5"));
    }

    #[test]
    fn test_keystroke_simple() {
        assert_eq!(
            keystroke_script("a"),
            "tell application \"System Events\" to keystroke \"a\""
        );
    }

    #[test]
    fn test_keystroke_with_cmd() {
        let script = keystroke_script("cmd+s");
        assert!(script.contains("keystroke \"s\""));
        assert!(script.contains("using command down"));
    }

    #[test]
    fn test_keystroke_with_multiple_modifiers() {
        let script = keystroke_script("cmd+shift+s");
        assert!(script.contains("keystroke \"s\""));
        assert!(script.contains("command down"));
        assert!(script.contains("shift down"));
    }

    #[test]
    fn test_keystroke_return() {
        let script = keystroke_script("return");
        assert!(script.contains("key code 36"));
    }

    #[test]
    fn test_keystroke_ctrl_c() {
        let script = keystroke_script("ctrl+c");
        assert!(script.contains("keystroke \"c\""));
        assert!(script.contains("using control down"));
    }

    #[test]
    fn test_type_char_script() {
        let script = type_char_script('h');
        assert_eq!(
            script,
            "tell application \"System Events\" to keystroke \"h\""
        );
    }

    #[test]
    fn test_clear_script() {
        let script = clear_script();
        assert!(script.contains("control down"));
        assert!(script.contains("keystroke \"l\""));
    }
}
