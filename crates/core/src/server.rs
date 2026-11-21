use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use crate::proto::{Command, Config, DISCOVER, hello};

/// PC side. Lives for the whole app; dropping it does not stop the threads.
pub struct Server {
    status: Arc<ServerStatus>,
    control_port: u16,
}

/// What the desktop window needs to render: connected or not, last key.
pub struct ServerStatus {
    clients: AtomicUsize,
    last: Mutex<Option<(Command, Instant)>>,
}

impl ServerStatus {
    pub fn clients(&self) -> usize {
        self.clients.load(Ordering::Relaxed)
    }

    pub fn online(&self) -> bool {
        self.clients() > 0
    }

    /// Last command and when it arrived, for the desktop flash.
    pub fn last(&self) -> Option<(Command, Instant)> {
        *self.last.lock().unwrap()
    }
}

impl Server {
    /// Binds the control and discovery ports and starts listening.
    ///
    /// `control_port == 0` picks a free port; [`Server::control_port`] reports it.
    pub fn start(
        config: Config,
        on_command: impl Fn(Command) + Send + Sync + 'static,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("0.0.0.0", config.control_port))?;
        let control_port = listener.local_addr()?.port();

        let udp = UdpSocket::bind(("0.0.0.0", config.discovery_port))?;
        udp.set_broadcast(true)?;

        let status = Arc::new(ServerStatus {
            clients: AtomicUsize::new(0),
            last: Mutex::new(None),
        });

        thread::spawn(move || answer_discovery(udp, control_port));

        let on_command = Arc::new(on_command);
        let accept_status = status.clone();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let status = accept_status.clone();
                let on_command = on_command.clone();
                thread::spawn(move || serve_client(stream, status, on_command));
            }
        });

        Ok(Self {
            status,
            control_port,
        })
    }

    pub fn status(&self) -> &Arc<ServerStatus> {
        &self.status
    }

    pub fn control_port(&self) -> u16 {
        self.control_port
    }
}

fn answer_discovery(socket: UdpSocket, control_port: u16) {
    let reply = hello(control_port);
    let mut buf = [0u8; 64];
    while let Ok((len, from)) = socket.recv_from(&mut buf) {
        if &buf[..len] == DISCOVER {
            let _ = socket.send_to(reply.as_bytes(), from);
        }
    }
}

fn serve_client(
    stream: TcpStream,
    status: Arc<ServerStatus>,
    on_command: Arc<dyn Fn(Command) + Send + Sync>,
) {
    status.clients.fetch_add(1, Ordering::Relaxed);
    let _online = Online(status.clone());
    let _ = stream.set_nodelay(true);

    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else {
            break;
        };
        if let Some(command) = Command::parse(&line) {
            *status.last.lock().unwrap() = Some((command, Instant::now()));
            on_command(command);
        }
    }
}

struct Online(Arc<ServerStatus>);

impl Drop for Online {
    fn drop(&mut self) {
        self.0.clients.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::SocketAddr;
    use std::sync::mpsc;
    use std::time::Duration;

    fn free_discovery_port() -> u16 {
        42000 + (std::process::id() % 2000) as u16
    }

    #[test]
    fn commands_reach_the_handler() {
        let (tx, rx) = mpsc::channel();
        let server = Server::start(
            Config {
                discovery_port: free_discovery_port(),
                control_port: 0,
            },
            move |command| {
                let _ = tx.send(command);
            },
        )
        .unwrap();

        let addr = SocketAddr::from(([127, 0, 0, 1], server.control_port()));
        let mut stream = TcpStream::connect(addr).unwrap();
        stream.write_all(b"next\ndislike\n").unwrap();

        assert_eq!(
            rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            Command::Next
        );
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            Command::Dislike
        );
        assert_eq!(
            server.status().last().map(|(command, _)| command),
            Some(Command::Dislike)
        );
    }
}
