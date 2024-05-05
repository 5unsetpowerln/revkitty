use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr},
    sync::Mutex,
};

use anyhow::Result;
use log::info;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::{TcpListener, TcpStream},
};

// このモジュール以外から直接アクセスできないようにして、デッドロックを防止する
static SESSIONS_ARRAY: once_cell::sync::Lazy<Mutex<Vec<Session>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(vec![]));

static LARGEST_SESSION_ID: once_cell::sync::Lazy<Mutex<u16>> =
    once_cell::sync::Lazy::new(|| Mutex::new(0));

#[derive(Debug)]
pub struct Socket {
    address: SocketAddr,
    reader: ReadHalf<TcpStream>,
    writer: WriteHalf<TcpStream>,
}

impl Socket {
    async fn new(port: u16) -> Self {
        let address = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), port);
        let listener = TcpListener::bind(address).await.unwrap();
        let (socket, remote_address) = listener.accept().await.unwrap();
        let (reader, writer) = tokio::io::split(socket);
        Self {
            address: remote_address,
            reader,
            writer,
        }
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data.to_vec().as_slice()).await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn sendline(&mut self, data: &[u8]) -> Result<()> {
        self.send(data).await?;
        self.send(b"\n").await?;
        Ok(())
    }

    async fn recvuntil(&mut self, pattern: &[u8]) -> Result<Vec<u8>> {
        let mut buf = vec![];
        loop {
            let mut buf_ = [0];
            self.reader.read_exact(&mut buf_).await?;
            buf.extend_from_slice(&buf_[..]);
            if buf.ends_with(pattern) {
                break;
            }
        }
        Ok(buf)
    }

    async fn printuntil(&mut self, pattern: &[u8]) -> Result<()> {
        let mut buf = vec![];
        let mut last_line_index = 0;
        loop {
            let mut buf_ = [0];
            self.reader.read_exact(&mut buf_).await?;

            buf.extend_from_slice(&buf_[..]);

            if buf_ == [0x0A] {
                println!(
                    "{}",
                    String::from_utf8(buf[last_line_index..buf.len()].to_vec())
                        .unwrap()
                        .strip_suffix("\n")
                        .unwrap()
                );
                last_line_index = buf.len();
            }

            if buf.ends_with(pattern) {
                println!(
                    "{}",
                    String::from_utf8(buf[last_line_index..buf.len()].to_vec()).unwrap()
                );
                return Ok(());
            }
        }
    }

    /// receive until \n and strip \n
    async fn recvline(&mut self) -> Result<Vec<u8>> {
        match self.recvuntil(b"\n").await {
            Ok(l) => Ok(l.strip_suffix(b"\n").unwrap().to_vec()),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug)]
pub struct Session {
    pub metadata: SessionMetadata,
    // pub id: u16,
    // pub username: String,
    // pub address: SocketAddr,
    pub socket: Socket,
    // pub cwd: String,
}

#[derive(Debug, Clone)]
pub struct SessionMetadata {
    pub id: u16,
    pub username: String,
    pub address: SocketAddr,
    pub cwd: String,
}

impl Session {
    pub async fn new(port: u16) -> Self {
        let socket = Socket::new(port).await;
        let address = socket.address;
        let username = "unknown".to_string();
        let cwd = "unknown".to_string();

        let largest_session_id = LARGEST_SESSION_ID.lock().unwrap();
        let id = if *largest_session_id == 0 {
            0
        } else {
            *largest_session_id + 1
        };

        let metadata = SessionMetadata {
            id,
            username,
            address,
            cwd,
        };

        Session { metadata, socket }
    }

    pub async fn init(&mut self) {
        self.socket.recvuntil("\u{1b}]0;".as_bytes()).await.unwrap();

        self.socket.sendline(b"whoami").await.unwrap();
        self.socket.recvuntil(b"whoami\n").await.unwrap();
        let username = self.socket.recvline().await.unwrap();
        let username = String::from_utf8(username).unwrap();
        self.metadata.username = username;
        info!("username: {}", self.metadata.username);

        self.socket.sendline(b"pwd").await.unwrap();
        self.socket.recvuntil(b"pwd\n").await.unwrap();
        let cwd = self.socket.recvline().await.unwrap();
        let cwd = String::from_utf8(cwd).unwrap();
        self.metadata.cwd = cwd;
        info!("current directory = {}", self.metadata.cwd);
    }

    async fn execute_command(&mut self, command: &[u8]) -> Vec<u8> {
        let command = if command.ends_with(b"\n") {
            &command[0..command.len() - 1]
        } else {
            command
        };

        // execute command
        self.socket.sendline(command).await.unwrap();
        self.socket.recvuntil(command).await.unwrap();
        self.socket.recvuntil(b"\n").await.unwrap();
        let output = self.socket.recvuntil("\u{1b}]0;".as_bytes()).await.unwrap();

        // update cwd
        self.socket.sendline(b"pwd").await.unwrap();
        self.socket.recvuntil(b"pwd\n").await.unwrap();
        let cwd = self.socket.recvline().await.unwrap();
        let cwd = String::from_utf8(cwd).unwrap();
        self.metadata.cwd = cwd;

        output
    }

    pub async fn execute_command_prettily(&mut self, command: &[u8]) {
        let command = if command.ends_with(b"\n") {
            &command[0..command.len() - 1]
        } else {
            command
        };

        // execute command
        self.socket.sendline(command).await.unwrap();
        self.socket.recvuntil(command).await.unwrap();
        self.socket.recvuntil(b"\n").await.unwrap();
        self.socket
            .printuntil("\u{1b}]0;".as_bytes())
            .await
            .unwrap();

        // update cwd
        self.socket.sendline(b"pwd").await.unwrap();
        self.socket.recvuntil(b"pwd\n").await.unwrap();
        let cwd = self.socket.recvline().await.unwrap();
        println!("{:?}", cwd);
        let cwd = String::from_utf8(cwd).unwrap();
        self.metadata.cwd = cwd;
    }
}

pub async fn new_session(port: u16) -> u16 {
    let mut sessions = SESSIONS_ARRAY.lock().unwrap();

    let mut session = Session::new(port).await;
    let id = session.metadata.id;
    session.init().await;
    sessions.push(session);
    id
}

pub fn get_metadata(id: u16) -> SessionMetadata {
    let sessions = SESSIONS_ARRAY.lock().unwrap();
    let session = sessions.iter().find(|x| x.metadata.id == id).unwrap();
    session.metadata.clone()
}

/// DON"T USE THIS FUNCTION FROM INSIDE MODULE!!
pub async fn execute_command_prettily(id: u16, command: &[u8]) {
    let mut sessions = SESSIONS_ARRAY.lock().unwrap();
    let session = sessions.iter_mut().find(|x| x.metadata.id == id).unwrap();
    session.execute_command_prettily(command).await;
}

/// DON"T USE THIS FUNCTION FROM INSIDE MODULE!!
pub async fn execute_command(id: u16, command: &[u8]) -> Vec<u8> {
    let mut sessions = SESSIONS_ARRAY.lock().unwrap();
    let session = sessions.iter_mut().find(|x| x.metadata.id == id).unwrap();
    session.execute_command(command).await
}

pub fn is_session_exist(id: u16) -> bool {
    let mut sessions = SESSIONS_ARRAY.lock().unwrap();
    let mut result = false;
    sessions.iter().for_each(|s| {
        if s.metadata.id == id {
            result = true;
        }
    });
    result
}

/// Make a table(string) of sessions
pub fn make_session_table() -> String {
    let mut sessions = SESSIONS_ARRAY.lock().unwrap();
    use cli_table::{format::Justify, Cell, Style, Table};
    let mut vector = vec![];
    sessions.iter().for_each(|s| {
        vector.push(vec![
            s.metadata.id.to_string().cell().justify(Justify::Right),
            s.metadata.username.clone().cell().justify(Justify::Left),
            s.metadata
                .address
                .to_string()
                .cell()
                .justify(Justify::Right),
        ]);
    });
    let table = vector
        .table()
        .title(vec![
            "id".cell().bold(true),
            "username".cell().bold(true),
            "address".cell().bold(true),
        ])
        .bold(true);

    table.display().unwrap().to_string()
}
