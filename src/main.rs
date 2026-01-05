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
        } else if tokens[0] == "type" {
            if tokens.len() < 2 {
                continue;
            }
            match tokens[1] {
                "echo" | "exit" | "type" => {
                    println!("{} is a shell builtin", tokens[1]);
                }
                _ => {
                    println!("{}: not found", tokens[1]);
                }
            }
        } else {
            println!("{}: command not found", tokens[0]);
        }
    }
}
