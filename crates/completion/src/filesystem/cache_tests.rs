use super::*;
use crate::filesystem::loader::{Loader, Reply};
use std::time::Instant;

#[test]
fn stalled_directory_loading_does_not_block_the_editor_and_replies_refresh_candidates() {
    let (requests, pending) = mpsc::sync_channel(16);
    let (replies, responses) = mpsc::channel();
    let (_, invalidations) = mpsc::channel();
    let mut cache = DirectoryCache {
        entries: HashMap::new(),
        invalidations,
        watcher: None,
        loading: Loading::Background(Some(Loader {
            requests,
            replies: responses,
        })),
        pending: HashSet::new(),
    };
    let directory = Path::new("/fixture/slow-mount");
    let started = Instant::now();
    assert!(cache.entries(directory).is_empty());
    assert!(started.elapsed() < std::time::Duration::from_millis(16));
    assert_eq!(pending.try_recv().expect("one queued directory"), directory);
    assert!(cache.entries(directory).is_empty());
    assert!(
        pending.try_recv().is_err(),
        "pending requests are deduplicated"
    );
    replies
        .send(Reply::Loaded(
            directory.into(),
            vec![Entry {
                name: "ready".into(),
                path: directory.join("ready"),
                directory: false,
                modified: None,
            }],
        ))
        .expect("receiver alive");
    assert!(cache.poll());
    assert_eq!(cache.entries(directory)[0].name, "ready");
    replies
        .send(Reply::Invalidated(directory.join("ready")))
        .expect("receiver alive");
    assert!(cache.poll());
    assert!(cache.entries(directory).is_empty());
    assert_eq!(pending.try_recv().expect("reload queued"), directory);
}
