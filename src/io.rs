use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::process::Stdio;

pub struct ShellIO<'a> {
    pub stdin: Option<Box<dyn Read + 'a>>,
    pub stdout: Box<dyn Write + 'a>,
    pub stderr: Box<dyn Write + 'a>,
    pub capture_stdout: bool,
    pub capture_stderr: bool,
}

impl<'a> ShellIO<'a> {
    pub fn new() -> Self {
        Self {
            stdin: None,
            stdout: Box::new(io::stdout()),
            stderr: Box::new(io::stderr()),
            capture_stdout: false,
            capture_stderr: false,
        }
    }

    pub fn with_stdin(mut self, stdin: impl Read + 'a) -> Self {
        self.stdin = Some(Box::new(stdin));
        self
    }

    pub fn with_piped_stdout(mut self, stdout: impl Write + 'a) -> Self {
        self.stdout = Box::new(stdout);
        self.capture_stdout = true;
        self
    }

    fn new_capture_stdout(writer: impl Write + 'a) -> Self {
        Self {
            stdin: None,
            stdout: Box::new(writer),
            stderr: Box::new(io::stderr()),
            capture_stdout: true,
            capture_stderr: false,
        }
    }

    fn new_capture_stderr(writer: impl Write + 'a) -> Self {
        Self {
            stdin: None,
            stdout: Box::new(io::stdout()),
            stderr: Box::new(writer),
            capture_stdout: false,
            capture_stderr: true,
        }
    }

    fn new_capture_both(stdout_writer: impl Write + 'a, stderr_writer: impl Write + 'a) -> Self {
        Self {
            stdin: None,
            stdout: Box::new(stdout_writer),
            stderr: Box::new(stderr_writer),
            capture_stdout: true,
            capture_stderr: true,
        }
    }

    pub fn stdin_stdio(&self) -> Stdio {
        if self.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::inherit()
        }
    }

    pub fn stdout_stdio(&self) -> Stdio {
        if self.capture_stdout {
            Stdio::piped()
        } else {
            Stdio::inherit()
        }
    }

    pub fn stderr_stdio(&self) -> Stdio {
        if self.capture_stderr {
            Stdio::piped()
        } else {
            Stdio::inherit()
        }
    }
}

pub fn setup_redirections<'a>(tokens: &mut Vec<&str>) -> Result<ShellIO<'a>, String> {
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

/// Split a command line into pipeline segments.
/// Returns a vector of command strings separated by `|`.
pub fn parse_pipeline(input: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let chars = input.chars();

    for c in chars {
        match c {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                current.push(c);
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                current.push(c);
            }
            '|' if !in_single_quote && !in_double_quote => {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    segments.push(trimmed);
                }
                current.clear();
            }
            _ => current.push(c),
        }
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        segments.push(trimmed);
    }

    segments
}
