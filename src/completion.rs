use rustyline::completion::{Completer, Pair};
use rustyline::{Context, Helper, Highlighter, Hinter, Validator};

use crate::builtins::BUILTINS;
use crate::path::collect_from_path;

#[derive(Helper, Highlighter, Hinter, Validator)]
pub struct ShellHelper;

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
