use std::env;
use std::io::Write;
use std::path::PathBuf;

use rustyline::history::DefaultHistory;

use crate::io::ShellIO;
use crate::path::find_in_path;

pub const BUILTINS: &[&str] = &["cd", "echo", "exit", "history", "pwd", "type"];

pub fn handle_cd(tokens: &[&str], ctx: &mut ShellIO) {
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

pub fn handle_echo(tokens: &[&str], ctx: &mut ShellIO) {
    writeln!(ctx.stdout, "{}", tokens[1..].join(" ")).unwrap();
}

pub fn handle_history(_tokens: &[&str], history: &DefaultHistory, ctx: &mut ShellIO) {
    history
        .iter()
        .enumerate()
        .for_each(|(i, e)| writeln!(ctx.stdout, "    {}  {e}", i + 1).unwrap());
}

pub fn handle_pwd(ctx: &mut ShellIO) {
    if let Ok(path) = env::current_dir() {
        writeln!(ctx.stdout, "{}", path.display()).unwrap();
    } else {
        writeln!(ctx.stderr, "pwd: can't obtain working directory").unwrap();
    }
}

pub fn handle_type(tokens: &[&str], ctx: &mut ShellIO) {
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
