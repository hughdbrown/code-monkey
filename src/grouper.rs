use serde::{Deserialize, Serialize};

use crate::parser::types::{Directive, Script};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockType {
    Action,
    Pause(Option<u64>),
    NarrationOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionBlock {
    pub narration: Option<String>,
    pub actions: Vec<Directive>,
    pub section: Option<String>,
    pub block_type: BlockType,
}

pub fn group_into_blocks(script: &Script) -> Vec<ActionBlock> {
    let mut blocks = Vec::new();
    let mut current_narration: Vec<String> = Vec::new();
    let mut current_actions: Vec<Directive> = Vec::new();
    let mut current_section: Option<String> = None;

    for parsed_line in &script.lines {
        match &parsed_line.directive {
            Directive::Say(text) => {
                // Flush any pending action block before accumulating narration
                if !current_actions.is_empty() {
                    blocks.push(ActionBlock {
                        narration: flush_narration(&mut current_narration),
                        actions: std::mem::take(&mut current_actions),
                        section: current_section.clone(),
                        block_type: BlockType::Action,
                    });
                }
                current_narration.push(text.clone());
            }
            Directive::Section(name) => {
                // Flush any pending action block
                if !current_actions.is_empty() {
                    blocks.push(ActionBlock {
                        narration: flush_narration(&mut current_narration),
                        actions: std::mem::take(&mut current_actions),
                        section: current_section.clone(),
                        block_type: BlockType::Action,
                    });
                }
                current_section = Some(name.clone());
            }
            Directive::Pause(timeout) => {
                // Flush any pending action block first
                if !current_actions.is_empty() {
                    blocks.push(ActionBlock {
                        narration: flush_narration(&mut current_narration),
                        actions: std::mem::take(&mut current_actions),
                        section: current_section.clone(),
                        block_type: BlockType::Action,
                    });
                }
                // Pause is always its own block
                blocks.push(ActionBlock {
                    narration: flush_narration(&mut current_narration),
                    actions: vec![],
                    section: current_section.clone(),
                    block_type: BlockType::Pause(*timeout),
                });
            }
            directive => {
                // All other directives accumulate into the current action block
                current_actions.push(directive.clone());
            }
        }
    }

    // Flush remaining
    if !current_actions.is_empty() {
        blocks.push(ActionBlock {
            narration: flush_narration(&mut current_narration),
            actions: std::mem::take(&mut current_actions),
            section: current_section.clone(),
            block_type: BlockType::Action,
        });
    } else if !current_narration.is_empty() {
        blocks.push(ActionBlock {
            narration: flush_narration(&mut current_narration),
            actions: vec![],
            section: current_section.clone(),
            block_type: BlockType::NarrationOnly,
        });
    }

    blocks
}

fn flush_narration(narration: &mut Vec<String>) -> Option<String> {
    if narration.is_empty() {
        None
    } else {
        let text = narration.join("\n");
        narration.clear();
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::types::{FrontMatter, ParsedLine, Script, SlideAction};

    fn make_script(directives: Vec<Directive>) -> Script {
        Script {
            front_matter: FrontMatter::default(),
            lines: directives
                .into_iter()
                .enumerate()
                .map(|(i, directive)| ParsedLine {
                    line_number: i + 1,
                    directive,
                })
                .collect(),
        }
    }

    #[test]
    fn test_group_single_action() {
        let script = make_script(vec![Directive::Type("hello".into()), Directive::Run]);
        let blocks = group_into_blocks(&script);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].actions.len(), 2);
        assert_eq!(blocks[0].block_type, BlockType::Action);
    }

    #[test]
    fn test_group_say_before_action() {
        let script = make_script(vec![
            Directive::Say("text".into()),
            Directive::Focus("T".into()),
            Directive::Type("cmd".into()),
            Directive::Run,
        ]);
        let blocks = group_into_blocks(&script);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].narration, Some("text".to_string()));
        assert_eq!(blocks[0].actions.len(), 3);
    }

    #[test]
    fn test_group_multiple_say_accumulate() {
        let script = make_script(vec![
            Directive::Say("line1".into()),
            Directive::Say("line2".into()),
            Directive::Type("x".into()),
        ]);
        let blocks = group_into_blocks(&script);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].narration, Some("line1\nline2".to_string()));
    }

    #[test]
    fn test_group_pause_standalone() {
        let script = make_script(vec![
            Directive::Type("x".into()),
            Directive::Pause(None),
            Directive::Type("y".into()),
        ]);
        let blocks = group_into_blocks(&script);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].block_type, BlockType::Action);
        assert_eq!(blocks[1].block_type, BlockType::Pause(None));
        assert_eq!(blocks[2].block_type, BlockType::Action);
    }

    #[test]
    fn test_group_pause_with_timeout() {
        let script = make_script(vec![Directive::Pause(Some(3))]);
        let blocks = group_into_blocks(&script);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Pause(Some(3)));
    }

    #[test]
    fn test_group_section_header() {
        let script = make_script(vec![
            Directive::Section("Intro".into()),
            Directive::Say("hello".into()),
            Directive::Type("x".into()),
        ]);
        let blocks = group_into_blocks(&script);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].section, Some("Intro".to_string()));
    }

    #[test]
    fn test_group_empty_script() {
        let script = make_script(vec![]);
        let blocks = group_into_blocks(&script);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_group_say_only() {
        let script = make_script(vec![Directive::Say("text".into())]);
        let blocks = group_into_blocks(&script);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::NarrationOnly);
        assert_eq!(blocks[0].narration, Some("text".to_string()));
    }

    #[test]
    fn test_group_complex_script() {
        let script = make_script(vec![
            Directive::Section("Intro".into()),
            Directive::Say("Welcome".into()),
            Directive::Focus("Terminal".into()),
            Directive::Type("echo hi".into()),
            Directive::Run,
            Directive::Pause(None),
            Directive::Say("Now watch".into()),
            Directive::Type("ls".into()),
            Directive::Run,
            Directive::Pause(Some(3)),
            Directive::Section("Demo".into()),
            Directive::Slide(SlideAction::Next),
            Directive::Say("That's all".into()),
        ]);
        let blocks = group_into_blocks(&script);
        assert_eq!(blocks.len(), 6);
        // Block 0: action (Focus + Type + Run) with narration "Welcome"
        assert_eq!(blocks[0].block_type, BlockType::Action);
        assert_eq!(blocks[0].narration, Some("Welcome".to_string()));
        assert_eq!(blocks[0].actions.len(), 3);
        // Block 1: pause
        assert_eq!(blocks[1].block_type, BlockType::Pause(None));
        // Block 2: action (Type + Run) with narration "Now watch"
        assert_eq!(blocks[2].block_type, BlockType::Action);
        assert_eq!(blocks[2].narration, Some("Now watch".to_string()));
        // Block 3: pause with timeout
        assert_eq!(blocks[3].block_type, BlockType::Pause(Some(3)));
        // Block 4: action (Slide next) with section "Demo"
        assert_eq!(blocks[4].block_type, BlockType::Action);
        assert_eq!(blocks[4].section, Some("Demo".to_string()));
        // Block 5: narration only "That's all"
        assert_eq!(blocks[5].block_type, BlockType::NarrationOnly);
        assert_eq!(blocks[5].narration, Some("That's all".to_string()));
    }
}
