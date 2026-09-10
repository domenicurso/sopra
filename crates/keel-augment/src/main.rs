use std::{env, process::ExitCode};

use keel_core::HostSnapshot;
use keel_renderer::Renderer;
use keel_ui::badge_scene;

fn main() -> ExitCode {
    match parse_args() {
        Ok(snapshot) => {
            let snapshot = snapshot.sanitized();
            let label = format!(
                " Keel | {} | {}c:{} ",
                snapshot.keymap,
                snapshot.character_count(),
                snapshot.cursor
            );
            let scene = badge_scene(fit_label(&label, snapshot.columns));
            let renderer = Renderer;
            let frame = renderer.render(&scene, snapshot.columns);
            print!("{}", renderer.to_zsh_prompt(&frame));
            ExitCode::SUCCESS
        }
        Err(message) if message == "help" => {
            println!("usage: keel-augment --buffer TEXT --cursor N --width N [--keymap NAME]");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("keel-augment: {message}");
            ExitCode::from(2)
        }
    }
}

fn parse_args() -> Result<HostSnapshot, String> {
    let mut snapshot = HostSnapshot::default();
    let mut args = env::args().skip(1);

    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--buffer" => snapshot.buffer = args.next().ok_or("--buffer needs a value")?,
            "--cursor" => {
                snapshot.cursor = args
                    .next()
                    .ok_or("--cursor needs a value")?
                    .parse()
                    .map_err(|_| "--cursor must be an integer")?;
            }
            "--width" => {
                snapshot.columns = args
                    .next()
                    .ok_or("--width needs a value")?
                    .parse()
                    .map_err(|_| "--width must be an integer")?;
            }
            "--keymap" => snapshot.keymap = args.next().ok_or("--keymap needs a value")?,
            "--help" | "-h" => return Err("help".to_string()),
            other => return Err(format!("unknown option {other}")),
        }
    }

    Ok(snapshot)
}

fn fit_label(label: &str, columns: u16) -> String {
    let max_label_width = columns.max(1).saturating_sub(4) as usize;
    if label.chars().count() <= max_label_width {
        return label.to_string();
    }

    label.chars().take(max_label_width.max(1)).collect()
}
