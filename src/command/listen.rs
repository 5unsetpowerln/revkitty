use anyhow::anyhow;
use log::info;

use crate::util::{print_error, tidy_usage};

use super::CommandReturns;

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
            Self::help();
            return CommandReturns::new(true, args.manager);
        }

        let port = match args.args[0].parse::<u16>() {
            Ok(port) => port,
            Err(e) => {
                print_error("failed to parse an arg as port", anyhow!(e));
                return CommandReturns::new(false, args.manager);
            }
        };

        let mut manager = args.manager;
        manager.current_session_id = Some(match crate::session::new_session(port).await {
            Ok(s) => s,
            Err(e) => {
                print_error("failed to create a new session", e);
                return CommandReturns::new(false, manager);
            }
        });

        CommandReturns::new(true, manager)
    }

    fn help() {
        info!("usage:");
        println!("  {}", tidy_usage("listen <port>", "Listen on a port"));
        println!(
            "  {}",
            tidy_usage("listen <port> -bg", "Listen on a port in background")
        );
    }
}
