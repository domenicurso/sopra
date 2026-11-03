use crate::CompletionEngine;
use std::{
    fs,
    time::{Duration, Instant},
};

#[test]
fn asynchronous_filesystem_results_refresh_the_latest_request_and_observe_changes()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::temp_dir().join(format!("sopra-files-{}", std::process::id()));
    fs::create_dir_all(&directory)?;
    fs::write(directory.join("ready-first"), b"fixture")?;
    let mut engine = CompletionEngine::new().ok_or("worker unavailable")?;
    engine.request("cat ./r", 7, &directory);
    engine.request("cat ./ready", 11, &directory);
    await_file(&mut engine, "ready-first")?;
    fs::remove_file(directory.join("ready-first"))?;
    fs::write(directory.join("ready-second"), b"fixture")?;
    await_file(&mut engine, "ready-second")?;
    drop(engine);
    fs::remove_dir_all(directory)?;
    Ok(())
}

fn await_file(engine: &mut CompletionEngine, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        for response in engine.poll() {
            if response
                .items
                .iter()
                .any(|item| item.insert == format!("./{name}"))
            {
                assert_eq!(response.line, "cat ./ready");
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Err(format!("filesystem result never arrived: {name}").into())
}
