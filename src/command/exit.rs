use std::process;

pub struct Exit {}

impl super::Command for Exit {
    fn name() -> String {
        "exit".to_string()
    }

    fn info() -> String {
        "Exit the program".to_string()
    }

    #[allow(unreachable_code)]
    async fn exec(_args: super::CommandArgs) -> super::CommandReturns {
        process::exit(0);
        super::CommandReturns::new(true, _args.manager)
    }
}
