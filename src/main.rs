mod builtins;
mod completion;
mod io;
mod path;
mod pipeline;

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
    Ok(())
}
