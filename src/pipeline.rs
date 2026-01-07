use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};

use rustyline::history::{DefaultHistory};

use crate::builtins::{BUILTINS, handle_cd, handle_echo, handle_history, handle_pwd, handle_type};
use crate::io::{parse_pipeline, setup_redirections, ShellIO};

/// Execute a pipeline of commands
pub fn run_pipeline(input: &str, history: &DefaultHistory) {
    let segments = parse_pipeline(input);

    if segments.is_empty() {
        return;
    }

    // Single command - use the original flow
    if segments.len() == 1 {
        run_single_command(&segments[0], history);
        return;
    }

    // Multiple commands - set up the pipeline
    run_piped_commands(&segments, history);
}

/// Run a single command (no pipes)
fn run_single_command(command: &str, history: &DefaultHistory) {
    let args_owned = match shell_words::split(command) {
        Ok(args) => args,
        Err(_) => {
            eprintln!("failed to parse command input");
            return;
        }
    };
    let mut tokens: Vec<&str> = args_owned.iter().map(String::as_str).collect();

    if tokens.is_empty() {
        return;
    }

    let mut shellio = match setup_redirections(&mut tokens) {
        Ok(io) => io,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    if tokens.is_empty() {
        return;
    }

    match tokens[0] {
        "cd" => handle_cd(&tokens, &mut shellio),
        "echo" => handle_echo(&tokens, &mut shellio),
        "exit" => std::process::exit(0),
        "history" => handle_history(&tokens, history, &mut shellio),
        "pwd" => handle_pwd(&mut shellio),
        "type" => handle_type(&tokens, &mut shellio),
        _ => run_external(&tokens, &mut shellio),
    }
}

/// Run an external (non-builtin) command
fn run_external(tokens: &[&str], ctx: &mut ShellIO) {
    let target = tokens[0];

    let mut cmd = Command::new(target);
    cmd.args(&tokens[1..])
        .stdin(ctx.stdin_stdio())
        .stdout(ctx.stdout_stdio())
        .stderr(ctx.stderr_stdio());

    match cmd.spawn() {
        Ok(mut child) => {
            // If we have stdin data to pipe in, write it
            if let Some(ref mut stdin_data) = ctx.stdin
                && let Some(mut child_stdin) = child.stdin.take() {
                    let mut buffer = Vec::new();
                    let _ = stdin_data.read_to_end(&mut buffer);
                    let _ = child_stdin.write_all(&buffer);
                }

            match child.wait_with_output() {
                Ok(output) => {
                    if ctx.capture_stdout {
                        ctx.stdout.write_all(&output.stdout).unwrap();
                    }
                    if ctx.capture_stderr {
                        ctx.stderr.write_all(&output.stderr).unwrap();
                    }
                }
                Err(e) => writeln!(ctx.stderr, "Error waiting for command: {e}").unwrap(),
            }
        }
        Err(_) => {
            writeln!(ctx.stderr, "{target}: command not found").unwrap();
        }
    }
}

/// Run multiple commands connected by pipes
fn run_piped_commands(segments: &[String], history: &DefaultHistory) {
    let mut children: Vec<Child> = Vec::new();
    let mut prev_stdout: Option<std::process::ChildStdout> = None;

    for (i, segment) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;

        let args_owned = match shell_words::split(segment) {
            Ok(args) => args,
            Err(_) => {
                eprintln!("failed to parse command input");
                return;
            }
        };
        let mut tokens: Vec<&str> = args_owned.iter().map(String::as_str).collect();

        if tokens.is_empty() {
            continue;
        }

        // Only apply redirections on the last command
        let shellio = if is_last {
            match setup_redirections(&mut tokens) {
                Ok(io) => io,
                Err(e) => {
                    eprintln!("{e}");
                    return;
                }
            }
        } else {
            ShellIO::new()
        };

        if tokens.is_empty() {
            continue;
        }

        let cmd_name = tokens[0];

        // Handle builtins in pipeline
        if BUILTINS.contains(&cmd_name) {
            let output = run_builtin_for_pipe(&tokens, history, prev_stdout.take());
            if !is_last {
                // For builtins in the middle, we need to create a pipe manually
                // Store the output in a cursor for the next command
                prev_stdout = None; // Builtins don't produce ChildStdout
                                    // We need a different approach - use the output directly
                if segments.get(i + 1).is_some() {
                    // Run remaining pipeline with this output as input
                    run_pipeline_with_input(history, &segments[i + 1..], output);
                    return;
                }
            } else {
                // Last command, print output
                print!("{}", String::from_utf8_lossy(&output));
            }
            continue;
        }

        // External command
        let stdin_cfg = if prev_stdout.is_some() {
            Stdio::piped()
        } else {
            Stdio::inherit()
        };

        let stdout_cfg = if is_last {
            shellio.stdout_stdio()
        } else {
            Stdio::piped()
        };

        let mut cmd = Command::new(cmd_name);
        cmd.args(&tokens[1..])
            .stdin(stdin_cfg)
            .stdout(stdout_cfg)
            .stderr(shellio.stderr_stdio());

        match cmd.spawn() {
            Ok(mut child) => {
                // Connect previous command's stdout to this command's stdin
                if let Some(mut prev_out) = prev_stdout.take()
                    && let Some(mut child_stdin) = child.stdin.take() {
                        std::thread::spawn(move || {
                            let _ = std::io::copy(&mut prev_out, &mut child_stdin);
                        });
                    }

                // Save stdout for next command
                if !is_last {
                    prev_stdout = child.stdout.take();
                }

                children.push(child);
            }
            Err(_) => {
                eprintln!("{cmd_name}: command not found");
                return;
            }
        }
    }

    // Wait for all children to complete
    for mut child in children {
        let _ = child.wait();
    }
}

/// Run a builtin command and capture its output for piping
fn run_builtin_for_pipe(tokens: &[&str], history: &DefaultHistory, stdin: Option<std::process::ChildStdout>) -> Vec<u8> {
    let mut output = Vec::new();

    {
        let mut shellio = ShellIO::new().with_piped_stdout(&mut output);

        if let Some(stdin_data) = stdin {
            shellio = shellio.with_stdin(stdin_data);
        }

        match tokens[0] {
            "cd" => handle_cd(tokens, &mut shellio),
            "echo" => handle_echo(tokens, &mut shellio),
            "history" => handle_history(tokens, history, &mut shellio),
            "pwd" => handle_pwd(&mut shellio),
            "type" => handle_type(tokens, &mut shellio),
            _ => {}
        }
    }

    output
}

/// Run remaining pipeline segments with given input data
fn run_pipeline_with_input(history: &DefaultHistory, segments: &[String], input: Vec<u8>) {
    if segments.is_empty() {
        print!("{}", String::from_utf8_lossy(&input));
        return;
    }

    let mut prev_data = input;

    for (i, segment) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;

        let args_owned = match shell_words::split(segment) {
            Ok(args) => args,
            Err(_) => {
                eprintln!("failed to parse command input");
                return;
            }
        };
        let mut tokens: Vec<&str> = args_owned.iter().map(String::as_str).collect();

        if tokens.is_empty() {
            continue;
        }

        // Only apply redirections on the last command
        let shellio = if is_last {
            match setup_redirections(&mut tokens) {
                Ok(io) => io,
                Err(e) => {
                    eprintln!("{e}");
                    return;
                }
            }
        } else {
            ShellIO::new()
        };

        if tokens.is_empty() {
            continue;
        }

        let cmd_name = tokens[0];

        // Handle builtins
        if BUILTINS.contains(&cmd_name) {
            let output = run_builtin_with_bytes(&tokens, history, std::mem::take(&mut prev_data));
            if is_last {
                print!("{}", String::from_utf8_lossy(&output));
            } else {
                prev_data = output;
            }
            continue;
        }

        // External command
        let stdout_cfg = if is_last {
            shellio.stdout_stdio()
        } else {
            Stdio::piped()
        };

        let mut cmd = Command::new(cmd_name);
        cmd.args(&tokens[1..])
            .stdin(Stdio::piped())
            .stdout(stdout_cfg)
            .stderr(shellio.stderr_stdio());

        match cmd.spawn() {
            Ok(mut child) => {
                // Write input data to stdin
                if let Some(mut child_stdin) = child.stdin.take() {
                    let data = prev_data.clone();
                    std::thread::spawn(move || {
                        let _ = child_stdin.write_all(&data);
                    });
                }

                match child.wait_with_output() {
                    Ok(output) => {
                        if is_last {
                            if shellio.capture_stdout {
                                // Already handled by wait_with_output writing to file
                            } else {
                                std::io::stdout().write_all(&output.stdout).unwrap();
                            }
                        } else {
                            prev_data = output.stdout;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error waiting for command: {e}");
                        return;
                    }
                }
            }
            Err(_) => {
                eprintln!("{cmd_name}: command not found");
                return;
            }
        }
    }
}

/// Run a builtin with byte input (for pipelines)
fn run_builtin_with_bytes(tokens: &[&str], history: &DefaultHistory, input: Vec<u8>) -> Vec<u8> {
    let mut output = Vec::new();

    {
        let cursor = std::io::Cursor::new(input);
        let mut shellio = ShellIO::new()
            .with_stdin(cursor)
            .with_piped_stdout(&mut output);

        match tokens[0] {
            "cd" => handle_cd(tokens, &mut shellio),
            "echo" => handle_echo(tokens, &mut shellio),
            "history" => handle_history(tokens, history, &mut shellio),
            "pwd" => handle_pwd(&mut shellio),
            "type" => handle_type(tokens, &mut shellio),
            _ => {}
        }
    }

    output
}
