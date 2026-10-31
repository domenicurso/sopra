use super::{
    Action, Widget,
    protocol::{self, Response},
};
use std::io::{self, BufRead, Write};

pub fn run(mut widget: impl Widget) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let (context, event) = protocol::read_request(&line?)?;
        let action = event.map_or(Action::Ignored, |event| widget.handle(event, &context));
        let response = Response {
            text: widget.render(&context),
            action,
            active: widget.active(),
            interactive: widget.interactive(),
            refresh: widget.refresh(),
        };
        writeln!(stdout, "{}", protocol::response(&response))?;
        stdout.flush()?;
    }
    Ok(())
}
