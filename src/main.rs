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

        let tokens: Vec<&str> = command.split_whitespace().collect();

        if tokens.is_empty() {
            continue;
        }

        if tokens[0] == "echo" {
            println!("{}", tokens[1..].join(" "));
        } else {
            println!("{}: command not found", command.trim());
        }
    }
}
