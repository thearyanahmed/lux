use colored::Colorize;

use crate::api::{PaginatedResponse, Project};

pub struct Message;

// Fixed width for all prefixes (7 chars = "[ERROR]")
const PREFIX_WIDTH: usize = 7;

impl Message {
    pub fn greet(name: &str) {
        let msg = format!(
            "hello {}, welcome to {}!",
            name.bold(),
            "projectlighthouse".yellow()
        );
        println!("{:>WIDTH$} {}", "[LUX]".blue(), msg, WIDTH = PREFIX_WIDTH);
    }

    pub fn say(msg: &str) {
        println!("{:>WIDTH$} {}", "[LUX]".blue(), msg, WIDTH = PREFIX_WIDTH);
    }

    pub fn cheer(msg: &str) {
        println!("{:>WIDTH$} {}", "[OK]".green(), msg, WIDTH = PREFIX_WIDTH);
    }

    pub fn complain(msg: &str) {
        eprintln!("{:>WIDTH$} {}", "[WARN]".yellow(), msg, WIDTH = PREFIX_WIDTH);
    }

    pub fn oops(msg: &str) {
        eprintln!("{:>WIDTH$} {}", "[ERROR]".red(), msg, WIDTH = PREFIX_WIDTH);
    }

    pub fn print_projects(response: &PaginatedResponse<Project>) {
        Self::say(&format!("available projects ({} total):\n", response.meta.total));

        for project in &response.data {
            Self::print_project(project);
        }
    }

    fn print_project(project: &Project) {
        println!("  {} {}", "[#]".dimmed(), project.name.bold());
        if let Some(desc) = &project.short_description {
            println!("    {}", desc);
        }
        let tasks_count = project.tasks_count.unwrap_or(0);
        println!("    {} {}  {} {}", "slug:".dimmed(), project.slug.dimmed(), "tasks:".dimmed(), tasks_count.to_string().dimmed());
        println!("    {} {}\n", "url:".dimmed(), project.url().dimmed());
    }

    pub fn print_project_detail(project: &Project) {
        println!("  {} {}", "[#]".dimmed(), project.name.bold());
        println!("    {} {}", "slug:".dimmed(), project.slug.dimmed());
        println!("    {} {}\n", "url:".dimmed(), project.url().dimmed());

        if let Some(tasks) = &project.tasks {
            println!("  {} ({}):\n", "tasks".bold(), tasks.len());
            for task in tasks {
                println!("    {} {}", format!("{}.", task.sort_order).dimmed(), task.title);
                println!("      {} {}\n", "status:".dimmed(), task.status.dimmed());
            }
        }
    }
}

/// Welcome/greet the user
#[macro_export]
macro_rules! greet {
    ($name:expr) => {
        $crate::message::Message::greet($name)
    };
}

/// General info message
#[macro_export]
macro_rules! say {
    ($msg:expr) => {
        $crate::message::Message::say($msg)
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::message::Message::say(&format!($fmt, $($arg)*))
    };
}

/// Success message
#[macro_export]
macro_rules! cheer {
    ($msg:expr) => {
        $crate::message::Message::cheer($msg)
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::message::Message::cheer(&format!($fmt, $($arg)*))
    };
}

/// Warning message
#[macro_export]
macro_rules! complain {
    ($msg:expr) => {
        $crate::message::Message::complain($msg)
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::message::Message::complain(&format!($fmt, $($arg)*))
    };
}

/// Error message
#[macro_export]
macro_rules! oops {
    ($msg:expr) => {
        $crate::message::Message::oops($msg)
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::message::Message::oops(&format!($fmt, $($arg)*))
    };
}
