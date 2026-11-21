use std::io::Write;
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicU16, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use mote_core::{Client, Command, Config, Server};

/// Give every test its own discovery port so parallel tests don't collide.
fn test_config() -> Config {
    static NEXT: AtomicU16 = AtomicU16::new(0);
    Config {
        discovery_port: 44000
            + (std::process::id() % 1000) as u16
            + NEXT.fetch_add(1, Ordering::Relaxed),
        control_port: 0,
    }
}

#[test]
fn discovery_reveals_the_control_port() {
    let config = test_config();
    let server = Server::start(config, |_| {}).unwrap();

    let socket = UdpSocket::bind(("127.0.0.1", 0)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    socket
        .send_to(b"mote?", ("127.0.0.1", config.discovery_port))
        .unwrap();

    let mut buf = [0u8; 64];
    let (len, _) = socket.recv_from(&mut buf).unwrap();
    let reply = std::str::from_utf8(&buf[..len]).unwrap().trim().to_owned();
    assert_eq!(reply, format!("mote {}", server.control_port()));
}

#[test]
fn raw_tcp_client_can_drive_the_server() {
    let (tx, rx) = std::sync::mpsc::channel();
    let server = Server::start(test_config(), move |command| {
        let _ = tx.send(command);
    })
    .unwrap();

    let addr = SocketAddr::from(([127, 0, 0, 1], server.control_port()));
    let mut stream = TcpStream::connect(addr).unwrap();
    stream.write_all(b"author\n").unwrap();

    assert_eq!(
        rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        Command::Author
    );
    assert!(server.status().online());
    assert_eq!(
        server.status().last().map(|(command, _)| command),
        Some(Command::Author)
    );
}

#[test]
fn client_discovers_and_sends() {
    let config = test_config();
    let (tx, rx) = std::sync::mpsc::channel();
    let _server = Server::start(config, move |command| {
        let _ = tx.send(command);
    })
    .unwrap();

    let client = Client::start(config);
    client.send(Command::Next);

    assert_eq!(
        rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        Command::Next
    );

    let deadline = Instant::now() + Duration::from_secs(2);
    while !client.status().online() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(client.status().online());
}
