#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        let command = buffer.trim();
        let tokens: Vec<&str> = command.split_whitespace().collect();

        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "exit" => {
                break;
            }
            "echo" => {
                println!("{}", tokens[1..].join(" "));
            }
            "type" => {
                if tokens.len() < 2 {
                    continue;
                }
                match tokens[1] {
                    "exit" | "echo" | "type" => {
                        println!("{} is a shell builtin", tokens[1]);
                    }
                    _ => {
                        println!("{}: not found", tokens[1]);
                    }
                }
            }
            _ => {
                println!("{}: command not found", tokens[0]);
            }
        }
    }
}
