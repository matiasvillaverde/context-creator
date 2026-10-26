use super::{
    index::{relative, Snapshot},
    types::*,
};
use anyhow::{bail, Context, Result};
use git2::{DiffFindOptions, DiffOptions, Patch, Repository};
use std::path::Path;

pub(crate) fn changes(root: &Path, baseline: &str, snapshot: &Snapshot) -> Result<Vec<Change>> {
    if baseline.is_empty()
        || baseline.len() > 256
        || baseline.starts_with('-')
        || baseline
            .chars()
            .any(|ch| ch.is_control() || [';', '&', '|', '`', '$', '(', ')'].contains(&ch))
    {
        bail!("Invalid base_ref");
    }
    let repository =
        Repository::discover(root).context("review_change needs a Git working tree")?;
    let workdir = repository
        .workdir()
        .context("A bare repository has no working tree")?
        .canonicalize()?;
    let scope = root
        .strip_prefix(&workdir)
        .context("Repository root is outside the Git working tree")?;
    let tree = repository
        .revparse_single(baseline)
        .context("base_ref does not resolve")?
        .peel_to_tree()?;
    let mut options = DiffOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true)
        .context_lines(0);
    if !scope.as_os_str().is_empty() {
        options.pathspec(scope);
    }
    let mut diff = repository.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut options))?;
    let mut find = DiffFindOptions::new();
    find.renames(true).for_untracked(true);
    diff.find_similar(Some(&mut find))?;
    if diff.deltas().len() > 1000 {
        bail!("Change context is limited to 1000 file changes; narrow the root or baseline");
    }
    let mut changes = Vec::new();
    for (index, delta) in diff.deltas().enumerate() {
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .context("Git delta has no path")?;
        let Ok(scoped) = path.strip_prefix(scope) else {
            continue;
        };
        let Ok(path) = relative(scoped) else {
            continue;
        };
        if crate::core::walker::repository_path_is_ignored(root, &root.join(scoped))? {
            continue;
        }
        let mut ranges = Vec::new();
        if let Some(patch) = Patch::from_diff(&diff, index)? {
            for hunk in 0..patch.num_hunks().min(128) {
                let (hunk, _) = patch.hunk(hunk)?;
                let start = (hunk.new_start() as usize).max(1);
                ranges.push(LineRange {
                    start,
                    end: start + (hunk.new_lines() as usize).saturating_sub(1),
                });
            }
        }
        let mut removed_symbols = Vec::new();
        if let Ok(blob) = repository.find_blob(delta.old_file().id()) {
            if blob.size() <= 2 * 1024 * 1024 {
                if let Ok(content) = std::str::from_utf8(blob.content()) {
                    let old_path = delta.old_file().path().unwrap_or(scoped);
                    if let Ok(old) =
                        crate::core::semantic::symbols::extract_symbols(old_path, content)
                    {
                        for symbol in old {
                            if snapshot.sources.get(&path).is_none_or(|source| {
                                !source
                                    .declarations
                                    .iter()
                                    .any(|declaration| declaration.symbol.name == symbol.name)
                            }) {
                                removed_symbols.push(symbol.name);
                            }
                        }
                    }
                }
            }
        }
        removed_symbols.truncate(128);
        let old_path = delta
            .old_file()
            .path()
            .and_then(|path| path.strip_prefix(scope).ok())
            .and_then(|path| relative(path).ok())
            .filter(|old| old != &path);
        changes.push(Change {
            path,
            old_path,
            status: format!("{:?}", delta.status()).to_ascii_lowercase(),
            ranges,
            removed_symbols,
        });
    }
    changes.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(changes)
}
