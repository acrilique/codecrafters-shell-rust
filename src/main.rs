use is_executable::IsExecutable;
use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

struct ShellIO<'a> {
    pub stdout: Box<dyn Write + 'a>,
    pub stderr: Box<dyn Write + 'a>,
    pub capture_stdout: bool,
    pub capture_stderr: bool,
}

impl<'a> ShellIO<'a> {
    fn new() -> Self {
        Self {
            stdout: Box::new(io::stdout()),
            stderr: Box::new(io::stderr()),
            capture_stdout: false,
            capture_stderr: false,
        }
    }

    fn new_capture_stdout(writer: impl Write + 'a) -> Self {
        Self {
            stdout: Box::new(writer),
            stderr: Box::new(io::stderr()),
            capture_stdout: true,
            capture_stderr: false,
        }
    }

    fn new_capture_stderr(writer: impl Write + 'a) -> Self {
        Self {
            stdout: Box::new(io::stdout()),
            stderr: Box::new(writer),
            capture_stdout: false,
            capture_stderr: true,
        }
    }

    fn new_capture_both(stdout_writer: impl Write + 'a, stderr_writer: impl Write + 'a) -> Self {
        Self {
            stdout: Box::new(stdout_writer),
            stderr: Box::new(stderr_writer),
            capture_stdout: true,
            capture_stderr: true,
        }
    }
}

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

fn handle_type(tokens: &[&str], ctx: &mut ShellIO) {
    if tokens.len() > 1 {
        let target = tokens[1];
        match target {
            "exit" | "echo" | "type" | "pwd" | "cd" => println!("{target} is a shell builtin"),
            _ => {
                if let Some(path) = find_in_path(target) {
                    writeln!(ctx.stdout, "{} is {}", target, path.display()).unwrap();
                } else {
                    writeln!(ctx.stderr, "{target}: not found").unwrap();
                }
            }
        }
    }
}

fn handle_pwd(ctx: &mut ShellIO) {
    if let Ok(path) = env::current_dir() {
        writeln!(ctx.stdout, "{}", path.display()).unwrap();
    } else {
        writeln!(ctx.stderr, "pwd: can't obtain working directory").unwrap();
    }
}

fn handle_cd(tokens: &[&str], ctx: &mut ShellIO) {
    if tokens.len() > 1 {
        let mut dir = PathBuf::from(tokens[1]);
        if tokens[1] == "~"
            && let Some(path) = env::home_dir()
        {
            dir = path;
        }
        if env::set_current_dir(&dir).is_err() {
            writeln!(
                ctx.stderr,
                "cd: {}: No such file or directory",
                dir.display()
            )
            .unwrap();
        }
    }
}

fn handle_not_builtin(tokens: &[&str], ctx: &mut ShellIO) {
    let target = tokens[0];

    let stdout_cfg = if ctx.capture_stdout {
        Stdio::piped()
    } else {
        Stdio::inherit()
    };
    let stderr_cfg = if ctx.capture_stderr {
        Stdio::piped()
    } else {
        Stdio::inherit()
    };

    match Command::new(target)
        .args(&tokens[1..])
        .stdout(stdout_cfg)
        .stderr(stderr_cfg)
        .spawn()
    {
        Ok(child) => match child.wait_with_output() {
            Ok(output) => {
                if ctx.capture_stdout {
                    ctx.stdout.write_all(&output.stdout).unwrap();
                }
                if ctx.capture_stderr {
                    ctx.stderr.write_all(&output.stderr).unwrap();
                }
            }
            Err(e) => writeln!(ctx.stderr, "Error waiting for command: {e}").unwrap(),
        },
        Err(_) => {
            writeln!(ctx.stderr, "{target}: command not found").unwrap();
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
        let args_owned = shell_words::split(command).expect("failed to parse command input");
        let mut tokens: Vec<&str> = args_owned.iter().map(String::as_str).collect();

        if tokens.is_empty() {
            continue;
        }

        let redirect_pos = tokens.iter().position(|&t| t == ">" || t == "1>");
        let mut shellio;

        if let Some(pos) = redirect_pos {
            if pos + 1 >= tokens.len() {
                eprintln!("Syntax error: missing filename after redirect token");
                continue;
            }
            let filename = tokens[pos + 1];

            match File::create(filename) {
                Ok(file) => {
                    shellio = ShellIO::new_capture_stdout(file);
                    tokens.truncate(pos);
                }
                Err(e) => {
                    eprintln!("Failed to open file {filename}: {e}");
                    continue;
                }
            }
        } else {
            shellio = ShellIO::new();
        }

        match tokens[0] {
            "exit" => break,
            "echo" => writeln!(shellio.stdout, "{}", tokens[1..].join(" ")).unwrap(),
            "type" => handle_type(&tokens, &mut shellio),
            "pwd" => handle_pwd(&mut shellio),
            "cd" => handle_cd(&tokens, &mut shellio),
            _ => handle_not_builtin(&tokens, &mut shellio),
        }

        if shellio.capture_stdout {}
    }
}
