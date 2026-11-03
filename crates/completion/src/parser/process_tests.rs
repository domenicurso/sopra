use std::{process::Command, time::Instant};

use super::run;

#[test]
fn timeout_terminates_descendants_that_hold_output_pipes() {
    let started = Instant::now();
    let mut command = Command::new("sh");
    command.args(["-c", "sleep 10 & wait"]);

    assert!(run(&mut command).is_none());
    assert!(started.elapsed().as_secs() < 2);
}

#[test]
fn explicit_probe_environment_overrides_the_shell_snapshot() {
    let mut command = Command::new("sh");
    command
        .args(["-c", "printf %s \"$SOPRA_PROBE_MARKER\""])
        .env("SOPRA_PROBE_MARKER", "probe-value");
    assert_eq!(
        super::run_stdout(&mut command).as_deref(),
        Some("probe-value")
    );
}

#[test]
fn stdout_probe_excludes_diagnostics() {
    let mut command = Command::new("sh");
    command.args(["-c", "printf stdout; printf stderr >&2"]);

    assert_eq!(super::run_stdout(&mut command).as_deref(), Some("stdout"));
}

#[test]
fn cancellation_terminates_a_running_process_group_promptly() {
    let control = crate::parser::control::Control::new(std::time::Duration::from_secs(2));
    let cancel = control.clone();
    let task = std::thread::spawn(move || {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 10 & wait"]);
        super::run_controlled(&mut command, &control, false)
    });
    std::thread::sleep(std::time::Duration::from_millis(30));
    let started = Instant::now();
    cancel.cancel();
    assert!(task.join().expect("probe thread").is_none());
    assert!(started.elapsed() < std::time::Duration::from_millis(300));
}

#[test]
fn excessive_output_is_rejected_without_waiting_for_the_timeout() {
    let mut command = Command::new("sh");
    command.args(["-c", "yes excessively-long-output"]);
    assert!(super::run(&mut command).is_none());
}
