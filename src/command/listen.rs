use crate::util::tidy_usage;

use super::{CommandArgs, CommandReturns};

pub struct Listen {}

impl super::Command for Listen {
    fn name() -> String {
        "listen".to_string()
    }

    fn info() -> String {
        "Start listening a reverse shell".to_string()
    }

    async fn exec(args: super::CommandArgs) -> super::CommandReturns {
        if args.args.is_empty()
            || (args.args.len() == 1 && args.args[0] == "help")
            || args.args.len() > 2
        {
            help();
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

        let mut manager = args.manager;
        manager.current_session_id = Some(crate::session::new_session(port).await);

        CommandReturns::new(true, manager)
    }
}

fn help() {
    println!("listen on a port to receive a reverse shell");
    println!("Usage:");
    println!("  {}", tidy_usage("listen <port>", "Listen on a port"));
    println!(
        "  {}",
        tidy_usage("listen <port> -bg", "Listen on a port in background")
    );
}
