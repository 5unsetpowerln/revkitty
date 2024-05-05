use log::{debug, info};

use crate::session::{self, make_session_table};

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
            let table = make_session_table();

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
        if session::is_session_exist(id) {
            let mut manager = args.manager;
            manager.current_session_id = Some(id);
            manager.is_shell_remote = true;
            return CommandReturns::new(true, manager);
        } else {
            println!("Session {} not found", id);
        }

        CommandReturns::new(true, args.manager)
    }
}
