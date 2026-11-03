use super::{CommandNode, Synopsis};

impl CommandNode {
    pub(crate) fn constrain(&mut self, synopsis: &Synopsis) {
        match synopsis {
            Synopsis::Choice(branches) => {
                let names = branches
                    .iter()
                    .map(|branch| {
                        let options = tokens(branch)
                            .into_iter()
                            .filter(|token| token.starts_with('-'))
                            .collect::<Vec<_>>();
                        (options.len() == 1).then(|| {
                            options[0]
                                .split('=')
                                .next()
                                .unwrap_or(options[0])
                                .to_string()
                        })
                    })
                    .collect::<Option<Vec<_>>>();
                if let Some(names) = names
                    && names.len() > 1
                    && !self.exclusive.contains(&names)
                {
                    self.exclusive.push(names);
                }
                for branch in branches {
                    self.constrain(&Synopsis::Sequence(branch.clone()));
                }
            }
            Synopsis::Sequence(nodes) | Synopsis::Optional(nodes) => {
                for node in nodes {
                    self.constrain(node);
                }
            }
            Synopsis::Repeat(node) => self.constrain(node),
            Synopsis::Token(_) => {}
        }
    }
}

fn tokens(nodes: &[Synopsis]) -> Vec<&str> {
    nodes
        .iter()
        .flat_map(|node| match node {
            Synopsis::Token(token) => vec![token.as_str()],
            Synopsis::Sequence(nodes) => tokens(nodes),
            _ => Vec::new(),
        })
        .collect()
}
