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
