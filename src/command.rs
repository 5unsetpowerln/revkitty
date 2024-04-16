pub struct Command {
    pub name: String,
    pub description: String,
    pub exec: Box<dyn Fn(CommandArgs) -> CommandReturns>,
}

pub struct CommandArgs {
    pub args: Vec<String>,
    pub app_state: crate::AppState,
}

pub struct CommandReturns {
    pub is_ok: bool,
    pub new_app_state: crate::AppState,
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
    pub fn new(args: Vec<String>, app_state: crate::AppState) -> Self {
        Self { args, app_state }
    }
}

impl CommandReturns {
    pub fn new(is_ok: bool, new_app_state: crate::AppState) -> Self {
        Self {
            is_ok,
            new_app_state,
        }
    }
}