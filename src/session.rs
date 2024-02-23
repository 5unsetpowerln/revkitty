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
    pub ignored_prompt: Vec<u8>,
}

impl Session {
    pub fn new(socket: std::net::TcpStream, sessions: &[Session]) -> Self {
        use std::io::BufReader;
        use std::io::BufWriter;
        let id = if let Some(s) = sessions.last() {
            s.id + 1
        } else {
            0
        };

        let mut socket = socket;
        let mut username = "unknown".to_string();
        let address = socket.peer_addr().unwrap();
        let mut ignored_prompt = Vec::new();

        let ignored_part = Self::read_all_without_output(&mut socket);
        ignored_prompt.extend(ignored_part);
        let ignored_part = Self::read_all_without_output(&mut socket);
        ignored_prompt.extend(ignored_part);

        // Self::init_socket(&mut socket);
        username = Self::enter_command_without_output(&mut socket, b"whoami\n", &ignored_prompt)
            .trim_end()
            .to_string();

        Session {
            id,
            username,
            address,
            socket,
            ignored_prompt,
        }
    }

    // execute command with output
    pub fn exec_command(&mut self, command: &str) {
        let command = if command.ends_with('\n') {
            command.to_string()
        } else {
            command.to_owned() + "\n"
        };

        Self::enter_command_without_output(
            &mut self.socket,
            command.as_bytes(),
            &self.ignored_prompt,
        );
    }

    fn enter_command_without_output(
        socket: &mut std::net::TcpStream,
        command: &[u8],
        ignored_prompt: &[u8],
    ) -> String {
        socket.write_all(command).unwrap();

        let mut outputs = Vec::new();

        loop {
            let output = Self::read_all_without_output(socket);
            outputs.extend(&output);
            for (i, win) in output.windows(ignored_prompt.len()).enumerate() {
                println!("{:?}", win);
                if win == ignored_prompt {
                    return String::from_utf8_lossy(
                        outputs.get(..outputs.len() - ignored_prompt.len()).unwrap(),
                    )
                    .to_string();
                }
            }
        }
    }

    fn read_all_without_output(socket: &mut std::net::TcpStream) -> Vec<u8> {
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
        // println!("{:?}", buffer);
        buffer
    }

    fn enter_command(socket: &mut std::net::TcpStream, command: &[u8]) {
        socket.write_all(command).unwrap();

        // command name
        let _ = Self::read_all_without_output(socket);
        // output
        let _ = Self::read_all_with_output(socket);
        // terminal window
        let _ = Self::read_all_without_output(socket);
        // prompt
        let _ = Self::read_all_without_output(socket);
    }

    fn read_all_with_output(socket: &mut std::net::TcpStream) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        loop {
            let mut buf = [0; 512];
            let len = socket.read(&mut buf).unwrap();
            let new_buf = buf.get(..len).unwrap();
            println!("{}", String::from_utf8_lossy(new_buf));
            buffer.extend(new_buf);
            if len == 1024 {
                continue;
            }
            break;
        }
        println!("{:?}", buffer);
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
            ignored_prompt: self.ignored_prompt.clone(),
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
