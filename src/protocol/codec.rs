use anyhow::Result;

use super::messages::Message;

pub fn encode_message(msg: &Message) -> Result<Vec<u8>> {
    let json = serde_json::to_vec(msg)?;
    let len = json.len() as u32;
    let mut buf = Vec::with_capacity(4 + json.len());
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(&json);
    Ok(buf)
}

/// Decode a message from a buffer.
/// Returns `Ok(None)` if the buffer doesn't contain a complete message yet.
/// Returns `Ok(Some((message, bytes_consumed)))` on success.
pub fn decode_message(buf: &[u8]) -> Result<Option<(Message, usize)>> {
    if buf.len() < 4 {
        return Ok(None);
    }

    let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;

    if buf.len() < 4 + len {
        return Ok(None);
    }

    let msg: Message = serde_json::from_slice(&buf[4..4 + len])?;
    Ok(Some((msg, 4 + len)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::types::Directive;
    use crate::protocol::messages::AckStatus;

    #[test]
    fn test_encode_message() {
        let msg = Message::Ping;
        let encoded = encode_message(&msg).unwrap();
        assert!(encoded.len() > 4);
        let len = u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]) as usize;
        assert_eq!(encoded.len(), 4 + len);
    }

    #[test]
    fn test_decode_message() {
        let msg = Message::Ack {
            status: AckStatus::Ok,
            message: None,
        };
        let encoded = encode_message(&msg).unwrap();
        let (decoded, consumed) = decode_message(&encoded).unwrap().unwrap();
        assert_eq!(decoded, msg);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn test_decode_partial_read() {
        let msg = Message::Ping;
        let encoded = encode_message(&msg).unwrap();
        // Only provide first 2 bytes
        assert!(decode_message(&encoded[..2]).unwrap().is_none());
        // Provide length but not full body
        if encoded.len() > 5 {
            assert!(decode_message(&encoded[..5]).unwrap().is_none());
        }
    }

    #[test]
    fn test_roundtrip_large_message() {
        let long_text = "x".repeat(10000);
        let msg = Message::Execute {
            actions: vec![Directive::Type(long_text.clone())],
            typing_speed: 40,
            typing_variance: 15,
        };
        let encoded = encode_message(&msg).unwrap();
        let (decoded, consumed) = decode_message(&encoded).unwrap().unwrap();
        assert_eq!(decoded, msg);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn test_length_prefix_roundtrip() {
        let msg = Message::Execute {
            actions: vec![Directive::Focus("Terminal".into()), Directive::Run],
            typing_speed: 40,
            typing_variance: 15,
        };
        let encoded = encode_message(&msg).unwrap();
        let (decoded, _) = decode_message(&encoded).unwrap().unwrap();
        assert_eq!(decoded, msg);
    }
}
