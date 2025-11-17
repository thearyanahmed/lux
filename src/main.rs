mod tasks;
mod validators;
mod version;

fn main() {
    println!("the version is {}", version::version());
}
