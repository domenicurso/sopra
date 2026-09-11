use std::{
    io::{self, BufRead, Write},
    time::Instant,
};

use keel_core::HostSnapshot;
use keel_renderer::{RenderedFrame, Renderer, truncate_to_width};
use keel_scheduler::{FrameClock, InvalidationReason};
use keel_ui::{
    Align, Column, Panel, Row, Rule, Scene, Size, StyleToken, Text, Theme, ratatui_component,
};
use ratatui::widgets::{Borders, LineGauge};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderOutput {
    pub prompt_fragment: String,
    pub right_prompt_fragment: String,
    pub cursor: CursorDirective,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorDirective {
    pub style: CursorStyle,
    pub highlight: Option<HighlightSpan>,
    pub syntax: Vec<HighlightSpan>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AugmentProfile {
    pub prompt: bool,
    pub syntax: bool,
    pub hints: bool,
    pub diagnostics: bool,
    pub full_width: bool,
    pub widget: bool,
}

impl AugmentProfile {
    pub fn from_environment() -> Self {
        let poc = std::env::var("KEEL_AUGMENT_POC")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let is = |name: &str| poc == name || poc == "all";
        let enabled = |name: &str| {
            std::env::var(name)
                .map(|value| {
                    matches!(
                        value.trim().to_ascii_lowercase().as_str(),
                        "1" | "true" | "yes" | "on"
                    )
                })
                .unwrap_or(false)
        };

        Self {
            prompt: is("prompt") || is("dashboard") || enabled("KEEL_AUGMENT_PROMPT"),
            syntax: is("syntax") || is("dashboard") || enabled("KEEL_AUGMENT_SYNTAX"),
            hints: is("hints") || is("dashboard") || enabled("KEEL_AUGMENT_HINTS"),
            diagnostics: is("diagnostics") || is("dashboard") || enabled("KEEL_AUGMENT_DEBUG"),
            full_width: is("dashboard") || enabled("KEEL_AUGMENT_FULL_WIDTH"),
            widget: is("widget") || is("dashboard") || enabled("KEEL_AUGMENT_WIDGET"),
        }
    }

    pub fn name(self) -> &'static str {
        if self.full_width {
            "dashboard"
        } else if self.prompt {
            "prompt"
        } else if self.syntax {
            "syntax"
        } else if self.widget {
            "widget"
        } else if self.hints {
            "hints"
        } else if self.diagnostics {
            "diagnostics"
        } else {
            "status"
        }
    }
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
    profile: AugmentProfile,
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
            profile: AugmentProfile::from_environment(),
            clock: FrameClock::default(),
            last_snapshot: None,
            last_frame: None,
            last_output: RenderOutput {
                prompt_fragment: String::new(),
                right_prompt_fragment: String::new(),
                cursor: CursorDirective {
                    style: CursorStyle::Default,
                    highlight: None,
                    syntax: Vec::new(),
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
        self.render_output(snapshot).right_prompt_fragment
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

        let frame_number = self.stats.rendered_frames.saturating_add(1);
        let scene = context_scene_with_profile(&snapshot, self.theme, self.profile, frame_number);
        let prompt_scene = prompt_scene(&snapshot, self.theme, self.profile);
        let frame = self.renderer.render(&scene, snapshot.columns);
        let prompt_frame = prompt_scene
            .as_ref()
            .map(|scene| self.renderer.render(scene, snapshot.columns));
        let diff = self.renderer.diff(self.last_frame.as_ref(), &frame);
        self.stats.rendered_frames += 1;
        self.stats.changed_cells += diff.changed_cells as u64;
        self.last_output = RenderOutput {
            prompt_fragment: prompt_frame
                .as_ref()
                .map(|frame| self.renderer.to_zsh_prompt(frame))
                .unwrap_or_default(),
            right_prompt_fragment: self.renderer.to_zsh_prompt(&frame),
            cursor: CursorDirective {
                style: self.cursor_style,
                highlight: cursor_highlight(&snapshot),
                syntax: if self.profile.syntax {
                    syntax_highlights(&snapshot)
                } else {
                    Vec::new()
                },
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
    RenderService::default()
        .render_output(snapshot)
        .right_prompt_fragment
}

pub fn render_output_snapshot(snapshot: HostSnapshot) -> RenderOutput {
    RenderService::default().render_output(snapshot)
}

pub fn context_scene(snapshot: &HostSnapshot, theme: Theme) -> Scene {
    context_scene_with_profile(snapshot, theme, AugmentProfile::default(), 0)
}

fn context_scene_with_profile(
    snapshot: &HostSnapshot,
    theme: Theme,
    profile: AugmentProfile,
    frame_number: u64,
) -> Scene {
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
    let mut content = Row::new()
        .child(Text::token("Keel", StyleToken::Accent, theme))
        .child(Text::token(" | ", StyleToken::Muted, theme))
        .child(Text::styled(keymap, theme.value))
        .child(Text::token(" | ", StyleToken::Muted, theme))
        .child(Text::styled(metadata, theme.accent));
    content = content
        .child(Text::token(" | ", StyleToken::Muted, theme))
        .child(Text::styled(status, theme.value));
    if profile.hints {
        content = content
            .child(Text::token(" | ", StyleToken::Muted, theme))
            .child(Text::styled(hint_for(&snapshot), theme.muted));
    }
    if profile.diagnostics {
        content = content
            .child(Text::token(" | ", StyleToken::Muted, theme))
            .child(Text::styled(format!("frame {frame_number}"), theme.muted));
    }

    if profile.widget {
        let progress = if snapshot.grapheme_count() == 0 {
            0.0
        } else {
            (snapshot.cursor as f64 / snapshot.grapheme_count() as f64).clamp(0.0, 1.0)
        };
        let gauge = LineGauge::default()
            .ratio(progress)
            .label(format!("cursor {:.0}%", progress * 100.0))
            .filled_style(theme.accent)
            .unfilled_style(theme.muted);
        content = content
            .child(Text::token(" | ", StyleToken::Muted, theme))
            .child(ratatui_component(
                gauge,
                Size {
                    width: 16,
                    height: 1,
                },
            ));
    }

    Scene::new(Align::right(
        Panel::new(content)
            .style(theme.surface)
            .bordered(theme.border)
            .borders(Borders::LEFT | Borders::RIGHT)
            .padding_horizontal(1),
    ))
}

fn prompt_scene(snapshot: &HostSnapshot, theme: Theme, profile: AugmentProfile) -> Option<Scene> {
    if !profile.prompt {
        return None;
    }

    let snapshot = snapshot.sanitized();
    let cwd = truncate_to_width(&snapshot.cwd, snapshot.columns.saturating_sub(8));
    let context = Row::new()
        .child(Text::token("Keel", StyleToken::Accent, theme))
        .child(Text::token(" | ", StyleToken::Muted, theme))
        .child(Text::styled(cwd, theme.value))
        .child(Text::token(" | ", StyleToken::Muted, theme))
        .child(Text::styled(snapshot.keymap, theme.accent));
    let mut surface = Column::new();
    if profile.full_width {
        surface = surface.child(Rule::horizontal(theme.border));
    }
    let surface = surface
        .child(context)
        .child(Text::token(">", StyleToken::Accent, theme));
    Some(Scene::new(surface))
}

fn hint_for(snapshot: &HostSnapshot) -> String {
    let trimmed = snapshot.buffer.trim_start();
    if trimmed.is_empty() {
        "type to inspect".to_string()
    } else if trimmed == "git" || trimmed.starts_with("git ") {
        "tab: status  diff  log".to_string()
    } else if trimmed.starts_with("cargo") {
        "rust toolchain".to_string()
    } else {
        "zsh editing".to_string()
    }
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

fn syntax_highlights(snapshot: &HostSnapshot) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    let text = &snapshot.buffer;
    let mut start = 0;
    let mut token_start = None;
    let mut quote = None;

    for (index, ch) in text.char_indices() {
        if let Some(active_quote) = quote {
            if ch == active_quote {
                let end = index + ch.len_utf8();
                spans.push(HighlightSpan {
                    start: char_offset(text, token_start.unwrap_or(index)),
                    end: char_offset(text, end),
                    style: "fg=yellow".to_string(),
                });
                quote = None;
                token_start = None;
            }
            continue;
        }

        if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            token_start = Some(index);
            continue;
        }

        if ch.is_whitespace() {
            if let Some(token) = token_start.take() {
                add_token_highlight(text, token, index, start == token, &mut spans);
            }
            start = index + ch.len_utf8();
        } else if token_start.is_none() {
            token_start = Some(index);
        }
    }

    if let Some(token) = token_start {
        add_token_highlight(text, token, text.len(), start == token, &mut spans);
    }
    spans
}

fn add_token_highlight(
    text: &str,
    start: usize,
    end: usize,
    command: bool,
    spans: &mut Vec<HighlightSpan>,
) {
    let token = &text[start..end];
    let style = if command {
        "fg=cyan,bold"
    } else if token.starts_with('-') {
        "fg=magenta"
    } else if token.parse::<f64>().is_ok() {
        "fg=green"
    } else {
        "fg=white"
    };
    spans.push(HighlightSpan {
        start: char_offset(text, start),
        end: char_offset(text, end),
        style: style.to_string(),
    });
}

fn char_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset].chars().count()
}

fn encode_output(output: &RenderOutput) -> String {
    let mut highlights = Vec::new();
    highlights.extend(output.cursor.syntax.iter());
    if let Some(span) = output.cursor.highlight.as_ref() {
        highlights.push(span);
    }
    let highlight_payload = highlights
        .into_iter()
        .map(|span| format!("{},{},{}", span.start, span.end, escape_field(&span.style)))
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "{}\t{}\t{}\t{}",
        escape_field(&output.prompt_fragment),
        escape_field(&output.right_prompt_fragment),
        output.cursor.style.name(),
        highlight_payload
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
        Some("render") if fields.len() == 8 => Ok(Request::Render(HostSnapshot {
            buffer: unescape(fields[1])?,
            cursor: fields[2].parse().map_err(|_| "invalid cursor")?,
            columns: fields[3].parse().map_err(|_| "invalid columns")?,
            rows: fields[4].parse().map_err(|_| "invalid rows")?,
            keymap: unescape(fields[5])?,
            last_status: fields[6].parse().map_err(|_| "invalid status")?,
            cwd: unescape(fields[7])?,
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
    use keel_renderer::Renderer;

    #[test]
    fn server_protocol_round_trips_multiline_and_tabs() {
        let request = parse_request(
            "render\techo\\nhi\\tthere\t2\t80\t24\tmain\t0\t/Users/dom/Projects/keel",
        )
        .unwrap();
        let Request::Render(snapshot) = request else {
            panic!("expected render request");
        };
        assert_eq!(snapshot.buffer, "echo\nhi\tthere");
        assert_eq!(snapshot.cwd, "/Users/dom/Projects/keel");
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
        assert!(output.prompt_fragment.is_empty());
        assert!(!output.right_prompt_fragment.is_empty());
        assert!(output.cursor.syntax.is_empty());
        assert_eq!(service.stats().cursor_directives, 1);
    }

    #[test]
    fn dashboard_prompt_keeps_the_input_prefix_compact() {
        let profile = super::AugmentProfile {
            prompt: true,
            syntax: false,
            hints: false,
            diagnostics: false,
            full_width: true,
            widget: false,
        };
        let scene = super::prompt_scene(&HostSnapshot::default(), super::Theme::default(), profile)
            .expect("dashboard prompt");
        let frame = Renderer.render(&scene, 20);
        let fragment = Renderer.to_zsh_prompt(&frame);

        assert!(fragment.contains(">%{[0m%}"));
    }

    #[test]
    fn dashboard_profile_composes_a_ratatuified_widget_and_diagnostics() {
        let profile = super::AugmentProfile {
            prompt: true,
            syntax: true,
            hints: true,
            diagnostics: true,
            full_width: true,
            widget: true,
        };
        let scene = super::context_scene_with_profile(
            &HostSnapshot {
                buffer: "git status".to_string(),
                cursor: 3,
                ..HostSnapshot::default()
            },
            super::Theme::default(),
            profile,
            4,
        );
        let fragment = Renderer.to_zsh_prompt(&Renderer.render(&scene, 100));

        assert!(fragment.contains("frame 4"));
        assert!(fragment.contains("cursor 30%"));
        assert!(fragment.contains("tab: status"));
    }

    #[test]
    fn syntax_highlights_use_zsh_character_offsets_for_unicode() {
        let spans = super::syntax_highlights(&HostSnapshot {
            buffer: "echo \"hi界\" --flag".to_string(),
            ..HostSnapshot::default()
        });

        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, 4);
        assert_eq!(spans[1].start, 5);
        assert_eq!(spans[1].end, 10);
        assert_eq!(spans[2].start, 11);
        assert_eq!(spans[2].end, 17);
    }
}
