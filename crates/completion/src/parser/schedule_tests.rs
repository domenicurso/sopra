use super::*;
use crate::inspection::{GraphMode, GraphQuery};

fn request(name: &str) -> HelpRequest {
    let mut request = crate::parser::dump::request(&GraphQuery {
        command: "sh",
        path: &[],
        cwd: std::path::Path::new("/tmp"),
        mode: GraphMode::Root,
        refresh: false,
    })
    .expect("system shell resolves");
    request.key = name.into();
    request
}

#[test]
fn urgent_work_preempts_active_background_and_runs_before_queued_prefetch() {
    let shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
    let dispatcher = Dispatcher {
        shared: shared.clone(),
        thread: None,
    };
    assert!(dispatcher.send(request("background"), true));
    let active = next(&shared).expect("background starts");
    assert!(dispatcher.send(request("later-background"), true));
    assert!(dispatcher.send(request("foreground"), false));
    assert!(active.control.stopped());
    assert_eq!(next(&shared).expect("urgent starts").key, "foreground");
    assert_eq!(
        next(&shared).expect("prefetch follows").key,
        "later-background"
    );
}

#[test]
fn cancellation_and_queue_limit_bound_obsolete_work() {
    let shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
    let dispatcher = Dispatcher {
        shared: shared.clone(),
        thread: None,
    };
    for index in 0..QUEUED_LIMIT {
        assert!(dispatcher.send(request(&index.to_string()), true));
    }
    assert!(!dispatcher.send(request("overflow"), true));
    dispatcher.cancel();
    assert!(
        next(&shared)
            .expect("cancelled response can release pending bookkeeping")
            .control
            .stopped()
    );
}
