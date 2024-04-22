use std::{
    io::{BufRead, BufReader, Read, Write},
    net::SocketAddr,
    sync::Mutex,
};

use log::info;

use crate::command::{CommandArgs, CommandReturns};

pub static SESSIONS: once_cell::sync::Lazy<Mutex<Vec<Session>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(vec![]));

static LARGEST_ID: once_cell::sync::Lazy<Mutex<u16>> = once_cell::sync::Lazy::new(|| Mutex::new(0));

#[derive(Debug)]
pub struct Session {
    pub id: u16,
    pub username: String,
    pub address: SocketAddr,
    pub socket: std::net::TcpStream,
    pub pwd: String,
}

impl Session {
    pub fn new(socket: std::net::TcpStream) -> Self {
        let id = {
            let largest_id = *LARGEST_ID.lock().unwrap();
            if largest_id == 0 {
                0
            } else {
                largest_id + 1
            }
        };

        let mut socket = socket;

        let address = socket.peer_addr().unwrap();
        let username = "unknown".to_string();
        let pwd = "unknown".to_string();

        Session {
            id,
            username,
            address,
            socket,
            pwd,
        }
    }

    pub fn init(&mut self) {
        self.read_until("\u{1b}]0;".as_bytes());

        let username = self.exec_command("whoami").trim().to_string();
        self.username = username;
        info!("username = {}", self.username);

        let pwd = self.exec_command("pwd").trim().to_string();
        self.pwd = pwd;
        info!("current directory = {}", self.pwd);
    }

    pub fn update_pwd(&mut self) {
        let pwd = self.exec_command("pwd").trim().to_string();
        self.pwd = pwd
    }

    pub fn exec_command_with_pretty_output(&mut self, command: &str) {
        let command = if command.ends_with('\n') {
            command.to_string()
        } else {
            command.to_owned() + "\n"
        };

        self.socket.write_all(command.as_bytes()).unwrap();

        let data = self.read_until("\u{1b}]0;".as_bytes());
        let output = String::from_utf8(data).unwrap();
        let (_, result) = output.split_once(&command).unwrap();
        println!("{}", result);
    }

    fn exec_command(&mut self, command: &str) -> String {
        let command = if command.ends_with('\n') {
            command.to_string()
        } else {
            command.to_owned() + "\n"
        };

        self.socket.write_all(command.as_bytes()).unwrap();

        let data = self.read_until("\u{1b}]0;".as_bytes());
        let output = String::from_utf8(data).unwrap();
        let (_, result) = output.split_once(&command).unwrap();
        result.to_string()
    }

    // 超効率悪いけどとりあえず動く
    fn read_until(&mut self, delim: &[u8]) -> Vec<u8> {
        let mut data = vec![];
        let mut reader = BufReader::new(self.socket.try_clone().unwrap());
        loop {
            let mut buf = [0; 1];
            reader.read_exact(&mut buf).unwrap();
            data.push(buf[0]);

            for (i, w) in data.windows(delim.len()).enumerate() {
                if w == delim {
                    return data[..i].to_vec();
                }
            }
        }
    }

    fn update(&mut self, s: Session) {
        *self = s;
    }
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            username: self.username.clone(),
            address: self.address,
            socket: self.socket.try_clone().unwrap(),
            pwd: self.pwd.clone(),
        }
    }
}

pub fn sessions(args: CommandArgs) -> CommandReturns {
    let sessions = SESSIONS.lock().unwrap();

    // print session list
    if args.args.is_empty() {
        // print all sessions available as a table;
        let s = &sessions.iter().filter(|_| true).collect::<Vec<_>>();
        let table = make_session_table(s);

        println!("{}", table);
        return CommandReturns::new(true, args.manager);
    }

    // print help message
    if args.args.len() == 1 && args.args[0] == "help" || args.args.len() > 2 {
        // print help message
        use crate::util::tidy_usage;
        println!("Usage:");
        println!(
            "  {}",
            tidy_usage("sessions", "List all sessions available")
        );
        println!(
            "  {}",
            tidy_usage(
                "sessions <id>",
                "Switch current shell context to a remote session with the given id"
            )
        );
        return CommandReturns::new(true, args.manager);
    }

    // change shell local to remote
    let id = args.args[0].parse::<u16>().unwrap();
    if let Some(session) = sessions.iter().find(|s| s.id == id) {
        let session = session.clone();
        let mut manager = args.manager;
        manager.current_session_id = Some(id);
        manager.is_shell_remote = true;
        return CommandReturns::new(true, manager);
    }

    CommandReturns::new(true, args.manager)
}

/// push a new session to the global SESSIONS
pub fn push_session(session: Session) {
    let mut sessions = SESSIONS.lock().unwrap();
    sessions.push(session);
}

/// search for a session with the given id from the global SESSIONS
pub fn get_session(id: Option<u16>) -> Option<Session> {
    let id = if id.is_none() {
        return None;
    } else {
        id.unwrap()
    };

    let sessions = SESSIONS.lock().unwrap();
    let session = sessions.iter().filter(|s| s.id == id).collect::<Vec<_>>();

    match session.len() {
        0 => None,
        1 => Some(session[0].clone()),
        _ => {
            let table = make_session_table(&session);

            panic!("Multiple sessions with the same id: {}", table);
        }
    }
}

/// Overwrite a session with the given id from the global SESSIONS with the given one
pub fn set_session(id: u16, session: &Session) {
    let mut sessions = SESSIONS.lock().unwrap();
    sessions.iter_mut().for_each(|s| {
        if s.id == id {
            s.update(session.clone());
        }
    });
}

/// Make a table(string) of sessions
pub fn make_session_table(sessions: &[&Session]) -> String {
    use cli_table::{format::Justify, Cell, Style, Table};
    let mut vector = vec![];
    sessions.iter().for_each(|s| {
        vector.push(vec![
            s.id.to_string().cell().justify(Justify::Right),
            s.username.clone().cell().justify(Justify::Left),
            s.address.to_string().cell().justify(Justify::Right),
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
