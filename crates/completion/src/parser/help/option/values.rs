use super::choices::choices;
use crate::graph::ValueSpec;

pub(crate) fn value_spec(name: &str, optional: bool) -> ValueSpec {
    let mut value = ValueSpec::named(clean_value_name(name));
    value.optional = optional;
    value.choices = choices(name);
    value
}

pub(super) fn literal_value_spec(value: &str) -> ValueSpec {
    let mut spec = value_spec("value", false);
    spec.literal = Some(value.to_string());
    spec
}

pub(super) fn description_value(description: &str) -> Option<ValueSpec> {
    let lower = description.to_ascii_lowercase();
    let name = [
        "[string]",
        "[number]",
        "[array]",
        "[file]",
        "[path]",
        "[directory]",
    ]
    .iter()
    .find(|tag| lower.contains(**tag))
    .map(|tag| tag.trim_matches(['[', ']']))
    .or_else(|| lower.contains("requires an argument").then_some("value"))?;
    let mut value = value_spec(name, false);
    value.choices = choices(description);
    Some(value)
}

fn clean_value_name(name: &str) -> String {
    name.trim_end_matches(',')
        .trim_end_matches("...")
        .trim_matches(['<', '>', '[', ']', '=', ','])
        .trim()
        .to_string()
}
