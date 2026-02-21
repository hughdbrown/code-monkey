use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use socket2::{SockRef, TcpKeepalive};

use anyhow::Result;

use crate::grouper::{ActionBlock, BlockType, group_into_blocks};
use crate::parser::types::{FrontMatter, Script};
use crate::protocol::codec::{decode_message, encode_message};
use crate::protocol::messages::{AckStatus, Message};

#[derive(Debug, PartialEq)]
pub enum StepResult {
    Executed,
    Paused(Option<u64>),
    NarrationOnly,
    Finished,
    AgentError(String),
    ConnectionLost,
}

pub struct Presenter {
    blocks: Vec<ActionBlock>,
    current: usize,
    front_matter: FrontMatter,
    connection: Option<TcpStream>,
    agent_addr: SocketAddr,
}

impl Presenter {
    pub fn new(script: Script, agent_addr: SocketAddr) -> Self {
        let blocks = group_into_blocks(&script);
        let front_matter = script.front_matter.clone();
        Self {
            blocks,
            current: 0,
            front_matter,
            connection: None,
            agent_addr,
        }
    }

    pub fn connect(&mut self) -> Result<()> {
        let stream = TcpStream::connect_timeout(&self.agent_addr, Duration::from_secs(5))?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;

        let sock = SockRef::from(&stream);
        let keepalive = TcpKeepalive::new().with_time(Duration::from_secs(30));
        sock.set_tcp_keepalive(&keepalive)?;

        self.connection = Some(stream);

        // Validate the connection with a ping/pong handshake
        let response = self.send_and_receive(Message::Ping)?;
        if response != Message::Pong {
            self.connection = None;
            anyhow::bail!("Agent handshake failed: expected Pong, got {response:?}");
        }

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    pub fn current_block(&self) -> Option<&ActionBlock> {
        self.blocks.get(self.current)
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.current, self.blocks.len())
    }

    pub fn go_back(&mut self) {
        if self.current > 0 {
            self.current -= 1;
        }
    }

    pub fn skip(&mut self) {
        if self.current < self.blocks.len() {
            self.current += 1;
        }
    }

    pub fn step(&mut self) -> Result<StepResult> {
        let block = match self.blocks.get(self.current) {
            Some(b) => b.clone(),
            None => return Ok(StepResult::Finished),
        };

        let result = match &block.block_type {
            BlockType::NarrationOnly => {
                self.current += 1;
                StepResult::NarrationOnly
            }
            BlockType::Pause(timeout) => {
                self.current += 1;
                StepResult::Paused(*timeout)
            }
            BlockType::Action => {
                if block.actions.is_empty() {
                    self.current += 1;
                    return Ok(StepResult::Executed);
                }

                let msg = Message::Execute {
                    actions: block.actions.clone(),
                    typing_speed: self.front_matter.typing_speed,
                    typing_variance: self.front_matter.typing_variance,
                };

                match self.send_and_receive(msg) {
                    Ok(Message::Ack {
                        status: AckStatus::Ok,
                        ..
                    }) => {
                        self.current += 1;
                        StepResult::Executed
                    }
                    Ok(Message::Ack {
                        status: AckStatus::Error,
                        message,
                    }) => StepResult::AgentError(
                        message.unwrap_or_else(|| "Unknown agent error".into()),
                    ),
                    Ok(_) => StepResult::AgentError("Unexpected response from agent".into()),
                    Err(_) => {
                        self.connection = None;
                        StepResult::ConnectionLost
                    }
                }
            }
        };

        Ok(result)
    }

    fn send_and_receive(&mut self, msg: Message) -> Result<Message> {
        let stream = self
            .connection
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        let encoded = encode_message(&msg)?;
        stream.write_all(&encoded)?;
        stream.flush()?;

        let mut buf = vec![0u8; 65536];
        let mut pending = Vec::new();

        loop {
            let n = stream.read(&mut buf)?;
            if n == 0 {
                anyhow::bail!("Connection closed by agent");
            }
            pending.extend_from_slice(&buf[..n]);

            if let Some((response, _)) = decode_message(&pending)? {
                return Ok(response);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::types::{Directive, FrontMatter, ParsedLine};
    use std::net::TcpListener;
    use std::thread;

    fn make_test_script(directives: Vec<Directive>) -> Script {
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

    /// Mock server that handles the initial ping/pong handshake automatically,
    /// then responds with the provided messages for subsequent requests.
    fn start_mock_server(
        responses: Vec<Message>,
    ) -> (SocketAddr, std::thread::JoinHandle<Vec<Message>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();

            let mut received = Vec::new();
            let mut response_iter = responses.into_iter();
            let mut buf = vec![0u8; 65536];
            let mut pending = Vec::new();
            let mut handshake_done = false;

            loop {
                let n = match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };
                pending.extend_from_slice(&buf[..n]);

                while let Some((msg, consumed)) = decode_message(&pending).unwrap() {
                    pending.drain(..consumed);

                    // Auto-respond to the initial Ping handshake
                    if !handshake_done && msg == Message::Ping {
                        handshake_done = true;
                        let encoded = encode_message(&Message::Pong).unwrap();
                        stream.write_all(&encoded).unwrap();
                        stream.flush().unwrap();
                        continue;
                    }

                    received.push(msg);

                    if let Some(response) = response_iter.next() {
                        let encoded = encode_message(&response).unwrap();
                        stream.write_all(&encoded).unwrap();
                        stream.flush().unwrap();
                    }
                }
            }

            received
        });

        // Small delay
        thread::sleep(Duration::from_millis(50));
        (addr, handle)
    }

    #[test]
    fn test_client_connects() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let _accept_thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            // Handle the ping/pong handshake
            let mut buf = vec![0u8; 65536];
            let n = stream.read(&mut buf).unwrap();
            let (msg, _) = decode_message(&buf[..n]).unwrap().unwrap();
            assert_eq!(msg, Message::Ping);
            let encoded = encode_message(&Message::Pong).unwrap();
            stream.write_all(&encoded).unwrap();
            stream.flush().unwrap();
        });

        thread::sleep(Duration::from_millis(50));

        let script = make_test_script(vec![Directive::Run]);
        let mut presenter = Presenter::new(script, addr);
        assert!(presenter.connect().is_ok());
        assert!(presenter.is_connected());
    }

    #[test]
    fn test_client_sends_execute_receives_ack() {
        let (addr, handle) = start_mock_server(vec![Message::Ack {
            status: AckStatus::Ok,
            message: None,
        }]);

        let script = make_test_script(vec![Directive::Focus("Terminal".into()), Directive::Run]);
        let mut presenter = Presenter::new(script, addr);
        presenter.connect().unwrap();

        let result = presenter.step().unwrap();
        assert_eq!(result, StepResult::Executed);
        assert_eq!(presenter.progress(), (1, 1));

        drop(presenter);
        let received = handle.join().unwrap();
        assert_eq!(received.len(), 1);
    }

    #[test]
    fn test_client_handles_error_ack() {
        let (addr, _handle) = start_mock_server(vec![Message::Ack {
            status: AckStatus::Error,
            message: Some("no accessibility".into()),
        }]);

        let script = make_test_script(vec![Directive::Run]);
        let mut presenter = Presenter::new(script, addr);
        presenter.connect().unwrap();

        let result = presenter.step().unwrap();
        match result {
            StepResult::AgentError(msg) => assert!(msg.contains("no accessibility")),
            other => panic!("Expected AgentError, got {other:?}"),
        }
        // current should NOT advance on error
        assert_eq!(presenter.progress(), (0, 1));
    }

    #[test]
    fn test_client_tracks_block_progress() {
        let responses = vec![
            Message::Ack {
                status: AckStatus::Ok,
                message: None,
            },
            Message::Ack {
                status: AckStatus::Ok,
                message: None,
            },
            Message::Ack {
                status: AckStatus::Ok,
                message: None,
            },
        ];
        let (addr, _handle) = start_mock_server(responses);

        let script = make_test_script(vec![
            Directive::Say("one".into()),
            Directive::Run,
            Directive::Say("two".into()),
            Directive::Run,
            Directive::Say("three".into()),
            Directive::Run,
        ]);
        let mut presenter = Presenter::new(script, addr);
        presenter.connect().unwrap();

        assert_eq!(presenter.progress(), (0, 3));
        presenter.step().unwrap();
        assert_eq!(presenter.progress(), (1, 3));
        presenter.step().unwrap();
        assert_eq!(presenter.progress(), (2, 3));
        presenter.step().unwrap();
        assert_eq!(presenter.progress(), (3, 3));
    }

    #[test]
    fn test_client_narration_only_no_network() {
        // No server needed — narration-only blocks don't use the network
        let script = make_test_script(vec![Directive::Say("hello".into())]);
        let addr: SocketAddr = "127.0.0.1:1".parse().unwrap(); // won't be used
        let mut presenter = Presenter::new(script, addr);
        // Don't connect — narration only shouldn't need it

        let result = presenter.step().unwrap();
        assert_eq!(result, StepResult::NarrationOnly);
    }

    #[test]
    fn test_client_reconnects_after_disconnect() {
        // Helper: read all complete messages from a stream, responding to pings
        // automatically and returning the provided response for Execute messages.
        fn serve_connection(
            stream: &mut TcpStream,
            execute_responses: &mut impl Iterator<Item = Message>,
        ) {
            let mut buf = vec![0u8; 65536];
            let mut pending = Vec::new();
            loop {
                let n = match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };
                pending.extend_from_slice(&buf[..n]);
                while let Some((msg, consumed)) = decode_message(&pending).unwrap() {
                    pending.drain(..consumed);
                    let response = if msg == Message::Ping {
                        Message::Pong
                    } else if let Some(resp) = execute_responses.next() {
                        resp
                    } else {
                        break;
                    };
                    let encoded = encode_message(&response).unwrap();
                    stream.write_all(&encoded).unwrap();
                    stream.flush().unwrap();
                }
            }
        }

        // First server — handles handshake + one Execute, then shuts down
        let listener1 = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener1.local_addr().unwrap();

        let handle1 = thread::spawn(move || {
            let (mut stream, _) = listener1.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut responses = vec![Message::Ack {
                status: AckStatus::Ok,
                message: None,
            }]
            .into_iter();
            serve_connection(&mut stream, &mut responses);
            drop(stream);
            // Return the listener so the port stays bound for reconnection
            listener1
        });

        thread::sleep(Duration::from_millis(50));

        let script = make_test_script(vec![
            Directive::Say("first".into()),
            Directive::Run,
            Directive::Say("second".into()),
            Directive::Run,
        ]);
        let mut presenter = Presenter::new(script, addr);
        presenter.connect().unwrap();

        // First step succeeds
        let result = presenter.step().unwrap();
        assert_eq!(result, StepResult::Executed);

        // Server closes connection, so second step loses connection
        let listener_back = handle1.join().unwrap();

        // Second step should detect connection lost
        let result = presenter.step().unwrap();
        assert_eq!(result, StepResult::ConnectionLost);
        assert!(!presenter.is_connected());

        // Start a new server on the same port — handles handshake + Execute
        let _handle2 = thread::spawn(move || {
            let (mut stream, _) = listener_back.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut responses = vec![Message::Ack {
                status: AckStatus::Ok,
                message: None,
            }]
            .into_iter();
            serve_connection(&mut stream, &mut responses);
        });

        thread::sleep(Duration::from_millis(50));

        // Reconnect (includes ping/pong handshake)
        assert!(presenter.connect().is_ok());
        assert!(presenter.is_connected());

        // Step should work again
        let result = presenter.step().unwrap();
        assert_eq!(result, StepResult::Executed);
    }

    #[test]
    fn test_client_pause_no_network() {
        let script = make_test_script(vec![Directive::Pause(Some(3))]);
        let addr: SocketAddr = "127.0.0.1:1".parse().unwrap();
        let mut presenter = Presenter::new(script, addr);

        let result = presenter.step().unwrap();
        assert_eq!(result, StepResult::Paused(Some(3)));
    }
}
