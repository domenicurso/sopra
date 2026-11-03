use super::{Output, render};
use crate::cli::help_pages;

#[test]
fn help_pages_wrap_without_losing_commands_or_coloring_redirected_output() {
    for page in [
        &help_pages::ROOT,
        &help_pages::DEV,
        &help_pages::DEBUG,
        &help_pages::GRAPH,
        &help_pages::CACHE,
        &help_pages::CONFIG,
    ] {
        for width in [20, 40, 63, 64, 80, 120] {
            let help = render(
                &Output {
                    width,
                    color: false,
                    terminal: true,
                },
                page,
            );
            assert!(
                help.lines().all(|line| line.len() <= width),
                "{width}: {help}"
            );
            for (name, _) in page.commands {
                assert!(
                    help.split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .contains(name),
                    "{name}: {help}"
                );
            }
            assert!(!help.contains('\x1b'));
        }
    }
    let help = render(
        &Output {
            width: 80,
            color: true,
            terminal: true,
        },
        &help_pages::ROOT,
    );
    assert!(help.contains("\x1b[1;36mrefresh"));
    assert!(help.contains("\x1b[2mReload config"));
}
