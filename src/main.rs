use clap::{arg, Parser, Subcommand};
use color_eyre::eyre::Result;

use lux::{VERSION, api, auth::TokenAuthenticator};

#[derive(Parser)]
#[command(name = "lux")]
#[command(version = VERSION)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(short ='p', long)]
        project: String,

        #[arg(short = 't', long)]
        task: String, },

    Auth {
        #[arg(short ='t', long)]
        token: String,
    },
}


// scratch pad:
// lux run -p project_slug -t task_slug 
// same as: lux run --project $project_slug --task $task_slug
//
// we should log as well, and maybe the user can have it as verbose log

#[tokio::main]
async fn main() -> Result<()>{
    color_eyre::install()?;
    env_logger::init();

    let client = api::LighthouseAPIClient::default();

    let cli = Cli::parse();

    match cli.commands {
        Commands::Run { project, task } => {
            log::info!("Running task '{}' for project '{}'", task, project);
        },
        Commands::Auth { token } => {
            let authenticator = TokenAuthenticator::new( client, &token);

            match authenticator.authenticate().await {
                Ok(user) => {
                    log::info!("welcome {} !", user.name())
                },
                Err(err) => {
                    log::error!("{}", err)
                }
            }
        },
    }

    Ok(())
}


