use anyhow::anyhow;

use crate::{
    session::{self, make_session_table},
    util::print_error,
};

use super::CommandReturns;

pub struct Sessions {}

impl super::Command for Sessions {
    fn name() -> String {
        "sessions".to_string()
    }

    fn info() -> String {
        "List available sessions".to_string()
    }

    async fn exec(args: super::CommandArgs) -> super::CommandReturns {
        // print session list
        if args.args.is_empty() {
            // print all sessions available as a table;
            let table = match make_session_table() {
                Ok(t) => t,
                Err(e) => {
                    print_error("failed to make session table", e);
                    return CommandReturns::new(false, args.manager);
                }
            };

            println!("{}", table);
            return CommandReturns::new(true, args.manager);
        }

        // print help message
        if args.args.len() == 1 && args.args[0] == "help" || args.args.len() > 2 {
            Self::help();
            return CommandReturns::new(true, args.manager);
        }

        // change shell local to remote
        let id = match args.args[0].parse::<u16>() {
            Ok(num) => num,
            Err(e) => {
                print_error("failed to parse an arg as port", anyhow!(e.to_string()));
                Self::help();
                return CommandReturns::new(false, args.manager);
            }
        };
        if match session::is_session_exist(id) {
            Ok(b) => b,
            Err(e) => {
                print_error("failed to check if session exists", e);
                return CommandReturns::new(false, args.manager);
            }
        } {
            let mut manager = args.manager;
            manager.current_session_id = Some(id);
            manager.is_shell_remote = true;
            return CommandReturns::new(true, manager);
        } else {
            println!("Session {} not found", id);
        }

        CommandReturns::new(true, args.manager)
    }

    fn help() {
        use crate::util::tidy_usage;
        println!("Usage:");
        println!(
            "\t{}",
            tidy_usage("sessions", "List all sessions available")
        );
        println!(
            "\t{}",
            tidy_usage(
                "sessions <id>",
                "Switch current shell context to a remote session with the given id"
            )
        );
    }
}
