#[allow(unused_imports)]
use std::io::{self, Write};

fn eval(tokens: Vec<&str>) {
    if !tokens.is_empty() {
        match tokens[0] {
            "echo" => {
                println!("{}", tokens[1..].join(" "));
            }
            _ => {
                println!("{}: command not found", tokens[0]);
            }
        }
    }
}

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
        eval(tokens);
    }
}
