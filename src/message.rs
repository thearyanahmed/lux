use colored::Colorize;

pub struct Message;

impl Message {
    pub fn greet(name: &str) {
        let msg = format!(
            "hello {}, welcome to {}!",
            name.bold(),
            "projectlighthouse".yellow()
        );
        println!("{} {}", "[LUX]".blue(), msg);
    }

    pub fn say(msg: &str) {
        println!("{} {}", "[LUX]".blue(), msg);
    }

    pub fn cheer(msg: &str) {
        println!("{} {}", "[OK]".green(), msg);
    }

    pub fn complain(msg: &str) {
        eprintln!("{} {}", "[WARN]".yellow(), msg);
    }

    pub fn oops(msg: &str) {
        eprintln!("{} {}", "[ERROR]".red(), msg);
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
