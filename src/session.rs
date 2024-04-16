use std::{
    io::{Read, Write},
    net::SocketAddr,
};

use crate::command::{CommandArgs, CommandReturns};

#[derive(Debug)]
pub struct Session {
    pub id: u16,
    pub username: String,
    pub address: SocketAddr,
    pub socket: std::net::TcpStream,
    pub pwd: String,
}

impl Session {
    pub fn new(socket: std::net::TcpStream, sessions: &[Session]) -> Self {
        let id = if let Some(s) = sessions.last() {
            s.id + 1
        } else {
            0
        };

        let mut socket = socket;
        Self::read_until_term_window_title(&mut socket);

        let address = socket.peer_addr().unwrap();

        let username = Self::enter_command(&mut socket, b"whoami\n")
            .trim()
            .to_string();

        let pwd = Self::enter_command(&mut socket, b"pwd\n")
            .trim()
            .to_string();

        Session {
            id,
            username,
            address,
            socket,
            pwd,
        }
    }

    pub fn update_pwd(&mut self) {
        let pwd = Self::enter_command(&mut self.socket, b"pwd\n")
            .trim()
            .to_string();
        self.pwd = pwd
    }

    pub fn exec_command(&mut self, command: &str) {
        let command = if command.ends_with('\n') {
            command.to_string()
        } else {
            command.to_owned() + "\n"
        };

        self.socket.write_all(command.as_bytes()).unwrap();
        // let _ = Self::read_all()

        loop {
            let mut buf = [0; 1024];
            let len = self.socket.read(&mut buf).unwrap();

            let output = String::from_utf8_lossy(buf.get(..len).unwrap())
                .split("\u{1b}0;")
                .collect::<Vec<&str>>()
                .first()
                .unwrap()
                .to_string();

            // if output == command || output == command.strip_suffix('\n').unwrap() {
                // continue;
            // }

            print!("{:?}", output);
            std::io::stdout().flush().unwrap();

            if len == 1024 {
                continue;
            }
            break;
        }
    }

    fn enter_command(socket: &mut std::net::TcpStream, command: &[u8]) -> String {
        socket.write_all(command).unwrap();

        let output =
            String::from_utf8_lossy(&Self::read_until_term_window_title(socket)).to_string();
        let output_list = output.split('\n').collect::<Vec<&str>>();
        output_list
            .get(1..output_list.len() - 1)
            .unwrap()
            .join("\n")
    }

    fn read_until_term_window_title(socket: &mut std::net::TcpStream) -> Vec<u8> {
        let mut outputs = Vec::new();

        loop {
            let output = Self::read_all(socket);
            outputs.extend(&output);
            for (i, w) in outputs.windows(2).enumerate() {
                if w == [27, 93] {
                    return outputs[..i].to_vec();
                }
            }
        }
    }

    fn read_all(socket: &mut std::net::TcpStream) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        loop {
            let mut buf = [0; 1024];
            let len = socket.read(&mut buf).unwrap();
            buffer.extend(buf.get(..len).unwrap());
            if len == 1024 {
                continue;
            }
            break;
        }
        buffer
    }

    //
    // with output version
    //
    fn enter_command_with_output(socket: &mut std::net::TcpStream, command: &[u8]) -> String {
        socket.write_all(command).unwrap();

        let output =
            String::from_utf8_lossy(&Self::read_until_term_window_title_with_output(socket))
                .to_string();
        let output_list = output.split('\n').collect::<Vec<&str>>();
        output_list
            .get(1..output_list.len() - 1)
            .unwrap()
            .join("\n")
    }

    fn read_until_term_window_title_with_output(socket: &mut std::net::TcpStream) -> Vec<u8> {
        let mut outputs = Vec::new();

        loop {
            let output = Self::read_all_with_output(socket);
            outputs.extend(&output);
            for (i, w) in outputs.windows(2).enumerate() {
                if w == [27, 93] {
                    return outputs[..i].to_vec();
                }
            }
        }
    }

    fn read_all_with_output(socket: &mut std::net::TcpStream) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        loop {
            let mut buf = [0; 1024];
            let len = socket.read(&mut buf).unwrap();

            buffer.extend(buf.get(..len).unwrap());
            let output = String::from_utf8_lossy(buf.get(..len).unwrap())
                .split("\u{1b}0;")
                .collect::<Vec<&str>>()
                .first()
                .unwrap()
                .to_string();

            print!("{}", output);
            std::io::stdout().flush().unwrap();

            if len == 1024 {
                continue;
            }
            break;
        }
        buffer
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
    if args.args.is_empty() {
        // print all sessions available as a table;
        use cli_table::{format::Justify, Cell, Style, Table};
        let mut vector = vec![];
        args.app_state.sessions.iter().for_each(|s| {
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

        println!("{}", table.display().unwrap());
        return CommandReturns::new(true, args.app_state);
    }
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
        return CommandReturns::new(true, args.app_state);
    }
    use crate::ShellContext;
    let id = args.args[0].parse::<u16>().unwrap();
    if let Some(session) = args.app_state.sessions.iter().find(|s| s.id == id) {
        let session = session.clone();
        let mut new_app_state = args.app_state;
        new_app_state.current_shell_ctx = ShellContext::Remote;
        new_app_state.current_session = session;
        return CommandReturns::new(true, new_app_state);
    }

    CommandReturns::new(true, args.app_state)
}
