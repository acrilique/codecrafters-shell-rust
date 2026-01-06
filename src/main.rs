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
        let args_owned = shell_words::split(command).expect("failed to parse command input");

        let tokens: Vec<&str> = args_owned.iter().map(String::as_str).collect();

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
                if tokens.len() < 2 {
                    continue;
                }
                let mut dir = PathBuf::from(tokens[1]);
                if tokens[1] == "~"
                    && let Some(path) = env::home_dir()
                {
                    dir = path;
                }
                if env::set_current_dir(&dir).is_err() {
                    println!("cd: {}: No such file or directory", dir.display());
                }
            }
            _ => {
                if find_in_path(target).is_some()
                    && let Ok(mut child) = Command::new(target).args(&tokens[1..]).spawn()
                {
                    let _ = child.wait();
                } else {
                    println!("{target}: command not found");
                }
            }
        }
    }
}
