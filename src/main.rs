mod builtins;
mod completion;
mod io;
mod path;
mod pipeline;

use std::{env, fs};

use completion::ShellHelper;
use pipeline::run_pipeline;

use rustyline::config::Configurer;
use rustyline::history::DefaultHistory;
use rustyline::{CompletionType, Editor};

fn main() -> rustyline::Result<()> {
    let mut editor: Editor<ShellHelper, DefaultHistory> = Editor::new()?;
    editor.set_helper(Some(ShellHelper));
    editor.set_completion_type(CompletionType::List);
    editor.set_history_ignore_dups(false)?;
    if let Some(path) = env::var_os("HISTFILE") {
        editor.load_history(&path)?;
    }

    loop {
        let line = editor.readline("$ ");
        match line {
            Ok(line) => {
                let command = line.trim();
                if command.is_empty() {
                    continue;
                }

                editor.add_history_entry(command)?;

                if command == "exit" {
                    break;
                }

                run_pipeline(command, editor.history_mut());
            }
            Err(_) => break,
        }
    }
    if let Some(path) = env::var_os("HISTFILE") {
        editor.save_history(&path)?;
        // rustyline adds a #V2 header, remove it to match bash behavior
        if let Ok(file) = fs::read_to_string(&path) {
            let content = file
                .lines()
                .filter(|line| *line != "#V2")
                .collect::<Vec<_>>()
                .join("\n");
            let content = if content.is_empty() {
                content
            } else {
                content + "\n"
            };
            fs::write(&path, content).unwrap();
        }
    }
    Ok(())
}
