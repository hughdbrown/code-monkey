use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SlideAction {
    Next,
    Prev,
    GoTo(u32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Directive {
    Say(String),
    Type(String),
    Run,
    Pause(Option<u64>),
    Focus(String),
    Slide(SlideAction),
    Key(String),
    Clear,
    Wait(u64),
    Exec(String),
    Section(String),
}

impl fmt::Display for Directive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Directive::Say(text) => write!(f, "[SAY] {text}"),
            Directive::Type(text) => write!(f, "[TYPE] {text}"),
            Directive::Run => write!(f, "[RUN]"),
            Directive::Pause(None) => write!(f, "[PAUSE]"),
            Directive::Pause(Some(secs)) => write!(f, "[PAUSE {secs}]"),
            Directive::Focus(app) => write!(f, "[FOCUS] {app}"),
            Directive::Slide(SlideAction::Next) => write!(f, "[SLIDE next]"),
            Directive::Slide(SlideAction::Prev) => write!(f, "[SLIDE prev]"),
            Directive::Slide(SlideAction::GoTo(n)) => write!(f, "[SLIDE {n}]"),
            Directive::Key(combo) => write!(f, "[KEY {combo}]"),
            Directive::Clear => write!(f, "[CLEAR]"),
            Directive::Wait(secs) => write!(f, "[WAIT {secs}]"),
            Directive::Exec(cmd) => write!(f, "[EXEC {cmd}]"),
            Directive::Section(name) => write!(f, "## Section: {name}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub typing_speed: u64,
    pub typing_variance: u64,
    pub agent_port: u16,
}

impl Default for FrontMatter {
    fn default() -> Self {
        Self {
            title: None,
            typing_speed: 40,
            typing_variance: 15,
            agent_port: 9876,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedLine {
    pub line_number: usize,
    pub directive: Directive,
}

#[derive(Debug, Clone)]
pub struct Script {
    pub front_matter: FrontMatter,
    pub lines: Vec<ParsedLine>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directive_display() {
        assert_eq!(Directive::Say("hello".into()).to_string(), "[SAY] hello");
        assert_eq!(
            Directive::Type("cargo build".into()).to_string(),
            "[TYPE] cargo build"
        );
        assert_eq!(Directive::Run.to_string(), "[RUN]");
        assert_eq!(Directive::Pause(None).to_string(), "[PAUSE]");
        assert_eq!(Directive::Pause(Some(3)).to_string(), "[PAUSE 3]");
        assert_eq!(
            Directive::Focus("Terminal".into()).to_string(),
            "[FOCUS] Terminal"
        );
        assert_eq!(
            Directive::Slide(SlideAction::Next).to_string(),
            "[SLIDE next]"
        );
        assert_eq!(
            Directive::Slide(SlideAction::Prev).to_string(),
            "[SLIDE prev]"
        );
        assert_eq!(
            Directive::Slide(SlideAction::GoTo(5)).to_string(),
            "[SLIDE 5]"
        );
        assert_eq!(Directive::Key("cmd+s".into()).to_string(), "[KEY cmd+s]");
        assert_eq!(Directive::Clear.to_string(), "[CLEAR]");
        assert_eq!(Directive::Wait(2).to_string(), "[WAIT 2]");
        assert_eq!(
            Directive::Exec("cargo build".into()).to_string(),
            "[EXEC cargo build]"
        );
        assert_eq!(
            Directive::Section("Intro".into()).to_string(),
            "## Section: Intro"
        );
    }

    #[test]
    fn test_front_matter_defaults() {
        let fm = FrontMatter::default();
        assert_eq!(fm.title, None);
        assert_eq!(fm.typing_speed, 40);
        assert_eq!(fm.typing_variance, 15);
        assert_eq!(fm.agent_port, 9876);
    }
}
