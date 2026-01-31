use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

use luxctl::{
    api::LighthouseAPIClient, auth::TokenAuthenticator, commands, config::Config, greet,
    message::Message, oops, LIGHTHOUSE_URL, VERSION,
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

    /// Project-specific helper tools (e.g., data generators)
    Helper {
        /// Helper name (e.g., 1brc)
        name: String,

        /// Number of rows to generate
        #[arg(short = 'r', long, default_value = "10000")]
        rows: u64,

        /// Measurements output file
        #[arg(short = 'm', long, default_value = "data/measurements.txt")]
        measurements: String,

        /// Expected output file
        #[arg(short = 'e', long, default_value = "expected/output.txt")]
        expected: String,
    },
}

#[derive(Subcommand)]
enum LabAction {
    /// See all available labs you can work on
    List {
        /// Open in browser: no value opens /labs, with index opens that lab
        #[arg(short = 'w', long, num_args = 0..=1, default_missing_value = "-1")]
        web: Option<i32>,
    },
    /// Get details about a lab before starting
    Show {
        #[arg(short = 's', long)]
        slug: String,
    },
    /// Begin working on a lab in your current directory
    Start {
        /// Lab slug or index from `lab list`
        #[arg(short = 'i', long)]
        id: String,

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
                Ok(user) => println!("{}", user.name),
                Err(err) => oops!("failed to fetch user: {}", err),
            }
        }

        Commands::Lab { action } => match action {
            LabAction::List { web } => {
                let config = Config::load()?;
                if !config.has_auth_token() {
                    oops!("not authenticated. Run: `{}`", Commands::AUTH_USAGE);
                    return Ok(());
                }

                let client = LighthouseAPIClient::from_config(&config);
                match client.labs(None, None).await {
                    Ok(response) => {
                        Message::print_labs(&response);
                        if let Some(index) = web {
                            let url = if index < 0 {
                                format!("{}/labs", LIGHTHOUSE_URL)
                            } else if let Some(lab) = response.data.get(index as usize) {
                                lab.url()
                            } else {
                                oops!("invalid index: {}", index);
                                return Ok(());
                            };
                            let _ = std::process::Command::new("open").arg(&url).spawn();
                        }
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
                id,
                workspace,
                runtime,
            } => {
                let actual_slug = if let Ok(index) = id.parse::<usize>() {
                    let config = Config::load()?;
                    if !config.has_auth_token() {
                        oops!("not authenticated. Run: `{}`", Commands::AUTH_USAGE);
                        return Ok(());
                    }
                    let client = LighthouseAPIClient::from_config(&config);
                    match client.labs(None, None).await {
                        Ok(response) => {
                            if let Some(lab) = response.data.get(index) {
                                lab.slug.clone()
                            } else {
                                oops!("invalid index: {}", index);
                                return Ok(());
                            }
                        }
                        Err(err) => {
                            oops!("failed to fetch labs: {}", err);
                            return Ok(());
                        }
                    }
                } else {
                    id
                };
                commands::lab::start(&actual_slug, &workspace, runtime.as_deref()).await?;
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

        Commands::Helper {
            name,
            rows,
            measurements,
            expected,
        } => {
            commands::helpers::run(&name, rows, &measurements, &expected)?;
        }
    }

    Ok(())
}
