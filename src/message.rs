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
        Self::say(&format!("Available projects ({} total):\n", response.meta.total));

        for project in &response.data {
            Self::print_project(project);
        }

        if response.meta.last_page > 1 {
            Self::say(&format!(
                "Page {} of {}",
                response.meta.current_page, response.meta.last_page
            ));
        }
    }

    fn print_project(project: &Project) {
        let featured = if project.is_featured { " [featured]" } else { "" };
        println!("  {} {}{}", "→".cyan(), project.name.bold(), featured.yellow());
        println!("    {}", project.short_description.dimmed());
        println!("    slug: {}  tasks: {}\n", project.slug.cyan(), project.tasks_count);
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
