use is_executable::IsExecutable;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Iterates over all executable files in PATH, calling the provided function for each.
/// Returns early with `Some(T)` if the function returns `Some`, otherwise `None`.
pub fn find_in_path_by<T>(mut f: impl FnMut(&PathBuf, &str) -> Option<T>) -> Option<T> {
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

pub fn find_in_path(command: &str) -> Option<PathBuf> {
    find_in_path_by(|path, name| (name == command).then(|| path.clone()))
}

/// Collects all executables from PATH matching a predicate, avoiding duplicates.
pub fn collect_from_path(mut predicate: impl FnMut(&str) -> bool) -> Vec<String> {
    let mut results = Vec::new();
    find_in_path_by(|_, name| {
        if predicate(name) && !results.contains(&name.to_string()) {
            results.push(name.to_string());
        }
        None::<()> // Never return early, collect all
    });
    results
}
