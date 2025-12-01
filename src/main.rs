use clap::{arg, Parser, Subcommand};
use color_eyre::eyre::Result;

use lux::{VERSION, api::LighthouseAPIClient, auth::TokenAuthenticator, config::Config, greet, message::Message, oops};

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
        #[arg(short = 's', long)]
        slug: Option<String>,
    }
}

impl Commands {
    pub const AUTH_USAGE: &'static str = "lux auth --token <TOKEN>";
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let cli = Cli::parse();

    match cli.commands {
        Commands::Auth { token } => {
            let authenticator = TokenAuthenticator::new(&token);

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
        Commands::Projects { slug } => {
            let config = Config::load()?;
            if !config.has_auth_token() {
                oops!("not authenticated. Run: `{}`", Commands::AUTH_USAGE);
                return Ok(());
            }

            let client = LighthouseAPIClient::from_config(&config);

            match slug {
                Some(slug) => {
                    match client.project_by_slug(&slug).await {
                        Ok(project) => {
                            Message::print_project_detail(&project);
                        }
                        Err(err) => {
                            oops!("failed to fetch project: {}", err);
                        }
                    }
                }
                None => {
                    match client.projects(None, None).await {
                        Ok(response) => {
                            Message::print_projects(&response);
                        }
                        Err(err) => {
                            oops!("failed to fetch projects: {}", err);
                        }
                    }
                }
            }
        }
        Commands::Run { project, task } => {
            log::info!("Running task '{}' for project '{}'", task, project);
        },
    }

    Ok(())
}
