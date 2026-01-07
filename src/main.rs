use is_executable::IsExecutable;
use rustyline::completion::{Completer, Pair};
use rustyline::config::Configurer;
use rustyline::{CompletionType, Context, Editor, Helper, Highlighter, Hinter, Validator};
use std::env;
use std::fs::{self, File, OpenOptions};
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

const BUILTINS: &[&str] = &["cd", "echo", "exit", "pwd", "type"];

#[derive(Helper, Highlighter, Hinter, Validator)]
struct ShellHelper;

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        // Only complete the first word (command position)
        let line_to_cursor = &line[..pos];
        if line_to_cursor.contains(' ') {
            return Ok((0, vec![]));
        }

        let mut candidates: Vec<Pair> = Vec::new();

        // Add matching builtins
        for &builtin in BUILTINS {
            if builtin.starts_with(line_to_cursor) {
                candidates.push(Pair {
                    display: builtin.to_string(),
                    replacement: format!("{builtin} "),
                });
            }
        }

        // Add matching executables from PATH (excluding already-added builtins)
        for name in collect_from_path(|name| name.starts_with(line_to_cursor)) {
            if !candidates.iter().any(|c| c.display == name) {
                candidates.push(Pair {
                    display: name.clone(),
                    replacement: format!("{name} "),
                });
            }
        }

        candidates.sort_by(|a, b| a.display.cmp(&b.display));
        Ok((0, candidates))
    }
}

/// Iterates over all executable files in PATH, calling the provided function for each.
/// Returns early with `Some(T)` if the function returns `Some`, otherwise `None`.
fn find_in_path_by<T>(mut f: impl FnMut(&PathBuf, &str) -> Option<T>) -> Option<T> {
    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_executable()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
                && let Some(result) = f(&path, name)
            {
                return Some(result);
            }
        }
    }
    None
}

fn find_in_path(command: &str) -> Option<PathBuf> {
    find_in_path_by(|path, name| (name == command).then(|| path.clone()))
}

/// Collects all executables from PATH matching a predicate, avoiding duplicates.
fn collect_from_path(mut predicate: impl FnMut(&str) -> bool) -> Vec<String> {
    let mut results = Vec::new();
    find_in_path_by(|_, name| {
        if predicate(name) && !results.contains(&name.to_string()) {
            results.push(name.to_string());
        }
        None::<()> // Never return early, collect all
    });
    results
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
        if BUILTINS.contains(&target) {
            writeln!(ctx.stdout, "{target} is a shell builtin").unwrap();
        } else if let Some(path) = find_in_path(target) {
            writeln!(ctx.stdout, "{} is {}", target, path.display()).unwrap();
        } else {
            writeln!(ctx.stderr, "{target}: not found").unwrap();
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
    let mut editor: Editor<ShellHelper, _> = Editor::new()?;
    editor.set_helper(Some(ShellHelper));
    editor.set_completion_type(CompletionType::List);

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
