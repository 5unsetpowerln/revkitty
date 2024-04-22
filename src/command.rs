pub struct Command {
    pub name: String,
    pub description: String,
    pub exec: Box<dyn Fn(CommandArgs) -> CommandReturns>,
}

pub struct CommandArgs {
    pub args: Vec<String>,
    pub manager: crate::Manager,
}

pub struct CommandReturns {
    pub is_ok: bool,
    pub new_manager: crate::Manager,
}

impl Command {
    pub fn new(
        name: &str,
        description: &str,
        exec: Box<dyn Fn(CommandArgs) -> CommandReturns>,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            exec,
        }
    }
}

impl CommandArgs {
    pub fn new(args: Vec<String>, manager: crate::Manager) -> Self {
        Self { args, manager }
    }
}

impl CommandReturns {
    pub fn new(is_ok: bool, new_manager: crate::Manager) -> Self {
        Self {
            is_ok,
            new_manager,
        }
    }
}
