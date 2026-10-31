use super::*;
struct Waiting {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
    count: usize,
}
impl Widget for Waiting {
    fn render(&mut self, _: &Context) -> Text {
        if self.count == 0 {
            let _ = self.entered.send(());
            let _ = self.release.recv();
        }
        self.count += 1;
        self.count.to_string().into()
    }
}
#[test]
fn callbacks_do_not_hold_the_display_lock_and_instance_state_survives_events()
-> Result<(), Box<dyn std::error::Error>> {
    let (entered, notified) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let mut background = Background::new(Box::new(Waiting {
        entered,
        release: blocked,
        count: 0,
    }));
    background.update(Context::default());
    notified.recv_timeout(Duration::from_secs(2))?;
    assert!(background.display().parts.is_empty());
    background.handle(Event::Activate);
    release.send(())?;
    let deadline = Instant::now() + Duration::from_secs(2);
    while background.poll_action().is_none() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(background.display(), Text::from("2"));
    Ok(())
}
