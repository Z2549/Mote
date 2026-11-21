use std::io::Write;
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use crate::proto::{CONNECT_TIMEOUT, Command, Config, DISCOVER, DISCOVER_TIMEOUT, parse_hello};

/// Phone side. Sending is non-blocking: a background thread owns the socket,
/// discovers the PC, reconnects on failure and retries the key once.
pub struct Client {
    tx: Sender<Command>,
    status: Arc<ClientStatus>,
}

pub struct ClientStatus {
    online: AtomicBool,
}

impl ClientStatus {
    pub fn online(&self) -> bool {
        self.online.load(Ordering::Relaxed)
    }
}

impl Client {
    pub fn start(config: Config) -> Self {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(ClientStatus {
            online: AtomicBool::new(false),
        });
        let thread_status = status.clone();
        thread::spawn(move || run(config, rx, thread_status));
        Self { tx, status }
    }

    pub fn send(&self, command: Command) {
        let _ = self.tx.send(command);
    }

    pub fn status(&self) -> &Arc<ClientStatus> {
        &self.status
    }
}

fn run(config: Config, rx: Receiver<Command>, status: Arc<ClientStatus>) {
    let mut conn: Option<TcpStream> = None;

    loop {
        if conn.is_none() {
            conn = connect(&config);
            status.online.store(conn.is_some(), Ordering::Relaxed);
        }

        match rx.recv_timeout(Duration::from_millis(400)) {
            Ok(command) => {
                let delivered = deliver(&mut conn, &config, command);
                status.online.store(delivered, Ordering::Relaxed);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

/// Writes one command. On a dead socket: reconnect once, send once.
fn deliver(conn: &mut Option<TcpStream>, config: &Config, command: Command) -> bool {
    if conn.is_none() {
        *conn = connect(config);
    }

    if let Some(stream) = conn.as_mut()
        && stream.write_all(command.wire()).is_ok()
    {
        return true;
    }

    *conn = connect(config);
    conn.as_mut()
        .is_some_and(|stream| stream.write_all(command.wire()).is_ok())
}

fn connect(config: &Config) -> Option<TcpStream> {
    let addr = discover(config)?;
    let stream = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).ok()?;
    stream.set_nodelay(true).ok()?;
    Some(stream)
}

fn discover(config: &Config) -> Option<SocketAddr> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    socket.set_broadcast(true).ok()?;
    socket.set_read_timeout(Some(DISCOVER_TIMEOUT)).ok()?;
    for target in [
        SocketAddr::from(([255, 255, 255, 255], config.discovery_port)),
        SocketAddr::from(([127, 0, 0, 1], config.discovery_port)),
    ] {
        let _ = socket.send_to(DISCOVER, target);
    }

    let mut buf = [0u8; 64];
    loop {
        let (len, from) = socket.recv_from(&mut buf).ok()?;
        let Some(port) = parse_hello(&buf[..len]) else {
            continue;
        };
        return Some(SocketAddr::new(from.ip(), port));
    }
}
