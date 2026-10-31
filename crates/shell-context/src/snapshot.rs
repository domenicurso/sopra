use std::{cell::RefCell, collections::BTreeMap, ffi::OsString, sync::Arc};

type Environment = BTreeMap<OsString, OsString>;
thread_local! { static CURRENT: RefCell<Option<Arc<Environment>>> = const { RefCell::new(None) }; }

/// An immutable shell environment captured when work is queued.
#[derive(Clone, Debug)]
pub struct Snapshot(Arc<Environment>);
impl Snapshot {
    pub fn capture() -> Self {
        Self(Arc::new(super::vars_os()))
    }

    /// Keep nested probes on the request's environment, even after the next command.
    pub fn with<T>(&self, operation: impl FnOnce() -> T) -> T {
        let previous = CURRENT.with(|slot| slot.replace(Some(self.0.clone())));
        let _scope = Scope(previous);
        operation()
    }
}
struct Scope(Option<Arc<Environment>>);
impl Drop for Scope {
    fn drop(&mut self) {
        CURRENT.with(|slot| slot.replace(self.0.take()));
    }
}
pub(super) fn current() -> Option<Environment> {
    CURRENT.with(|slot| slot.borrow().as_ref().map(|values| values.as_ref().clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_snapshots_restore_the_callers_environment() {
        let first = Snapshot(Arc::new(BTreeMap::from([("VALUE".into(), "first".into())])));
        let second = Snapshot(Arc::new(BTreeMap::from([(
            "VALUE".into(),
            "second".into(),
        )])));
        first.with(|| {
            assert_eq!(super::super::var("VALUE").ok().as_deref(), Some("first"));
            second.with(|| assert_eq!(super::super::var("VALUE").ok().as_deref(), Some("second")));
            assert_eq!(super::super::var("VALUE").ok().as_deref(), Some("first"));
        });
    }
}
