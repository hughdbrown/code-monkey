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
