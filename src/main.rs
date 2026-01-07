use is_executable::IsExecutable;
use rustyline::Editor;
use rustyline::completion::Completer;
use std::env;
use std::fs::{File, OpenOptions};
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

struct MyHelper;

impl rustyline::Helper for MyHelper {}
impl Completer for MyHelper {
    type Candidate = &'static str;
    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        if line.is_empty() {
            return Ok((0, vec![]));
        }
        if "exit".starts_with(&line[..pos]) {
            return Ok((0, vec!["exit"]));
        }
        if "echo".starts_with(&line[..pos]) {
            return Ok((0, vec!["echo"]));
        }
        if "type".starts_with(&line[..pos]) {
            return Ok((0, vec!["type"]));
        }
        if "pwd".starts_with(&line[..pos]) {
            return Ok((0, vec!["pwd"]));
        }
        if "cd".starts_with(&line[..pos]) {
            return Ok((0, vec!["cd"]));
        }
        Ok((0, vec![]))
    }
}
impl rustyline::hint::Hinter for MyHelper {
    type Hint = &'static str;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<Self::Hint> {
        None
    }
}
impl rustyline::highlight::Highlighter for MyHelper {}
impl rustyline::validate::Validator for MyHelper {}

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
fn setup_redirections<'a>(tokens: &mut Vec<&str>) -> Result<ShellIO<'a>, String> {
    let mut stdout_file: Option<File> = None;
    let mut stderr_file: Option<File> = None;

    let mut clean_tokens = Vec::new();
    let mut i = 0;

    let open = |path: &str, append: bool| -> Result<File, String> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(!append)
            .append(append)
            .open(path)
            .map_err(|e| format!("Failed to open {path}: {e}"))
    };

    while i < tokens.len() {
        let token = tokens[i];
        match token {
            // --- Standard Output Redirects ---
            ">" | "1>" => {
                if i + 1 >= tokens.len() {
                    return Err("Missing filename for stdout".into());
                }
                stdout_file = Some(open(tokens[i + 1], false)?);
                i += 2;
            }
            ">>" | "1>>" => {
                if i + 1 >= tokens.len() {
                    return Err("Missing filename for stdout append".into());
                }
                stdout_file = Some(open(tokens[i + 1], true)?);
                i += 2;
            }

            // --- Standard Error Redirects ---
            "2>" => {
                if i + 1 >= tokens.len() {
                    return Err("Missing filename for stderr".into());
                }
                stderr_file = Some(open(tokens[i + 1], false)?);
                i += 2;
            }
            "2>>" => {
                if i + 1 >= tokens.len() {
                    return Err("Missing filename for stderr append".into());
                }
                stderr_file = Some(open(tokens[i + 1], true)?);
                i += 2;
            }

            // --- Special Redirects ---
            "&>" => {
                // Redirect BOTH to same file (overwrite)
                if i + 1 >= tokens.len() {
                    return Err("Missing filename for &>".into());
                }
                let f = open(tokens[i + 1], false)?;
                // We must clone the file handle so both streams can write to it independently
                stderr_file = Some(f.try_clone().map_err(|e| e.to_string())?);
                stdout_file = Some(f);
                i += 2;
            }

            "2>&1" => {
                // Merge stderr into stdout
                // If stdout is currently a file, clone it for stderr.
                // If stdout is currently None (terminal), set stderr to None (terminal).
                if let Some(ref out) = stdout_file {
                    stderr_file = Some(out.try_clone().map_err(|e| e.to_string())?);
                } else {
                    stderr_file = None;
                }
                i += 1; // This token doesn't take a filename argument
            }

            // --- Normal Arguments ---
            _ => {
                clean_tokens.push(token);
                i += 1;
            }
        }
    }

    *tokens = clean_tokens;

    // Construct the ShellIO based on the final state of our file handles
    match (stdout_file, stderr_file) {
        (Some(out), Some(err)) => Ok(ShellIO::new_capture_both(out, err)),
        (Some(out), None) => Ok(ShellIO::new_capture_stdout(out)),
        (None, Some(err)) => Ok(ShellIO::new_capture_stderr(err)),
        (None, None) => Ok(ShellIO::new()),
    }
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

fn main() -> rustyline::Result<()> {
    let mut editor: Editor<MyHelper, _> = Editor::new()?;
    editor.set_helper(Some(MyHelper));

    loop {
        let line = editor.readline("$ ");
        match line {
            Ok(line) => {
                let command = line.trim();
                let args_owned =
                    shell_words::split(command).expect("failed to parse command input");
                let mut tokens: Vec<&str> = args_owned.iter().map(String::as_str).collect();

                if tokens.is_empty() {
                    continue;
                }

                let mut shellio = match setup_redirections(&mut tokens) {
                    Ok(io) => io,
                    Err(e) => {
                        eprintln!("{e}");
                        continue;
                    }
                };

                if tokens.is_empty() {
                    continue;
                }

                match tokens[0] {
                    "exit" => break,
                    "echo" => writeln!(shellio.stdout, "{}", tokens[1..].join(" ")).unwrap(),
                    "type" => handle_type(&tokens, &mut shellio),
                    "pwd" => handle_pwd(&mut shellio),
                    "cd" => handle_cd(&tokens, &mut shellio),
                    _ => handle_not_builtin(&tokens, &mut shellio),
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}
