use std::{
    io::{self, BufRead, Write},
    time::Instant,
};

use keel_core::HostSnapshot;
use keel_renderer::{RenderedFrame, Renderer, truncate_to_width};
use keel_scheduler::{FrameClock, InvalidationReason};
use keel_ui::{Align, Panel, Row, Scene, StyleToken, Text, Theme};
use ratatui::widgets::Borders;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderOutput {
    pub prompt_fragment: String,
    pub cursor: CursorDirective,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorDirective {
    pub style: CursorStyle,
    pub highlight: Option<HighlightSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub style: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Default,
    BlinkBlock,
    Block,
    BlinkBar,
    Bar,
    BlinkUnderline,
    Underline,
}

impl CursorStyle {
    pub fn from_environment() -> Self {
        match std::env::var("KEEL_AUGMENT_CURSOR_STYLE")
            .unwrap_or_else(|_| "blink-block".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "default" | "reset" => Self::Default,
            "block" => Self::Block,
            "bar" | "beam" => Self::Bar,
            "blink-bar" | "blink-beam" => Self::BlinkBar,
            "underline" => Self::Underline,
            "blink-underline" => Self::BlinkUnderline,
            _ => Self::BlinkBlock,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::BlinkBlock => "blink-block",
            Self::Block => "block",
            Self::BlinkBar => "blink-bar",
            Self::Bar => "bar",
            Self::BlinkUnderline => "blink-underline",
            Self::Underline => "underline",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ServiceStats {
    pub requests: u64,
    pub rendered_frames: u64,
    pub cache_hits: u64,
    pub changed_cells: u64,
    pub cursor_directives: u64,
}

pub struct RenderService {
    renderer: Renderer,
    theme: Theme,
    clock: FrameClock,
    last_snapshot: Option<HostSnapshot>,
    last_frame: Option<RenderedFrame>,
    last_output: RenderOutput,
    cursor_style: CursorStyle,
    stats: ServiceStats,
}

impl RenderService {
    pub fn new(theme: Theme) -> Self {
        Self {
            renderer: Renderer,
            theme,
            clock: FrameClock::default(),
            last_snapshot: None,
            last_frame: None,
            last_output: RenderOutput {
                prompt_fragment: String::new(),
                cursor: CursorDirective {
                    style: CursorStyle::Default,
                    highlight: None,
                },
            },
            cursor_style: CursorStyle::from_environment(),
            stats: ServiceStats::default(),
        }
    }

    pub fn from_environment() -> Self {
        let theme = std::env::var("KEEL_AUGMENT_THEME")
            .map(|name| Theme::named(&name))
            .unwrap_or_default();
        Self::new(theme)
    }

    pub fn render(&mut self, snapshot: HostSnapshot) -> String {
        self.render_output(snapshot).prompt_fragment
    }

    pub fn render_output(&mut self, snapshot: HostSnapshot) -> RenderOutput {
        let snapshot = snapshot.sanitized();
        self.stats.requests += 1;
        if self.last_snapshot.as_ref() == Some(&snapshot) {
            self.stats.cache_hits += 1;
            return self.last_output.clone();
        }

        let now = Instant::now();
        let reason = match self.last_snapshot.as_ref() {
            Some(previous)
                if previous.columns != snapshot.columns || previous.rows != snapshot.rows =>
            {
                InvalidationReason::Resize
            }
            Some(previous) if previous.cursor != snapshot.cursor => InvalidationReason::CursorMoved,
            _ => InvalidationReason::Typing,
        };
        self.clock.invalidate(reason, now);

        let scene = context_scene(&snapshot, self.theme);
        let frame = self.renderer.render(&scene, snapshot.columns);
        let diff = self.renderer.diff(self.last_frame.as_ref(), &frame);
        self.stats.rendered_frames += 1;
        self.stats.changed_cells += diff.changed_cells as u64;
        self.last_output = RenderOutput {
            prompt_fragment: self.renderer.to_zsh_prompt(&frame),
            cursor: CursorDirective {
                style: self.cursor_style,
                highlight: cursor_highlight(&snapshot),
            },
        };
        self.stats.cursor_directives += 1;
        self.last_snapshot = Some(snapshot);
        self.last_frame = Some(frame);
        self.last_output.clone()
    }

    pub fn stats(&self) -> ServiceStats {
        self.stats
    }
}

impl Default for RenderService {
    fn default() -> Self {
        Self::from_environment()
    }
}

pub fn render_snapshot(snapshot: HostSnapshot) -> String {
    RenderService::default().render(snapshot)
}

pub fn render_output_snapshot(snapshot: HostSnapshot) -> RenderOutput {
    RenderService::default().render_output(snapshot)
}

pub fn context_scene(snapshot: &HostSnapshot, theme: Theme) -> Scene {
    let snapshot = snapshot.sanitized();
    let keymap = truncate_to_width(&snapshot.keymap, snapshot.columns);
    let metadata = format!(
        "{}c:{}",
        snapshot.grapheme_count(),
        snapshot.cursor.min(snapshot.grapheme_count())
    );
    let status = if snapshot.last_status == 0 {
        "ok".to_string()
    } else {
        format!("exit {}", snapshot.last_status)
    };
    let content = Row::new()
        .child(Text::token("Keel", StyleToken::Accent, theme))
        .child(Text::token(" | ", StyleToken::Muted, theme))
        .child(Text::styled(keymap, theme.value))
        .child(Text::token(" | ", StyleToken::Muted, theme))
        .child(Text::styled(metadata, theme.accent));
    let content = content
        .child(Text::token(" | ", StyleToken::Muted, theme))
        .child(Text::styled(status, theme.value));
    Scene::new(Align::right(
        Panel::new(content)
            .style(theme.surface)
            .bordered(theme.border)
            .borders(Borders::LEFT | Borders::RIGHT)
            .padding_horizontal(1),
    ))
}

pub fn run_server() -> io::Result<ServiceStats> {
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    let mut service = RenderService::default();

    for line in stdin.lock().lines() {
        let line = line?;
        match parse_request(&line) {
            Ok(Request::Render(snapshot)) => {
                writeln!(
                    stdout,
                    "{}",
                    encode_output(&service.render_output(snapshot))
                )?;
                stdout.flush()?;
            }
            Ok(Request::Ping) => {
                writeln!(stdout, "pong")?;
                stdout.flush()?;
            }
            Ok(Request::Stats) => {
                let stats = service.stats();
                writeln!(
                    stdout,
                    "requests={} rendered_frames={} cache_hits={} changed_cells={} cursor_directives={}",
                    stats.requests,
                    stats.rendered_frames,
                    stats.cache_hits,
                    stats.changed_cells,
                    stats.cursor_directives
                )?;
                stdout.flush()?;
            }
            Ok(Request::Shutdown) => {
                writeln!(stdout, "bye")?;
                stdout.flush()?;
                break;
            }
            Err(_) => {
                writeln!(stdout)?;
                stdout.flush()?;
            }
        }
    }

    Ok(service.stats())
}

fn cursor_highlight(snapshot: &HostSnapshot) -> Option<HighlightSpan> {
    if std::env::var("KEEL_AUGMENT_CURSOR_MODE")
        .unwrap_or_else(|_| "native".to_string())
        .trim()
        .eq_ignore_ascii_case("highlight")
    {
        let cursor = snapshot.cursor.min(snapshot.buffer.chars().count());
        let mut start = 0;
        for grapheme in snapshot.buffer.graphemes(true) {
            let end = start + grapheme.chars().count();
            if cursor < end {
                return Some(HighlightSpan {
                    start,
                    end,
                    style: std::env::var("KEEL_AUGMENT_CURSOR_HIGHLIGHT")
                        .unwrap_or_else(|_| "fg=black,bg=cyan,bold".to_string()),
                });
            }
            start = end;
        }
    }
    None
}

fn encode_output(output: &RenderOutput) -> String {
    let (start, end, style) = output
        .cursor
        .highlight
        .as_ref()
        .map(|span| {
            (
                span.start.to_string(),
                span.end.to_string(),
                escape_field(&span.style),
            )
        })
        .unwrap_or_else(|| ("-".to_string(), "-".to_string(), "-".to_string()));
    format!(
        "{}\t{}\t{}\t{}\t{}",
        escape_field(&output.prompt_fragment),
        output.cursor.style.name(),
        start,
        end,
        style
    )
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

pub fn parse_one_shot_args<I>(args: I) -> Result<HostSnapshot, String>
where
    I: IntoIterator<Item = String>,
{
    let mut snapshot = HostSnapshot::default();
    let mut args = args.into_iter();

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
            "--rows" => {
                snapshot.rows = args
                    .next()
                    .ok_or("--rows needs a value")?
                    .parse()
                    .map_err(|_| "--rows must be an integer")?;
            }
            "--keymap" => snapshot.keymap = args.next().ok_or("--keymap needs a value")?,
            "--status" => {
                snapshot.last_status = args
                    .next()
                    .ok_or("--status needs a value")?
                    .parse()
                    .map_err(|_| "--status must be an integer")?;
            }
            "--help" | "-h" => return Err("help".to_string()),
            other => return Err(format!("unknown option {other}")),
        }
    }

    Ok(snapshot)
}

enum Request {
    Render(HostSnapshot),
    Ping,
    Stats,
    Shutdown,
}

fn parse_request(line: &str) -> Result<Request, String> {
    let fields = line.trim_end_matches('\r').split('\t').collect::<Vec<_>>();
    match fields.first().copied() {
        Some("render") if fields.len() == 7 => Ok(Request::Render(HostSnapshot {
            buffer: unescape(fields[1])?,
            cursor: fields[2].parse().map_err(|_| "invalid cursor")?,
            columns: fields[3].parse().map_err(|_| "invalid columns")?,
            rows: fields[4].parse().map_err(|_| "invalid rows")?,
            keymap: unescape(fields[5])?,
            last_status: fields[6].parse().map_err(|_| "invalid status")?,
        })),
        Some("ping") if fields.len() == 1 => Ok(Request::Ping),
        Some("stats") if fields.len() == 1 => Ok(Request::Stats),
        Some("shutdown") if fields.len() == 1 => Ok(Request::Shutdown),
        _ => Err("invalid request".to_string()),
    }
}

fn unescape(value: &str) -> Result<String, String> {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('t') => output.push('\t'),
            Some('\\') => output.push('\\'),
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => return Err("unterminated escape".to_string()),
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{CursorStyle, RenderService, Request, parse_request};
    use keel_core::HostSnapshot;

    #[test]
    fn server_protocol_round_trips_multiline_and_tabs() {
        let request = parse_request("render\techo\\nhi\\tthere\t2\t80\t24\tmain\t0").unwrap();
        let Request::Render(snapshot) = request else {
            panic!("expected render request");
        };
        assert_eq!(snapshot.buffer, "echo\nhi\tthere");
    }

    #[test]
    fn service_caches_identical_snapshots() {
        let mut service = RenderService::default();
        let snapshot = HostSnapshot {
            buffer: "echo hi".to_string(),
            cursor: 7,
            ..HostSnapshot::default()
        };

        service.render(snapshot.clone());
        service.render(snapshot);
        assert_eq!(service.stats().rendered_frames, 1);
        assert_eq!(service.stats().cache_hits, 1);
    }

    #[test]
    fn render_output_carries_a_native_cursor_directive() {
        let mut service = RenderService::default();
        let output = service.render_output(HostSnapshot::default());

        assert_eq!(output.cursor.style, CursorStyle::BlinkBlock);
        assert!(output.cursor.highlight.is_none());
        assert!(!output.prompt_fragment.is_empty());
        assert_eq!(service.stats().cursor_directives, 1);
    }
}
