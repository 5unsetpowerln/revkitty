use self::{exit::Exit, listen::Listen, sessions::Sessions};

mod exit;
mod listen;
mod sessions;

pub async fn execute_command(command: &str, args: CommandArgs) -> CommandReturns {
    match command {
        "exit" => Exit::exec(args).await,
        "listen" => Listen::exec(args).await,
        "sessions" => Sessions::exec(args).await,
        "help" => Help::exec(args).await,
        _ => {
            println!("Unknown command: {}", command);

            CommandReturns::new(false, args.manager)
        }
    }
}

pub fn display_help() {
    println!("Usage:");
    println!(
        "  {}{}{}",
        Exit::name(),
        " ".repeat(20 - Exit::name().len()),
        Exit::info()
    );

    println!(
        "  {}{}{}",
        Listen::name(),
        " ".repeat(20 - Listen::name().len()),
        Listen::info()
    );

    println!(
        "  {}{}{}",
        Sessions::name(),
        " ".repeat(20 - Sessions::name().len()),
        Sessions::info()
    );
}

struct Help {}

impl Command for Help {
    fn name() -> String {
        "help".to_string()
    }
    fn info() -> String {
        "Display help message".to_string()
    }
    async fn exec(_args: CommandArgs) -> CommandReturns {
        display_help();
        CommandReturns::new(true, _args.manager)
    }
    fn help() {
        display_help();
    }
}

pub struct CommandArgs {
    pub args: Vec<String>,
    pub manager: crate::Manager,
}

pub struct CommandReturns {
    pub is_ok: bool,
    pub new_manager: crate::Manager,
}

impl CommandArgs {
    pub fn new(args: Option<&[&str]>, manager: crate::Manager) -> Self {
        let args = if let Some(args) = args {
            args.iter().map(|s| s.to_string()).collect()
        } else {
            vec![]
        };
        Self { args, manager }
    }
}

impl CommandReturns {
    pub fn new(is_ok: bool, new_manager: crate::Manager) -> Self {
        Self { is_ok, new_manager }
    }
}

pub trait Command {
    fn name() -> String;
    fn info() -> String;
    fn help();
    async fn exec(args: CommandArgs) -> CommandReturns;
}
