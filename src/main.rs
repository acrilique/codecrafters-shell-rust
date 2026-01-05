#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    let command = &mut String::new();

    print!("$ ");
    io::stdout().flush().unwrap();

    io::stdin().read_line(command).unwrap();

    print!("{}: command not found", command.trim());
    io::stdout().flush().unwrap();
}
