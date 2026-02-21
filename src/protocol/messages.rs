use serde::{Deserialize, Serialize};

use crate::parser::types::Directive;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    Execute {
        actions: Vec<Directive>,
        typing_speed: u64,
        typing_variance: u64,
    },
    Ack {
        status: AckStatus,
        message: Option<String>,
    },
    Ping,
    Pong,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AckStatus {
    Ok,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::types::SlideAction;

    #[test]
    fn test_serialize_execute() {
        let msg = Message::Execute {
            actions: vec![Directive::Run],
            typing_speed: 40,
            typing_variance: 15,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"Execute\""));
        assert!(json.contains("\"Run\""));
    }

    #[test]
    fn test_deserialize_execute() {
        let json = r#"{"type":"Execute","actions":["Run"],"typing_speed":40,"typing_variance":15}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        match msg {
            Message::Execute {
                actions,
                typing_speed,
                typing_variance,
            } => {
                assert_eq!(actions, vec![Directive::Run]);
                assert_eq!(typing_speed, 40);
                assert_eq!(typing_variance, 15);
            }
            _ => panic!("Expected Execute"),
        }
    }

    #[test]
    fn test_serialize_ack_ok() {
        let msg = Message::Ack {
            status: AckStatus::Ok,
            message: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"Ok\""));
    }

    #[test]
    fn test_serialize_ack_error() {
        let msg = Message::Ack {
            status: AckStatus::Error,
            message: Some("no accessibility".into()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"Error\""));
        assert!(json.contains("no accessibility"));
    }

    #[test]
    fn test_deserialize_ack() {
        let msg = Message::Ack {
            status: AckStatus::Ok,
            message: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let roundtrip: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, msg);
    }

    #[test]
    fn test_serialize_ping_pong() {
        let ping_json = serde_json::to_string(&Message::Ping).unwrap();
        let pong_json = serde_json::to_string(&Message::Pong).unwrap();
        assert_eq!(
            serde_json::from_str::<Message>(&ping_json).unwrap(),
            Message::Ping
        );
        assert_eq!(
            serde_json::from_str::<Message>(&pong_json).unwrap(),
            Message::Pong
        );
    }

    #[test]
    fn test_roundtrip_complex_execute() {
        let msg = Message::Execute {
            actions: vec![
                Directive::Focus("Terminal".into()),
                Directive::Type("cargo build".into()),
                Directive::Run,
                Directive::Slide(SlideAction::Next),
                Directive::Key("cmd+s".into()),
            ],
            typing_speed: 50,
            typing_variance: 20,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let roundtrip: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, msg);
    }
}
