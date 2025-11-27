use clap::{arg, Parser, Subcommand};
use color_eyre::eyre::Result;

use lux::{VERSION, api};

#[derive(Parser)]
#[command(name = "lux")]
#[command(version = VERSION)]
struct CLI {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(short ='p', long)]
        project: String,

        #[arg(short = 't', long)]
        task: String,
    },

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

fn main() -> Result<()>{
    color_eyre::install()?;
    env_logger::init();

    let api = api::LighthouseAPI::default();

    log::info!("{}",api);

    let cli = CLI::parse();

    match cli.commands {
        Commands::Run { project, task } => {
            log::info!("Running task '{}' for project '{}'", task, project);
        },
        Commands::Auth { token } => {
            log::info!("Authenticating with token '{}'", token);
        },
    }

    Ok(())
}


