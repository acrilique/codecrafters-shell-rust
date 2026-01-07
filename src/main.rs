mod builtins;
mod completion;
mod io;
mod path;
mod pipeline;

use completion::ShellHelper;
use pipeline::run_pipeline;

use rustyline::config::Configurer;
use rustyline::{CompletionType, Editor};

fn main() -> rustyline::Result<()> {
    let mut editor: Editor<ShellHelper, _> = Editor::new()?;
    editor.set_helper(Some(ShellHelper));
    editor.set_completion_type(CompletionType::List);

    loop {
        let line = editor.readline("$ ");
        match line {
            Ok(line) => {
                let command = line.trim();
                if command.is_empty() {
                    continue;
                }

                // Check for exit before running pipeline
                if command == "exit" {
                    break;
                }

                run_pipeline(command);
            }
            Err(_) => break,
        }
    }
    Ok(())
}
