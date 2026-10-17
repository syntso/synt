use std::env;
use std::process::ExitCode;

use synt_core::{Arguments, Builtins, CoreError, LoginCommand, Server, Storage};

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("synt-core: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), CoreError> {
    if let Some(command) = LoginCommand::from_args(env::args_os().skip(1))? {
        println!("{}", command.run().await?);
        return Ok(());
    }

    let arguments = Arguments::parse(env::args_os().skip(1))?;
    let endpoint = arguments.endpoint()?;
    let data = arguments.data()?;

    let storage = Storage::open(&data, arguments.weak_vault())?;
    let builtins = Builtins::start(&data, &storage)?;
    let server = Server::start(&endpoint, builtins.connect().await?)?;

    builtins.serve(server, arguments.idle()).await;

    Ok(())
}
