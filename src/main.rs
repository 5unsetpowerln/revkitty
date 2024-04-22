mod command;
mod session;
mod util;

use std::{sync::Mutex, thread::current};

use command::{Command, CommandArgs, CommandReturns};
use log::{debug, error, info, log_enabled, Level, Metadata, Record};
use session::Session;
use std::io::Write;

use crate::util::color;

fn main() {
    let commands = get_commands();
    std::env::set_var("RUST_LOG", "info");

    let mut manager = Manager::new();

    let mut rl = rustyline::DefaultEditor::new().unwrap();

    env_logger::builder()
        .format(|buf, record| match record.level() {
            Level::Error => writeln!(buf, "[{}] {}", color::red("+"), record.args()),
            Level::Debug => writeln!(buf, "[{}] {}", color::green("+"), record.args()),
            Level::Info => writeln!(buf, "[{}] {}", color::cyan("+"), record.args()),
            Level::Warn => writeln!(buf, "[{}] {}", color::yellow("+"), record.args()),
            Level::Trace => writeln!(buf, "[{}] {}", color::white("+"), record.args()),
        })
        .init();

    'main_loop: loop {
        if manager.is_shell_remote {
            // let mut sessions = get_sessions();
            let mut current_session =
                crate::session::get_session(manager.current_session_id).unwrap();
            let prompt = format!(
                "{} {} {} ",
                color::red("[sayo]"),
                color::blue(&format!(
                    "{}@{}:{}",
                    current_session.username,
                    current_session.address.ip(), // manager.current_session.unwrap().address.to_string()
                    current_session.pwd
                )),
                color::red(">")
            );
            let readline = rl.readline(&prompt);

            if let Err(e) = &readline {
                match e {
                    rustyline::error::ReadlineError::Eof => {
                        manager.is_shell_remote = false;
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
                manager.is_shell_remote = false;
                continue;
            }

            current_session.exec_command_with_pretty_output(&line);
            current_session.update_pwd();
            crate::session::set_session(manager.current_session_id.unwrap(), &current_session);
        } else {
            let prompt = format!("{}", color::red("[sayo] > "));
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
                        CommandArgs::new(a.iter().map(|x| x.to_string()).collect(), manager.clone())
                    } else {
                        CommandArgs::new(vec![], manager.clone())
                    };
                    let ret = (command.exec)(args);
                    manager = ret.new_manager;
                    continue 'main_loop;
                }
            }
            println!("Unknown command: {}", input.first().unwrap());
        }
    }
}

#[derive(Debug, Clone)]
pub struct Manager {
    pub current_session_id: Option<u16>,
    pub is_shell_remote: bool,
}

impl Manager {
    fn new() -> Self {
        Self {
            current_session_id: None,
            is_shell_remote: false,
        }
    }
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
    CommandReturns::new(true, args.manager)
}

fn exit_cmd(args: CommandArgs) -> CommandReturns {
    exit();
    CommandReturns::new(true, args.manager)
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
        return CommandReturns::new(true, args.manager);
    }

    let port = match args.args[0].parse::<u16>() {
        Ok(port) => port,
        Err(e) => {
            println!(
                "{}: failed to parse an arg as port: {}",
                crate::util::color::red("Error"),
                e
            );
            return CommandReturns::new(true, args.manager);
        }
    };

    let laddr = SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        port,
    );

    let listener = TcpListener::bind(laddr).unwrap();

    info!("listening on {}", laddr);
    let (socket, raddr) = listener.accept().unwrap();
    info!(
        "recieved a connection from {}:{}",
        raddr.ip().to_string(),
        &raddr.port().to_string()
    );

    let mut new_session = Session::new(socket);
    new_session.init();
    // args.sessions.push(new_session);
    let mut manager = args.manager;
    crate::session::push_session(new_session);
    // new_manager.current_session = Some(new_session);

    CommandReturns::new(true, manager)
}
