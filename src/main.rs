#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let command = &mut String::new();
        io::stdin().read_line(command).unwrap();
        println!("{}: command not found", command.trim());
    }
}
