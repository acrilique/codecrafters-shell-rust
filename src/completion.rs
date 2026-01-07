use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::{Context, Helper, Highlighter, Hinter, Validator};

use crate::builtins::BUILTINS;
use crate::path::collect_from_path;

#[derive(Helper, Highlighter, Hinter, Validator)]
pub struct ShellHelper {
    filename_completer: FilenameCompleter,
}

impl ShellHelper {
    pub fn new() -> Self {
        Self {
            filename_completer: FilenameCompleter::new(),
        }
    }
}

impl Default for ShellHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line_to_cursor = &line[..pos];
        let is_first_word = !line_to_cursor.contains(' ');

        // Always get file completions
        let (start, mut candidates) = self.filename_completer.complete_path(line, pos)?;

        // For the first word, also add builtins and PATH executables
        if is_first_word {
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
        }

        Ok((start, candidates))
    }
}
