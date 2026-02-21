pub mod applescript;
pub mod typewriter;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use socket2::{SockRef, TcpKeepalive};

use anyhow::Result;

use crate::parser::types::{Directive, SlideAction};
use crate::protocol::codec::{decode_message, encode_message};
use crate::protocol::messages::{AckStatus, Message};

pub trait ActionExecutor: Send {
    fn execute(&self, actions: &[Directive], typing_speed: u64, typing_variance: u64)
    -> Result<()>;
}

pub struct AppleScriptExecutor;

impl ActionExecutor for AppleScriptExecutor {
    fn execute(
        &self,
        actions: &[Directive],
        typing_speed: u64,
        typing_variance: u64,
    ) -> Result<()> {
        for action in actions {
            match action {
                Directive::Focus(app) => {
                    let script = applescript::focus_app_script(app);
                    applescript::run_applescript(&script)?;
                }
                Directive::Type(text) => {
                    typewriter::execute_typewriter(text, typing_speed, typing_variance)?;
                }
                Directive::Run => {
                    let script = applescript::keystroke_script("return");
                    applescript::run_applescript(&script)?;
                }
                Directive::Slide(slide_action) => {
                    let script = match slide_action {
                        SlideAction::Next => applescript::slide_next_script(),
                        SlideAction::Prev => applescript::slide_prev_script(),
                        SlideAction::GoTo(n) => applescript::slide_goto_script(*n),
                    };
                    applescript::run_applescript(&script)?;
                }
                Directive::Key(combo) => {
                    let script = applescript::keystroke_script(combo);
                    applescript::run_applescript(&script)?;
                }
                Directive::Clear => {
                    let script = applescript::clear_script();
                    applescript::run_applescript(&script)?;
                }
                Directive::Wait(secs) => {
                    thread::sleep(Duration::from_secs(*secs));
                }
                Directive::Exec(cmd) => {
                    std::process::Command::new("sh")
                        .arg("-c")
                        .arg(cmd)
                        .spawn()?;
                }
                // Say, Pause, Section are client-side only
                Directive::Say(_) | Directive::Pause(_) | Directive::Section(_) => {}
            }
        }
        Ok(())
    }
}

pub struct Agent {
    executor: Box<dyn ActionExecutor>,
    port: u16,
}

impl Agent {
    pub fn new(executor: Box<dyn ActionExecutor>, port: u16) -> Self {
        Self { executor, port }
    }

    pub fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(("0.0.0.0", self.port))?;
        println!("Agent listening on 0.0.0.0:{}", self.port);

        loop {
            let (stream, addr) = listener.accept()?;
            println!("Client connected from {addr}");

            if let Err(e) = self.handle_connection(stream) {
                eprintln!("Connection error: {e}");
            }
            println!("Client disconnected. Waiting for new connection...");
        }
    }

    fn handle_connection(&self, mut stream: TcpStream) -> Result<()> {
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(60)))?;

        let sock = SockRef::from(&stream);
        let keepalive = TcpKeepalive::new().with_time(Duration::from_secs(30));
        sock.set_tcp_keepalive(&keepalive)?;

        let mut buf = vec![0u8; 65536];
        let mut pending = Vec::new();
        let mut idle_timeouts: u32 = 0;
        const MAX_IDLE_TIMEOUTS: u32 = 10; // 10 * 60s = 10 minutes max idle

        loop {
            let n = match stream.read(&mut buf) {
                Ok(0) => return Ok(()), // client disconnected
                Ok(n) => {
                    idle_timeouts = 0;
                    n
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock =>
                {
                    idle_timeouts += 1;
                    if idle_timeouts >= MAX_IDLE_TIMEOUTS {
                        eprintln!("Client idle too long, closing connection");
                        return Ok(());
                    }
                    continue;
                }
                Err(e) => return Err(e.into()),
            };

            pending.extend_from_slice(&buf[..n]);

            // Process all complete messages in the buffer
            while let Some((msg, consumed)) = decode_message(&pending)? {
                pending.drain(..consumed);
                let response = self.handle_message(msg);
                let encoded = encode_message(&response)?;
                stream.write_all(&encoded)?;
                stream.flush()?;
            }
        }
    }

    fn handle_message(&self, msg: Message) -> Message {
        match msg {
            Message::Execute {
                actions,
                typing_speed,
                typing_variance,
            } => match self
                .executor
                .execute(&actions, typing_speed, typing_variance)
            {
                Ok(()) => Message::Ack {
                    status: AckStatus::Ok,
                    message: None,
                },
                Err(e) => Message::Ack {
                    status: AckStatus::Error,
                    message: Some(e.to_string()),
                },
            },
            Message::Ping => Message::Pong,
            _ => Message::Ack {
                status: AckStatus::Error,
                message: Some("Unexpected message type".into()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpStream;
    use std::sync::{Arc, Mutex};

    struct MockExecutor {
        calls: Arc<Mutex<Vec<Vec<Directive>>>>,
    }

    impl MockExecutor {
        fn new() -> (Self, Arc<Mutex<Vec<Vec<Directive>>>>) {
            let calls = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    calls: calls.clone(),
                },
                calls,
            )
        }
    }

    impl ActionExecutor for MockExecutor {
        fn execute(
            &self,
            actions: &[Directive],
            _typing_speed: u64,
            _typing_variance: u64,
        ) -> Result<()> {
            self.calls.lock().unwrap().push(actions.to_vec());
            Ok(())
        }
    }

    struct FailingExecutor;

    impl ActionExecutor for FailingExecutor {
        fn execute(
            &self,
            _actions: &[Directive],
            _typing_speed: u64,
            _typing_variance: u64,
        ) -> Result<()> {
            anyhow::bail!("mock failure")
        }
    }

    fn start_agent(executor: Box<dyn ActionExecutor>) -> (u16, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let handle = std::thread::spawn(move || {
            let agent = Agent {
                executor,
                port: 0, // not used, already bound
            };
            // Accept exactly one connection
            if let Ok((stream, _)) = listener.accept() {
                let _ = agent.handle_connection(stream);
            }
        });

        // Small delay to let the listener start
        thread::sleep(Duration::from_millis(50));
        (port, handle)
    }

    #[test]
    fn test_agent_handles_execute() {
        let (executor, calls) = MockExecutor::new();
        let (port, _handle) = start_agent(Box::new(executor));

        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();

        let msg = Message::Execute {
            actions: vec![Directive::Focus("Terminal".into()), Directive::Run],
            typing_speed: 40,
            typing_variance: 15,
        };

        let encoded = encode_message(&msg).unwrap();
        stream.write_all(&encoded).unwrap();
        stream.flush().unwrap();

        // Read response
        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf).unwrap();
        let (response, _) = decode_message(&buf[..n]).unwrap().unwrap();

        assert_eq!(
            response,
            Message::Ack {
                status: AckStatus::Ok,
                message: None,
            }
        );

        let recorded = calls.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded[0],
            vec![Directive::Focus("Terminal".into()), Directive::Run]
        );
    }

    #[test]
    fn test_agent_handles_ping() {
        let (executor, _calls) = MockExecutor::new();
        let (port, _handle) = start_agent(Box::new(executor));

        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();

        let encoded = encode_message(&Message::Ping).unwrap();
        stream.write_all(&encoded).unwrap();
        stream.flush().unwrap();

        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf).unwrap();
        let (response, _) = decode_message(&buf[..n]).unwrap().unwrap();

        assert_eq!(response, Message::Pong);
    }

    #[test]
    fn test_agent_returns_error_on_failure() {
        let (port, _handle) = start_agent(Box::new(FailingExecutor));

        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();

        let msg = Message::Execute {
            actions: vec![Directive::Run],
            typing_speed: 40,
            typing_variance: 15,
        };

        let encoded = encode_message(&msg).unwrap();
        stream.write_all(&encoded).unwrap();
        stream.flush().unwrap();

        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf).unwrap();
        let (response, _) = decode_message(&buf[..n]).unwrap().unwrap();

        match response {
            Message::Ack { status, message } => {
                assert_eq!(status, AckStatus::Error);
                assert!(message.unwrap().contains("mock failure"));
            }
            _ => panic!("Expected Ack"),
        }
    }

    #[test]
    fn test_agent_accepts_reconnect() {
        let (executor, _calls) = MockExecutor::new();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let handle = std::thread::spawn(move || {
            let agent = Agent {
                executor: Box::new(executor),
                port: 0,
            };
            // Accept two connections
            for _ in 0..2 {
                if let Ok((stream, _)) = listener.accept() {
                    let _ = agent.handle_connection(stream);
                }
            }
        });

        thread::sleep(Duration::from_millis(50));

        // First connection
        {
            let stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
            drop(stream); // disconnect immediately
        }

        thread::sleep(Duration::from_millis(50));

        // Second connection — should work
        {
            let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();

            let encoded = encode_message(&Message::Ping).unwrap();
            stream.write_all(&encoded).unwrap();
            stream.flush().unwrap();

            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).unwrap();
            let (response, _) = decode_message(&buf[..n]).unwrap().unwrap();
            assert_eq!(response, Message::Pong);
        }

        drop(handle);
    }
}
