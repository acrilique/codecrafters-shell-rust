use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

fn find_in_path(command: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let full_path = dir.join(command);
            if full_path.is_file() {
                Some(full_path)
            } else {
                None
            }
        })
    })
}

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
            "exit" => break,
            "echo" => println!("{}", tokens[1..].join(" ")),
            "type" => {
                if tokens.len() < 2 {
                    continue;
                }
                let target = tokens[1];
                match target {
                    "exit" | "echo" | "type" => println!("{} is a shell builtin", target),
                    _ => {
                        if let Some(path) = find_in_path(target) {
                            println!("{} is {}", target, path.display());
                        } else {
                            println!("{}: not found", target);
                        }
                    }
                }
            }

            _ => println!("{}: command not found", tokens[0]),
        }
    }
}
