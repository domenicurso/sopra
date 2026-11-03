use super::candidates::complete;
use crate::{Request, filesystem::FilesystemEngine, parser::Invocation};

#[test]
fn colon_values_complete_after_the_separator_and_preserve_the_option_prefix() {
    let graph = crate::parser::help::parse(
        "Options:\n  -g:{lines,vars,source}  Choose debugging details\n",
        "javac",
    );
    for (active, expected, replace) in [
        ("-g:", vec!["lines", "vars", "source"], 9..9),
        ("-g:v", vec!["vars"], 9..10),
    ] {
        let line = format!("javac {active}");
        let request = Request {
            cursor: line.len(),
            context_cursor: line.len(),
            context_line: line.clone(),
            line,
            replace: 6..6 + active.len(),
            cwd: ".".into(),
            generation: 1,
            context_key: String::new(),
        };
        let invocation = Invocation {
            key: "javac".into(),
            command: None,
            program: "javac".into(),
            args: vec![active.into()],
            active: active.into(),
            trailing_space: false,
        };
        let items = complete(
            &graph.root,
            &invocation,
            &request,
            &mut FilesystemEngine::new(),
        );
        assert_eq!(
            items
                .iter()
                .map(|item| item.insert.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        assert!(items.iter().all(|item| item.replace == replace));
    }
}
