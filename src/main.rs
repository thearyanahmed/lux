use clap::Subcommand;
use clap::arg;
use clap::Parser;

mod tasks;
mod validators;

pub const VERSION: &str = "0.0.1";

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
        #[arg(short = 't', long)]
        task_id: String,
    }
}

fn main() {
    let cli = CLI::parse();

    match cli.commands {
        Commands::Run { task_id } => {
            match tasks::get_task(&task_id) {
                None => {
                    println!("not found");
                },
                _ => {
                    println!("probbly found something")
                }
            }
        }
    }
}


