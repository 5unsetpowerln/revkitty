mod command;
mod session;
mod util;

use command::{Command, CommandArgs, CommandReturns};
use session::Session;

fn main() {
    let commands = get_commands();

    let port = std::env::args()
        .collect::<Vec<String>>()
        .get(1)
        .unwrap()
        .parse::<u16>()
        .unwrap();
    let initial_session = initial_listen(port);

    let mut app_state = AppState::new(initial_session);

    let mut rl = rustyline::DefaultEditor::new().unwrap();

    'main_loop: loop {
        match app_state.current_shell_ctx {
            ShellContext::Local => {
                let prompt = format!("{}$ ", util::color::blue(" local"));
                let readline = rl.readline(&prompt);

                if let Err(e) = &readline {
                    match e {
                        rustyline::error::ReadlineError::Eof => {
                            exit();
                        }
                        rustyline::error::ReadlineError::Interrupted => {
                            continue;
                        }
                        _ => exit(),
                    }
                }

                let line = readline.unwrap();
                let input = line.split_whitespace().collect::<Vec<&str>>();

                // Skip when input is empty
                if input.first().is_none() {
                    continue;
                }

                for command in &commands {
                    if *input.first().unwrap() == command.name.as_str() {
                        let args = if let Some(a) = input.get(1..) {
                            CommandArgs::new(
                                a.iter().map(|x| x.to_string()).collect(),
                                app_state.clone(),
                            )
                        } else {
                            CommandArgs::new(vec![], app_state.clone())
                        };
                        let ret = (command.exec)(args);
                        app_state = ret.new_app_state;
                        continue 'main_loop;
                    }
                }
                println!("Unknown command: {}", input.first().unwrap());
            }
            ShellContext::Remote => {
                // let mut current_session = app_state.clone().current_session.unwrap();

                let prompt = format!(
                    "{}:{}$ ",
                    crate::util::color::red(&format!(
                        "󰢹 {}@{}",
                        app_state.current_session.username,
                        app_state.current_session.address.ip() // app_state.current_session.unwrap().address.to_string()
                    )),
                    crate::util::color::cyan(&app_state.current_session.pwd)
                );
                let readline = rl.readline(&prompt);

                if let Err(e) = &readline {
                    match e {
                        rustyline::error::ReadlineError::Eof => {
                            app_state.current_shell_ctx = ShellContext::Local;
                            continue;
                        }
                        rustyline::error::ReadlineError::Interrupted => {
                            continue;
                        }
                        _ => exit(),
                    }
                }

                let line = readline.unwrap();

                if line == "exit" {
                    app_state.current_shell_ctx = ShellContext::Local;
                    continue;
                }

                app_state.current_session.exec_command(&line);
                app_state.current_session.update_pwd();
            }
        };
    }
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub current_session: Session,
    pub current_shell_ctx: ShellContext,
    pub sessions: Vec<Session>,
}

impl AppState {
    fn new(initial_session: Session) -> Self {
        Self {
            current_session: initial_session.clone(),
            current_shell_ctx: ShellContext::Local,
            sessions: vec![initial_session],
        }
    }
}

#[derive(Clone, Debug)]
pub enum ShellContext {
    Local,
    Remote,
}

fn exit() {
    std::process::exit(0);
}

fn help_cmd(args: CommandArgs) -> CommandReturns {
    let commands = get_commands();

    println!("Usage:");
    commands.iter().for_each(|c| {
        println!(
            "  {}{}{}",
            c.name,
            " ".repeat(15 - c.name.len()),
            c.description
        );
    });
    CommandReturns::new(true, args.app_state)
}

fn exit_cmd(args: CommandArgs) -> CommandReturns {
    exit();
    CommandReturns::new(true, args.app_state)
}

fn get_commands() -> Vec<Command> {
    vec![
        Command::new("help", "Show this help message", Box::new(help_cmd)),
        Command::new("exit", "Exit the program", Box::new(exit_cmd)),
        Command::new(
            "sessions",
            "List available sessions",
            Box::new(session::sessions),
        ),
        Command::new(
            "listen",
            "Start listening a reverse shell",
            Box::new(listen),
        ),
    ]
}

fn listen(args: CommandArgs) -> CommandReturns {
    use std::net::SocketAddr;
    use std::net::TcpListener;

    if args.args.is_empty()
        || (args.args.len() == 1 && args.args[0] == "help")
        || args.args.len() > 2
    {
        use crate::util::tidy_usage;
        println!("listen on a port to receive a reverse shell");
        println!("Usage:");
        println!("  {}", tidy_usage("listen <port>", "Listen on a port"));
        println!(
            "  {}",
            tidy_usage("listen <port> -bg", "Listen on a port in background")
        );
        return CommandReturns::new(true, args.app_state);
    }

    let port = match args.args[0].parse::<u16>() {
        Ok(port) => port,
        Err(e) => {
            println!(
                "{}: failed to parse an arg as port: {}",
                crate::util::color::red("Error"),
                e
            );
            return CommandReturns::new(true, args.app_state);
        }
    };

    let laddr = SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        port,
    );

    let listener = TcpListener::bind(laddr).unwrap();

    println!("listening on {}", laddr);
    let (socket, raddr) = listener.accept().unwrap();
    println!(
        "recieved a connection from {}:{}",
        crate::util::color::cyan(&raddr.ip().to_string()),
        crate::util::color::magenta(&raddr.port().to_string())
    );

    let new_session = Session::new(socket, &args.app_state.sessions);
    // args.sessions.push(new_session);
    let mut new_app_state = args.app_state;
    new_app_state.sessions.push(new_session);
    // new_app_state.current_session = Some(new_session);

    CommandReturns::new(true, new_app_state)
}

fn initial_listen(port: u16) -> Session {
    use std::net::SocketAddr;
    use std::net::TcpListener;

    let laddr = SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        port,
    );

    let listener = TcpListener::bind(laddr).unwrap();

    println!("listening on {}", laddr);
    let (socket, raddr) = listener.accept().unwrap();
    println!(
        "recieved a connection from {}:{}",
        crate::util::color::cyan(&raddr.ip().to_string()),
        crate::util::color::magenta(&raddr.port().to_string())
    );

    Session::new(socket, &[])
}
