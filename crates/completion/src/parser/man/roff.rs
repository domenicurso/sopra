pub(crate) fn looks_like_source(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with(".TH ")
            || line.starts_with(".SH ")
            || line.starts_with(".Dd ")
            || line.starts_with(".Dt ")
            || line.starts_with(".Sh ")
    })
}

pub(crate) fn to_help_text(text: &str) -> String {
    let mut output = String::new();
    let mut pending_description = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(".\"") {
            continue;
        }
        if let Some((macro_name, args)) = macro_line(trimmed) {
            pending_description = append_macro(&mut output, &macro_name, args, pending_description);
            continue;
        }
        let value = clean(trimmed);
        if !value.is_empty() {
            if pending_description {
                output.push_str("  ");
            }
            output.push_str(&value);
            output.push('\n');
            pending_description = false;
        }
    }
    output
}

fn append_macro(output: &mut String, name: &str, args: &str, pending: bool) -> bool {
    match name {
        "SH" | "Sh" | "SS" | "Ss" => {
            output.push_str(&format!("{}:\n", clean(args)));
            false
        }
        "TP" | "Tp" | "IP" => true,
        "It" => {
            append_line(output, &render_args(args), false);
            true
        }
        "B" | "I" | "BR" | "BI" | "Nm" => {
            let value = render_args(args);
            append_line(output, &value, pending && !value.starts_with('-'));
            value.starts_with('-')
        }
        "Op" => {
            append_line(output, &format!("[{}]", render_args(args)), false);
            pending
        }
        "Fl" | "Ar" => {
            append_line(output, &render_macro(name, args), false);
            pending
        }
        _ => pending,
    }
}

fn append_line(output: &mut String, value: &str, indent: bool) {
    if value.is_empty() {
        return;
    }
    if indent {
        output.push_str("  ");
    }
    output.push_str(value);
    output.push('\n');
}

fn macro_line(line: &str) -> Option<(String, &str)> {
    if !line.starts_with('.') {
        return None;
    }
    let mut parts = line[1..].splitn(2, char::is_whitespace);
    let name = parts.next()?.to_string();
    Some((name, parts.next().unwrap_or_default().trim()))
}

fn render_args(args: &str) -> String {
    let mut output = String::new();
    let mut parts = args.split_whitespace();
    while let Some(part) = parts.next() {
        let value = match part {
            "Fl" | "fl" => parts
                .next()
                .map(|value| format!("-{}", clean(value)))
                .unwrap_or_default(),
            "Ar" | "ar" => parts
                .next()
                .map(|value| format!("<{}>", clean(value)))
                .unwrap_or_default(),
            "Op" | "op" => {
                let rest = parts.collect::<Vec<_>>().join(" ");
                let value = format!("[{}]", render_args(&rest));
                if !value.is_empty() {
                    if !output.is_empty() {
                        output.push(' ');
                    }
                    output.push_str(&value);
                }
                break;
            }
            _ => clean(part),
        };
        if !value.is_empty() {
            if !output.is_empty() {
                output.push(' ');
            }
            output.push_str(&value);
        }
    }
    output
}

fn render_macro(name: &str, args: &str) -> String {
    match name {
        "Fl" => format!("-{}", clean(args)),
        "Ar" => format!("<{}>", clean(args)),
        _ => render_args(args),
    }
}

fn clean(value: &str) -> String {
    value
        .replace("\\fB", "")
        .replace("\\fI", "")
        .replace("\\fR", "")
        .replace("\\-", "-")
        .trim_matches(['\"', '\''])
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{looks_like_source, to_help_text};

    #[test]
    fn converts_mdoc_synopsis_macros() {
        let text = ".Dt TOOL 1\n.Sh SYNOPSIS\n.Nm tool\n.Op Fl o Ar file\n.Sh OPTIONS\n.B Fl v\nverbose output\n";
        let rendered = to_help_text(text);
        assert!(looks_like_source(text));
        assert!(rendered.contains("SYNOPSIS:"));
        assert!(rendered.contains("[-o <file>]"));
        assert!(rendered.contains("-v"));
    }

    #[test]
    fn renders_mdoc_tagged_options_for_the_help_parser() {
        let text = ".Dt TOOL 1\n.Sh SYNOPSIS\n.Nm tool\n.Op Fl v\n.Sh OPTIONS\n.Bl -tag\n.It Fl o Ar file\nwrite to file\n.El\n";
        let graph = crate::parser::man::parse(text, "tool");
        let option = graph
            .root
            .options
            .iter()
            .find(|option| option.names == ["-o"])
            .expect("mdoc option");
        assert_eq!(
            option.value.as_ref().map(|value| value.name.as_str()),
            Some("file")
        );
        assert_eq!(option.description.as_deref(), Some("write to file"));
    }
}
