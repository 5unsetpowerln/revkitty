mod command;
mod session;
mod util;

use std::{env::args, process, sync::Mutex, thread::current};

use command::{CommandArgs, CommandReturns};
use log::{debug, error, info, log_enabled, Level, Metadata, Record};
use std::io::Write;

use crate::util::color;

#[tokio::main]
async fn main() {
    // let commands = get_commands();
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
            let session_id = manager.current_session_id.unwrap();
            let session_metadata = session::get_metadata(session_id);

            let prompt = format!(
                "{} {} {} ",
                color::red("[sayo]"),
                color::blue(&format!(
                    "{}@{}:{}",
                    session_metadata.username,
                    session_metadata.address, // manager.current_session.unwrap().address.to_string()
                    session_metadata.cwd
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
                    _ => process::exit(0),
                }
            }

            let line = readline.unwrap();

            if line == "exit" {
                manager.is_shell_remote = false;
                continue;
            }

            session::execute_command_prettily(session_id, line.as_bytes()).await;
        } else {
            let prompt = format!("{}", color::red("[sayo] > "));
            let readline = rl.readline(&prompt);

            if let Err(e) = &readline {
                match e {
                    rustyline::error::ReadlineError::Eof => {
                        // exit();
                        process::exit(0);
                    }
                    rustyline::error::ReadlineError::Interrupted => {
                        continue;
                    }
                    _ => process::exit(0),
                }
            }

            let line = readline.unwrap();
            let input = line.split_whitespace().collect::<Vec<&str>>();

            // Skip when input is empty
            if input.first().is_none() {
                continue;
            }

            let command = input.first().unwrap();

            // if command == "help" {
            // help_cmd(CommandArgs::new());
            // println!();
            // }

            if let Some(args) = input.get(1..) {
                let ret = crate::command::execute_command(
                    command,
                    CommandArgs::new(Some(args), manager.clone()),
                )
                .await;
                manager = ret.new_manager;
            } else {
                let ret = crate::command::execute_command(
                    command,
                    CommandArgs::new(None, manager.clone()),
                )
                .await;
                manager = ret.new_manager;
            }
        };
    }
}

#[derive(Debug, Clone)]
pub struct Manager {
    pub current_session_id: Option<u16>,
    pub is_shell_remote: bool,
}

pub trait Command {
    fn name(&self) -> String;
    fn info(&self) -> String;
    fn exec(&self, args: CommandArgs) -> CommandReturns;
}

impl Manager {
    fn new() -> Self {
        Self {
            current_session_id: None,
            is_shell_remote: false,
        }
    }
}
