use clap::{arg, Parser, Subcommand};
use color_eyre::eyre::Result;

use lux::{VERSION, api, auth::TokenAuthenticator, config::Config, greet, oops};

#[derive(Parser)]
#[command(name = "lux")]
#[command(version = VERSION)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Auth {
        #[arg(short = 't', long)]
        token: String,
    },
    
    Run {
        #[arg(short = 'p', long)]
        project: String,

        #[arg(short = 't', long)]
        task: String,
    },

    Projects {
    }


}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let client = api::LighthouseAPIClient::default();

    let cli = Cli::parse();

    let config = Config::load()?;

    match cli.commands {
        Commands::Run { project, task } => {
            log::info!("Running task '{}' for project '{}'", task, project);
        },
        Commands::Projects {} => {
            if config.has_auth_token() {
                oops!("please authenticate first");
            }
            log::debug!("projects called")
        },
        Commands::Auth { token } => {
            let authenticator = TokenAuthenticator::new(client, &token);

            match authenticator.authenticate().await {
                Ok(user) => {
                    greet!(user.name());
                }
                Err(err) => {
                    log::error!("{}", err);
                    oops!("{}", err);
                }
            }
        }
    }

    Ok(())
}
