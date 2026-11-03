use super::super::parse;
use crate::graph::{ValueAttachment, ValueKind};

#[test]
fn qualified_options_headings_preserve_aliases_and_wrapped_descriptions() {
    let graph = parse(
        "Usage: javac <options> <source files>\nwhere possible options include:\n  --class-path <path>, -classpath <path>, -cp <path>\n        Specify where to find user class files\n        and annotation processors\n  -g:{lines,vars,source}  Generate selected debug info\n  -g:none                Generate no debug info\n  -Djava.ext.dirs=<dirs>  Override extension directories\n",
        "javac",
    );
    let path = graph
        .root
        .options
        .iter()
        .find(|option| option.names.contains(&"-classpath".into()))
        .expect("class path aliases");
    assert_eq!(path.names, ["--class-path", "-classpath", "-cp"]);
    assert_eq!(
        path.description.as_deref(),
        Some("Specify where to find user class files and annotation processors")
    );
    let debug = graph
        .root
        .options
        .iter()
        .find(|option| option.names == ["-g:"])
        .expect("colon option");
    assert_eq!(debug.attachment, ValueAttachment::Attached);
    assert_eq!(
        debug.value.as_ref().expect("debug values").choices,
        ["lines", "vars", "source", "none"]
    );
    assert!(
        graph
            .root
            .options
            .iter()
            .any(|option| option.names == ["-Djava.ext.dirs"])
    );
    assert_eq!(graph.root.positionals.len(), 1);
    assert_eq!(graph.root.positionals[0].name, "source files");
    assert_eq!(graph.root.positionals[0].value.kind, ValueKind::File);
}

#[test]
fn optional_colon_values_preserve_base_flags() {
    let graph = parse(
        "Options:\n  -ea[:<package>...]  Enable assertions\n  --help[:compat]    Show help\n  --disable-@files   Disable argument files\n",
        "java",
    );
    for name in ["-ea", "--help", "--disable-@files"] {
        assert!(
            graph
                .root
                .options
                .iter()
                .any(|option| option.names.contains(&name.into())),
            "missing {name}"
        );
    }
    let help = &graph.root.options[1];
    assert!(help.value.as_ref().expect("optional help value").optional);
    assert_eq!(help.attachment, ValueAttachment::Attached);
}

#[test]
fn unsectioned_options_keep_wrapped_descriptions_out_of_the_preamble() {
    let graph = parse(
        "  --add-exports <module>/<package>=<target>\n        Export the package\n        to another module\n  --help-extra  Show extra help\n",
        "javac",
    );
    assert_eq!(graph.root.description, None);
    assert_eq!(
        graph.root.options[0].description.as_deref(),
        Some("Export the package to another module")
    );
}

#[test]
fn required_equals_values_and_quoted_placeholders_keep_their_grammar() {
    let graph = parse(
        "Options:\n  -Djava.ext.dirs=<dirs>  Extension directories\n  -Xplugin:\"name args\"  Run a plugin\n",
        "javac",
    );
    let directory = &graph.root.options[0];
    assert_eq!(directory.attachment, ValueAttachment::Attached);
    assert!(
        !directory
            .value
            .as_ref()
            .expect("required directory")
            .optional
    );
    let plugin = &graph.root.options[1];
    assert!(
        plugin
            .value
            .as_ref()
            .expect("plugin value")
            .choices
            .is_empty()
    );
}
