use colored::Colorize;

pub struct Message;

impl Message {
    pub fn welcome_user(name: &str) {
        let msg = format!("hello {}, welcome to {}!", name.bold(), "projectlighthouse".yellow());
        println!("{}", msg);
    }
}
