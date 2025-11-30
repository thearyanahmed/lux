use colored::Colorize;

pub struct Printer;

impl Printer {
    pub fn info(msg : &str) {
        println!("{}", msg.blue())
    }

    pub fn raw(msg: &str) {
        println!("{}", msg)
    }
}
