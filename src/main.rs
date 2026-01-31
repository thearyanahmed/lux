use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

use luxctl::{
    api::LighthouseAPIClient, auth::TokenAuthenticator, commands, config::Config, greet,
    message::Message, oops, VERSION,
};

#[derive(Parser)]
#[command(name = "luxctl")]
#[command(version = VERSION)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Log in with your API token from projectlighthouse.io
    Auth {
        #[arg(short = 't', long)]
        token: String,
    },

    /// See your profile and progress
    Whoami,

    /// Labs are a series of challenges that build on each other, preparing you for real-world problems
    Lab {
        #[command(subcommand)]
        action: LabAction,
    },

    /// Tasks are individual challenges within a project - tackle them in any order
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Test your solution to see if it passes
    Run {
        #[arg(short = 'l', long)]
        lab: Option<String>,

        #[arg(short = 't', long)]
        task: String,

        #[arg(short = 'd', long)]
        detailed: bool,
    },

    /// Run all the tasks of a project at once
    Validate {
        #[arg(short = 'd', long)]
        detailed: bool,

        #[arg(short = 'a', long)]
        all: bool,
    },

    /// Stuck on a task? Hints can help, but they might cost you XP
    Hint {
        #[command(subcommand)]
        action: HintAction,
    },

    /// Check your environment and diagnose issues
    Doctor,
}

#[derive(Subcommand)]
enum LabAction {
    /// See all available labs you can work on
    List,
    /// Get details about a lab before starting
    Show {
        #[arg(short = 's', long)]
        slug: String,
    },
    /// Begin working on a lab in your current directory
    Start {
        #[arg(short = 's', long)]
        slug: String,

        /// Workspace directory (defaults to current directory)
        #[arg(short = 'w', long, default_value = ".")]
        workspace: String,

        /// Runtime environment (go, rust, c)
        #[arg(short = 'r', long)]
        runtime: Option<String>,
    },
    /// See your progress on the current lab
    Status,
    /// Stop working on the current lab
    Stop,
    /// Change lab settings (runtime, workspace)
    Set {
        /// Runtime environment (go, rust, c)
        #[arg(short = 'r', long)]
        runtime: Option<String>,

        /// Workspace directory
        #[arg(short = 'w', long)]
        workspace: Option<String>,
    },
    /// Start fresh - reset all progress on the current lab
    Restart,
}

#[derive(Subcommand)]
enum TaskAction {
    /// See all tasks in your current project
    List {
        #[arg(short = 'r', long)]
        refresh: bool,
    },
    /// Read the task description and requirements
    Show {
        /// Task number or slug
        #[arg(short = 't', long)]
        task: String,

        /// Show full description
        #[arg(short = 'd', long)]
        detailed: bool,
    },
}

#[derive(Subcommand)]
enum HintAction {
    /// See what hints are available for a task
    List {
        #[arg(short = 't', long)]
        task: String,
    },
    /// Reveal a hint - this might cost you XP
    Unlock {
        #[arg(short = 't', long)]
        task: String,

        #[arg(short = 'i', long)]
        hint: String,
    },
}

impl Commands {
    pub const AUTH_USAGE: &'static str = "luxctl auth --token $token";
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

        Commands::Whoami => {
            let config = match Config::load() {
                Ok(c) if c.has_auth_token() => c,
                _ => {
                    println!("nobody");
                    println!("login with: {}", Commands::AUTH_USAGE);
                    return Ok(());
                }
            };

            let client = LighthouseAPIClient::from_config(&config);
            match client.me().await {
                Ok(user) => {
                    println!("{}", user.name);
                    println!("{}", user.email);
                    if let Some(stats) = user.stats {
                        println!();
                        println!("projects: {}", stats.projects_attempted);
                        println!("tasks completed: {}", stats.tasks_completed);
                        println!("total xp: {}", stats.total_xp);
                    }
                }
                Err(err) => {
                    oops!("failed to fetch user: {}", err);
                }
            }
        }

        Commands::Lab { action } => match action {
            LabAction::List => {
                let config = Config::load()?;
                if !config.has_auth_token() {
                    oops!("not authenticated. Run: `{}`", Commands::AUTH_USAGE);
                    return Ok(());
                }

                let client = LighthouseAPIClient::from_config(&config);
                match client.labs(None, None).await {
                    Ok(response) => {
                        Message::print_labs(&response);
                    }
                    Err(err) => {
                        oops!("failed to fetch labs: {}", err);
                    }
                }
            }
            LabAction::Show { slug } => {
                let config = Config::load()?;
                if !config.has_auth_token() {
                    oops!("not authenticated. Run: `{}`", Commands::AUTH_USAGE);
                    return Ok(());
                }

                let client = LighthouseAPIClient::from_config(&config);
                match client.lab_by_slug(&slug).await {
                    Ok(lab) => {
                        Message::print_lab_detail(&lab);
                    }
                    Err(err) => {
                        oops!("failed to fetch lab: {}", err);
                    }
                }
            }
            LabAction::Start {
                slug,
                workspace,
                runtime,
            } => {
                commands::lab::start(&slug, &workspace, runtime.as_deref()).await?;
            }
            LabAction::Status => {
                commands::lab::status()?;
            }
            LabAction::Stop => {
                commands::lab::stop()?;
            }
            LabAction::Set { runtime, workspace } => {
                if let Some(ref rt) = runtime {
                    commands::lab::set_runtime(rt)?;
                }
                if let Some(ref ws) = workspace {
                    commands::lab::set_workspace(ws)?;
                }
                if runtime.is_none() && workspace.is_none() {
                    oops!("provide --runtime or --workspace to set");
                }
            }
            LabAction::Restart => {
                commands::lab::restart().await?;
            }
        },

        Commands::Task { action } => match action {
            TaskAction::List { refresh } => {
                commands::tasks::list(refresh).await?;
            }
            TaskAction::Show { task, detailed } => {
                commands::task::show(&task, detailed).await?;
            }
        },

        Commands::Run {
            lab,
            task,
            detailed,
        } => {
            commands::run::run(&task, lab.as_deref(), detailed).await?;
        }

        Commands::Validate { detailed, all } => {
            commands::validate::validate_all(all, detailed).await?;
        }

        Commands::Hint { action } => match action {
            HintAction::List { task } => {
                commands::hints::list(&task).await?;
            }
            HintAction::Unlock { task, hint } => {
                commands::hints::unlock(&task, &hint).await?;
            }
        },

        Commands::Doctor => {
            commands::doctor::run().await?;
        }
    }

    Ok(())
}
