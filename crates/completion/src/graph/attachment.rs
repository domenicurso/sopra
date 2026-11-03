use super::ValueAttachment;
impl ValueAttachment {
    pub(crate) fn merge(self, incoming: Self) -> Self {
        if self == incoming {
            return self;
        }
        Self::Either
    }
}

impl ValueAttachment {
    pub(crate) fn separate(self) -> bool {
        matches!(self, Self::Separate | Self::Either)
    }

    pub(crate) fn value_start(self, name: &str, token: &str) -> Option<usize> {
        let suffix = token.strip_prefix(name)?;
        let separator = match self {
            Self::Separate => return None,
            Self::Equals => '=',
            Self::Colon => ':',
            Self::Attached | Self::Either if name.ends_with(':') => return Some(name.len()),
            Self::Attached | Self::Either if name.starts_with("--") => '=',
            Self::Attached | Self::Either => return (!suffix.is_empty()).then_some(name.len()),
        };
        suffix.starts_with(separator).then_some(name.len() + 1)
    }
}
