use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr},
    sync::Mutex,
};

use anyhow::{anyhow, Context, Result};
use log::{error, info};
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
    async fn new(port: u16) -> Result<Self> {
        let address = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), port);
        let listener = TcpListener::bind(address).await?;
        let (socket, remote_address) = listener.accept().await?;
        let (reader, writer) = tokio::io::split(socket);
        Ok(Self {
            address: remote_address,
            reader,
            writer,
        })
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

    async fn printuntil(&mut self, pattern: &[u8], print_pattern: bool) -> Result<()> {
        let mut buf = vec![];
        let mut last_line_index = 0;
        loop {
            let mut buf_ = [0];
            self.reader.read_exact(&mut buf_).await?;

            buf.extend_from_slice(&buf_[..]);

            if buf_ == [0x0A] {
                match String::from_utf8(buf[last_line_index..buf.len()].to_vec()) {
                    Ok(output) => match output.strip_suffix('\n') {
                        Some(l) => println!("{}", l),
                        None => println!("{}", output),
                    },
                    Err(e) => {
                        error!("utf8 error: {}", e.to_string());
                    }
                }
                last_line_index = buf.len();
            }

            if buf.ends_with(pattern) {
                if print_pattern {
                    match String::from_utf8(buf[last_line_index..buf.len()].to_vec()) {
                        Ok(output) => match output.strip_suffix('\n') {
                            Some(l) => println!("{}", l),
                            None => println!("{}", output),
                        },
                        Err(e) => {
                            error!("utf8 error: {}", e.to_string());
                        }
                    }
                } else {
                    match String::from_utf8(
                        buf[last_line_index..buf.len() - pattern.len()].to_vec(),
                    ) {
                        Ok(output) => match output.strip_suffix('\n') {
                            Some(l) => println!("{}", l),
                            None => println!("{}", output),
                        },
                        Err(e) => {
                            error!("utf8 error: {}", e.to_string());
                        }
                    }
                }
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
    pub socket: Socket,
}

#[derive(Debug, Clone)]
pub struct SessionMetadata {
    pub id: u16,
    pub username: String,
    pub address: SocketAddr,
    pub cwd: String,
}

impl Session {
    pub async fn new(port: u16) -> Result<Self> {
        let socket = Socket::new(port)
            .await
            .context("failed to create new socket")?;
        let address = socket.address;
        let username = "unknown".to_string();
        let cwd = "unknown".to_string();

        info!("connection from: {}", address);

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

        Ok(Session { metadata, socket })
    }

    pub async fn init(&mut self) -> Result<()> {
        // recv terminal window
        self.socket
            .recvuntil("\u{1b}]0;".as_bytes())
            .await
            .context("failed to recv a terminal window")?;

        // send whoami
        self.socket
            .sendline(b"whoami")
            .await
            .context("failed to send \"whoami\"")?;

        // recv terminal window
        self.socket
            .recvuntil(b"whoami\n")
            .await
            .context("failed to recv terminal window")?;

        // recv username
        let username = self
            .socket
            .recvline()
            .await
            .context("failed to recv username")?;

        // parse username as utf-8
        let username =
            String::from_utf8(username.clone()).context("failed to parse username as utf-8")?;

        // update username
        self.metadata.username = username;
        info!("username: {}", self.metadata.username);

        // send pwd
        self.socket
            .sendline(b"pwd")
            .await
            .context("failed to send \"pwd\"")?;

        // recv terminal window
        self.socket
            .recvuntil(b"pwd\n")
            .await
            .context("failed to recv terminal window")?;

        // recv cwd
        let cwd = self.socket.recvline().await.context("failed to recv cwd")?;

        // parse cwd as utf-8
        let cwd = String::from_utf8(cwd.clone()).context("failed to parse cwd as utf-8")?;

        // update cwd
        self.metadata.cwd = cwd;
        info!("cwd: {}", self.metadata.cwd);

        Ok(())
    }

    async fn execute_command(&mut self, command: &[u8]) -> Result<Vec<u8>> {
        let command = if command.ends_with(b"\n") {
            &command[0..command.len() - 1]
        } else {
            command
        };

        // execute command
        self.socket
            .sendline(command)
            .await
            .context("failed to send the command")?;

        // recieve terminal window
        self.socket
            .recvuntil(command)
            .await
            .context("failed to recv a terminal window")?;
        self.socket
            .recvuntil(b"\n")
            .await
            .context("failed to recv a terminal window")?;

        // recieve output
        let output = self
            .socket
            .recvuntil("\u{1b}]0;".as_bytes())
            .await
            .context("failed to recv un output")?;

        // update cwd
        // send pwd
        self.socket
            .sendline(b"pwd")
            .await
            .context("failed to send \"pwd\"")?;

        // recieve terminal window
        self.socket
            .recvuntil(b"pwd\n")
            .await
            .context("failed to recv a terminal window")?;

        // recieve an output
        let cwd = self
            .socket
            .recvline()
            .await
            .context("failed to crecv a terminal window")?;

        let cwd = String::from_utf8(cwd).context("failed to parse cwd as utf-8")?;
        self.metadata.cwd = cwd;

        Ok(output)
    }

    pub async fn execute_command_prettily(&mut self, command: &[u8]) -> Result<()> {
        let command = if command.ends_with(b"\n") {
            &command[0..command.len() - 1]
        } else {
            command
        };

        // send command
        self.socket
            .sendline(command)
            .await
            .context("failed to send the command")?;

        // recv terminal window
        self.socket
            .recvuntil(command)
            .await
            .context("failed to recv a terminal window")?;
        self.socket
            .recvuntil(b"\n")
            .await
            .context("failed to recv a terminal window")?;

        // recv and print output line by line
        self.socket
            .printuntil("\u{1b}]0;".as_bytes(), false)
            .await
            .context("failed to finish to recv and print an output line by line")?;

        // send pwd
        self.socket
            .sendline(b"pwd")
            .await
            .context("failed to send \"pwd\"")?;

        // recv terminal window
        self.socket
            .recvuntil(b"pwd\n")
            .await
            .context("failed to recv a terminal window")?;

        // recv cwd
        let cwd = self.socket.recvline().await.context("failed to recv cwd")?;

        // parse cwd as utf-8
        let cwd = String::from_utf8(cwd).context("failed to parse cwd as utf-8")?;

        // update cwd
        self.metadata.cwd = cwd;

        Ok(())
    }
}

pub async fn new_session(port: u16) -> Result<u16> {
    let mut sessions = match SESSIONS_ARRAY.lock() {
        Ok(s) => s,
        Err(e) => return Err(anyhow!(e.to_string())),
    };

    let mut session = Session::new(port)
        .await
        .context("failed to create a new session")?;
    let id = session.metadata.id;
    session
        .init()
        .await
        .context("failed to init the new session")?;
    sessions.push(session);
    Ok(id)
}

pub fn get_metadata(id: u16) -> Result<SessionMetadata> {
    let sessions = match SESSIONS_ARRAY.lock() {
        Ok(s) => s,
        Err(e) => return Err(anyhow!(e.to_string())),
    };
    let session = match sessions.iter().find(|x| x.metadata.id == id) {
        Some(s) => s,
        None => return Err(anyhow!("session with id {} not found", id)),
    };
    Ok(session.metadata.clone())
}

/// DON"T USE THIS FUNCTION FROM INSIDE MODULE!!
pub async fn execute_command_prettily(id: u16, command: &[u8]) -> Result<()> {
    let mut sessions = match SESSIONS_ARRAY.lock() {
        Ok(s) => s,
        Err(e) => return Err(anyhow!(e.to_string())),
    };
    let session = match sessions.iter_mut().find(|x| x.metadata.id == id) {
        Some(s) => s,
        None => return Err(anyhow!("session with id {} not found", id)),
    };
    session
        .execute_command_prettily(command)
        .await
        .context("failed to execute command prettily")?;
    Ok(())
}

/// DON"T USE THIS FUNCTION FROM INSIDE MODULE!!
pub async fn execute_command(id: u16, command: &[u8]) -> Result<Vec<u8>> {
    let mut sessions = match SESSIONS_ARRAY.lock() {
        Ok(s) => s,
        Err(e) => return Err(anyhow!(e.to_string())),
    };
    let session = match sessions.iter_mut().find(|x| x.metadata.id == id) {
        Some(s) => s,
        None => return Err(anyhow!("session with id {} not found", id)),
    };
    session
        .execute_command(command)
        .await
        .context("failed to execute command")
}

pub fn is_session_exist(id: u16) -> Result<bool> {
    let sessions = match SESSIONS_ARRAY.lock() {
        Ok(s) => s,
        Err(e) => return Err(anyhow!(e.to_string())),
    };
    Ok(sessions.iter().any(|x| x.metadata.id == id))
}

/// Make a table(string) of sessions
pub fn make_session_table() -> Result<String> {
    let sessions = match SESSIONS_ARRAY.lock() {
        Ok(s) => s,
        Err(e) => return Err(anyhow!(e.to_string())),
    };
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

    Ok(table.display().unwrap().to_string())
}
