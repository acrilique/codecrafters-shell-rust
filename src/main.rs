#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        let command = buffer.trim();

        if command == "exit" {
            break;
        }
        println!("{}: command not found", command.trim());
    }
}
