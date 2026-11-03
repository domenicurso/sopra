use super::items;
use crate::graph::{CommandNode, PositionalSpec, ValueSpec};

#[test]
fn a_single_action_is_one_suggestion_with_the_leaf_description() {
    let graph = crate::parser::help::parse(
        "Commands:\n  widgets list  List installed widgets\n  dev build     Build source\n  dev test      Test source\n",
        "tool",
    );
    let completions = items(&graph.root.subcommands, 5..7);
    assert_eq!(completions[0].insert, "widgets list");
    assert_eq!(
        completions[0].description.as_deref(),
        Some("List installed widgets")
    );
    assert_eq!(completions[0].replace, 5..7);
    assert_eq!(completions[1].insert, "dev");
}

#[test]
fn intermediate_arguments_and_options_keep_the_group_accessible() {
    let mut graph = crate::parser::help::parse("Commands:\n  config show  Show config\n", "tool");
    graph.root.subcommands[0].positionals.push(PositionalSpec {
        name: "file".into(),
        description: None,
        value: ValueSpec::named("file"),
        optional: true,
        repeatable: false,
    });
    assert_eq!(items(&graph.root.subcommands, 0..0)[0].insert, "config");
    let mut node = CommandNode::named("config");
    node.subcommands.push(CommandNode::named("show"));
    node.options = crate::parser::help::parse("Options:\n  --file <path>  Use file\n", "tool")
        .root
        .options;
    assert_eq!(items(&[node], 0..0)[0].insert, "config");
}

#[test]
fn aliases_preserve_their_spelling_when_completing_a_chain() {
    let mut node = CommandNode::named("widgets");
    node.aliases.push("w".into());
    node.subcommands.push(CommandNode::named("list"));
    let completions = items(&[node], 0..1);
    assert_eq!(
        completions
            .iter()
            .map(|item| item.insert.as_str())
            .collect::<Vec<_>>(),
        ["widgets list", "w list"]
    );
}
