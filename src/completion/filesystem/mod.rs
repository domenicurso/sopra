mod cache;
mod path;

use std::{ops::Range, path::PathBuf};

use neo_frizbee::{Config, Matcher};

use super::{CompletionItem, CompletionKind, CompletionSource, Request};
use cache::{DirectoryCache, Entry};
use path::{ParsedPath, hidden};

const BEAM_WIDTH: usize = 24;
const MAX_RESULTS: usize = 256;

pub(super) struct FilesystemEngine {
    cache: DirectoryCache,
}

#[derive(Clone)]
struct Branch {
    directory: PathBuf,
    segments: Vec<String>,
    score: u32,
}

impl FilesystemEngine {
    pub(super) fn new() -> Self {
        Self {
            cache: DirectoryCache::new(),
        }
    }

    pub(super) fn complete(&mut self, request: &Request, token: &str) -> Vec<CompletionItem> {
        let Some(path) = ParsedPath::parse(token, &request.cwd) else {
            return Vec::new();
        };
        let config = Config::default();
        let branches = self.expand(path.base.clone(), &path.segments, &config);
        let mut results = branches
            .into_iter()
            .flat_map(|branch| self.complete_leaf(&path, branch, request.replace.clone(), &config))
            .collect::<Vec<_>>();
        results.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.display.cmp(&right.display))
        });
        results.truncate(MAX_RESULTS);
        results
    }

    fn expand(&mut self, base: PathBuf, segments: &[String], config: &Config) -> Vec<Branch> {
        let mut branches = vec![Branch {
            directory: base,
            segments: Vec::new(),
            score: 0,
        }];
        for segment in segments {
            let mut next = Vec::new();
            for branch in branches {
                if let Some(next_branch) = explicit_directory(&branch, segment) {
                    next.push(next_branch);
                    continue;
                }
                for entry in self.cache.entries(&branch.directory) {
                    if !entry.directory || hidden(&entry.name, segment) {
                        continue;
                    }
                    let Some(matched) =
                        Matcher::new(segment, config).match_one_indices(&entry.name, 0)
                    else {
                        continue;
                    };
                    let mut path = branch.segments.clone();
                    path.push(entry.name.clone());
                    next.push(Branch {
                        directory: entry.path,
                        segments: path,
                        score: branch.score.saturating_add(u32::from(matched.score)),
                    });
                }
            }
            next.sort_by_key(|branch| std::cmp::Reverse(branch.score));
            next.truncate(BEAM_WIDTH);
            branches = next;
            if branches.is_empty() {
                break;
            }
        }
        branches
    }

    fn complete_leaf(
        &mut self,
        path: &ParsedPath,
        branch: Branch,
        replace: Range<usize>,
        config: &Config,
    ) -> Vec<CompletionItem> {
        self.cache
            .entries(&branch.directory)
            .into_iter()
            .filter(|entry| !hidden(&entry.name, &path.leaf))
            .filter_map(|entry| leaf_item(path, &branch, entry, replace.clone(), config))
            .collect()
    }
}

fn explicit_directory(branch: &Branch, segment: &str) -> Option<Branch> {
    let directory = match segment {
        "." => branch.directory.clone(),
        ".." => branch.directory.parent()?.to_path_buf(),
        _ => return None,
    };
    let mut segments = branch.segments.clone();
    segments.push(segment.to_string());
    Some(Branch {
        directory,
        segments,
        score: branch.score,
    })
}

fn leaf_item(
    path: &ParsedPath,
    branch: &Branch,
    entry: Entry,
    replace: Range<usize>,
    config: &Config,
) -> Option<CompletionItem> {
    let score = if path.leaf.is_empty() {
        0
    } else {
        Matcher::new(&path.leaf, config)
            .match_one_indices(&entry.name, 0)?
            .score
            .into()
    };
    let suffix = if entry.directory { "/" } else { "" };
    let display = format!("{}{suffix}", entry.name);
    let insert = path.render(&branch.segments, &entry.name, entry.directory);
    let mut item = CompletionItem::with_range(
        display,
        insert,
        None,
        if entry.directory {
            CompletionKind::Directory
        } else {
            CompletionKind::File
        },
        replace,
        CompletionSource::Filesystem,
    );
    item.location = Some(entry.path);
    item.score = branch.score.saturating_add(score) as f32;
    Some(item)
}

#[cfg(test)]
mod tests {
    use super::path::ParsedPath;

    #[test]
    fn parsed_paths_keep_shell_prefixes() {
        let path =
            ParsedPath::parse("--file=./src/ed", std::path::Path::new("/tmp")).expect("path");
        assert_eq!(path.option_prefix, "--file=");
        assert_eq!(path.display_prefix, "./");
        assert_eq!(path.segments, ["src"]);
        assert_eq!(path.leaf, "ed");
    }

    #[test]
    fn relative_paths_do_not_gain_a_leading_separator() {
        let path = ParsedPath::parse("src/ma", std::path::Path::new("/tmp")).expect("path");
        assert_eq!(path.render(&path.segments, "main.rs", false), "src/main.rs");
    }
}
