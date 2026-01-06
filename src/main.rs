use is_executable::IsExecutable;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

fn find_in_path(command: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let full_path = dir.join(command);
            if full_path.is_executable() {
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

        let target = tokens[0];
        match target {
            "exit" => break,
            "echo" => println!("{}", tokens[1..].join(" ")),
            "type" => {
                if tokens.len() < 2 {
                    continue;
                }
                let target = tokens[1];
                match target {
                    "exit" | "echo" | "type" | "pwd" | "cd" => {
                        println!("{target} is a shell builtin");
                    }
                    _ => {
                        if let Some(path) = find_in_path(target) {
                            println!("{} is {}", target, path.display());
                        } else {
                            println!("{target}: not found");
                        }
                    }
                }
            }
            "pwd" => {
                if let Ok(path) = env::current_dir() {
                    println!("{}", path.display());
                } else {
                    println!("can't obtain working directory");
                }
            }
            "cd" => {
                if env::set_current_dir(tokens[1]).is_err() {
                    println!("cd: {}: No such file or directory", tokens[1]);
                }
            }
            _ => {
                if find_in_path(target).is_some() {
                    if let Ok(mut child) = Command::new(target).args(&tokens[1..]).spawn() {
                        let _ = child.wait();
                    }
                } else {
                    println!("{target}: command not found");
                }
            }
        }
    }
}
