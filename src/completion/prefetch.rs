use std::path::Path;

use super::{CompletionEngine, Request, active_range, ranking};

impl CompletionEngine {
    pub(super) fn prefetch_argument_context(
        &mut self,
        line: &str,
        cursor: usize,
        cwd: &Path,
    ) -> bool {
        let byte_cursor = ranking::byte_offset(line, cursor);
        if byte_cursor != line.len() || !crate::syntax::command_position(line, byte_cursor) {
            return false;
        }
        let (start, _) = ranking::token_range(line, byte_cursor);
        let query = &line[start..byte_cursor];
        if query.chars().count() < 2 {
            return false;
        }
        let candidates = super::commands::complete(query, start..byte_cursor);
        let ranked = ranking::rank(&candidates, query);
        let Some(candidate) = ranked.first() else {
            return false;
        };

        let mut next_line = line[..start].to_string();
        next_line.push_str(&candidate.display);
        next_line.push(' ');
        let next_cursor = next_line.chars().count();
        let (context_line, context_cursor) = ranking::broad_context(&next_line, next_cursor);
        let context_key = format!("{}\0{}", cwd.display(), context_line);
        if self.cache.contains_key(&context_key)
            || self.pending_context.as_deref() == Some(context_key.as_str())
        {
            return true;
        }
        let request = Request {
            line: next_line.clone(),
            cursor: next_cursor,
            replace: active_range(&next_line, next_cursor),
            context_line,
            context_cursor,
            cwd: cwd.to_path_buf(),
            generation: self.generation,
            context_key: context_key.clone(),
        };
        if candidate.display == query {
            self.schedule_help(&request);
        }
        self.pending_context = Some(context_key);
        let _ = self.requests.send(request);
        true
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::CompletionEngine;

    #[test]
    fn prefetch_uses_the_best_command_match() {
        let mut engine = CompletionEngine::new().expect("completion worker");

        assert!(engine.prefetch_argument_context("git", 3, Path::new(".")));
        assert_eq!(engine.pending_context.as_deref(), Some(".\0git "));
    }
}
