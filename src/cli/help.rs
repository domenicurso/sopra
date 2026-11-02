use super::{
    help_pages::Page,
    output::{self, Output, wrap},
};
use std::io;

pub(super) fn print(page: &Page) -> io::Result<()> {
    output::print(&render(&Output::detect(), page))
}

fn render(output: &Output, page: &Page) -> String {
    let mut result = String::new();
    for line in wrap(page.usage, output.width.saturating_sub(1)) {
        result.push_str(&format!("{}\n", output.style(&line, "1")));
    }
    result.push('\n');
    result.push_str(&format!("{}\n", output.style("Commands", "1")));
    for (name, description) in page.commands {
        if output.width >= 64 {
            let indent = 18;
            let lines = wrap(description, output.width.saturating_sub(indent + 1));
            for (index, line) in lines.iter().enumerate() {
                let label = if index == 0 { *name } else { "" };
                result.push_str(&format!(
                    "  {}{}{}\n",
                    output.style(label, "1;36"),
                    " ".repeat(indent - 2 - label.len()),
                    output.style(line, "2")
                ));
            }
        } else {
            for line in wrap(name, output.width.saturating_sub(3)) {
                result.push_str(&format!("  {}\n", output.style(&line, "1;36")));
            }
            for line in wrap(description, output.width.saturating_sub(5)) {
                result.push_str(&format!("    {}\n", output.style(&line, "2")));
            }
            result.push('\n');
        }
    }
    result.push('\n');
    for note in page.notes {
        for line in wrap(note, output.width.saturating_sub(1)) {
            result.push_str(&output.style(&line, "2"));
            result.push('\n');
        }
    }
    result
}

#[cfg(test)]
#[path = "help_tests.rs"]
mod tests;
