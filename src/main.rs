#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    print!("$ ");
    io::stdout().flush().unwrap();

    let command = &mut String::new();
    io::stdin().read_line(command).unwrap();
    print!("{}: command not found", command.trim());
}
