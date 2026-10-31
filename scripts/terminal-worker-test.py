#!/usr/bin/env python3
"""Exercise persistent editor sessions and automatically compiled Rust widgets."""
from __future__ import annotations

import os
from pathlib import Path
import subprocess
import tempfile
import time

from terminal_session import TerminalSession

ROOT = Path(__file__).resolve().parent.parent
WIDGET = r'''
#[derive(Default)]
struct Demo { active: bool, initialized: bool }
impl sopra::Widget for Demo {
    fn render(&mut self, context: &sopra::Context) -> sopra::Text {
        // The fixture supplies a writable pid_file and a named child directory.
        if !self.initialized {
            std::fs::write(&context.options["pid_file"], std::process::id().to_string()).unwrap();
            self.initialized = true;
        }
        format!("first:{}:{}", context.environment.get("DEMO_VALUE").map_or("unset", String::as_str),
            context.cwd.file_name().unwrap().to_string_lossy()).into()
    }
    fn interactive(&self) -> bool { true }
    fn active(&self) -> bool { self.active }
    fn handle(&mut self, event: sopra::Event, context: &sopra::Context) -> sopra::Action {
        match event {
            sopra::Event::Activate => self.active = true,
            sopra::Event::Key(sopra::Key::Enter) => {
                self.active = false;
                // The fixture always runs below its temporary parent directory.
                return sopra::Action::ChangeDirectory(context.cwd.parent().unwrap().into());
            }
            sopra::Event::Key(sopra::Key::Escape) => self.active = false,
            _ => {}
        }
        sopra::Action::Consumed
    }
}
fn main() -> std::io::Result<()> { sopra::run(Demo::default()) }
'''


def submit(terminal: TerminalSession, command: str) -> None:
    terminal.send_text(command)
    terminal.key("ENTER")
    terminal.read(0.6)


def exercise(directory: Path) -> None:
    config = directory / "config" / "sopra"
    widgets = config / "widgets"
    widgets.mkdir(parents=True)
    widget = widgets / "demo.rs"
    widget.write_text(WIDGET)
    pid_file = directory / "widget.pid"
    server_file = directory / "server.pid"
    shell = directory / "shell"
    shell.mkdir()
    work = directory / "parent" / "child"
    work.mkdir(parents=True)
    (config / "config.toml").write_text(
        "[prompt]\nleft = 'ready $ '\nright = '[cyan]$demo[/cyan]'\ntransient = '$ '\n"
        "[completion]\nenabled = false\n[editor]\ncursor_animation = false\n"
        f"[widgets.demo]\npid_file = '{pid_file}'\n")
    (shell / ".zshrc").write_text(f"source '{ROOT}/zsh/editor.zsh'\n")
    environment = {"ZDOTDIR": str(shell), "XDG_CONFIG_HOME": str(config.parent),
                   "XDG_CACHE_HOME": str(directory / "cache"),
                   "SOPRA_BIN": str(ROOT / "target/debug/sopra-worker")}
    with TerminalSession(("zsh", "-di"), rows=20, columns=120,
                         cwd=str(work), env=environment) as terminal:
        server_pid, first_pid = exercise_context(terminal, directory)
        exercise_interaction(terminal, directory, first_pid)
        exercise_reload(terminal, directory, first_pid)
        check_shutdown(terminal, server_pid)


def exercise_context(terminal: TerminalSession, directory: Path) -> tuple[str, str]:
    terminal.wait_for("first:unset:child", timeout=30)
    first_pid = (directory / "widget.pid").read_text()
    submit(terminal, f'print -r -- "$_SOPRA_SERVER_PID" > {(directory / "server.pid")}')
    server_pid = (directory / "server.pid").read_text()
    submit(terminal, "export DEMO_VALUE=changed; alias demo_alias='echo alias-updated'")
    terminal.wait_for("first:changed:child", timeout=10)
    submit(terminal, "demo_alias")
    assert "alias-updated" in terminal.screen.lines(), terminal.screen.text()
    submit(terminal, f'print -r -- "$_SOPRA_SERVER_PID" > {(directory / "server.pid")}')
    assert (directory / "server.pid").read_text() == server_pid
    assert (directory / "widget.pid").read_text() == first_pid
    assert b"\x1b[36m" in terminal.output
    # The editor relinquishes terminal input while a foreground program runs.
    submit(terminal, "read -r answer; print -r -- received:$answer")
    submit(terminal, "terminal-input")
    assert "received:terminal-input" in terminal.screen.lines(), terminal.screen.text()
    return server_pid, first_pid


def exercise_interaction(terminal: TerminalSession, directory: Path, first_pid: str) -> None:
    for width in [*range(120, 39, -2), *range(40, 121, 2)]:
        terminal.resize(20, width)
        terminal.read(0.005)
    terminal.read(0.3)
    assert terminal.screen.text().count("ready $") == 1, terminal.screen.text()
    terminal.send(b"\x1bg")
    terminal.read(0.3)
    terminal.key("ENTER")
    terminal.wait_for("first:changed:parent", timeout=10)
    assert (directory / "widget.pid").read_text() == first_pid


def exercise_reload(terminal: TerminalSession, directory: Path, first_pid: str) -> None:
    widget = directory / "config/sopra/widgets/demo.rs"
    widget.write_text(WIDGET.replace("first:", "updated:"))
    terminal.wait_for("updated:changed:parent", timeout=30)
    assert (directory / "widget.pid").read_text() != first_pid
    binaries = list((directory / "cache").rglob("widget"))
    cache_times = {path: path.stat().st_mtime_ns for path in binaries}
    widget.write_text("this is invalid Rust")
    terminal.wait_for("widget error", timeout=30)
    submit(terminal, "echo editor-still-works")
    assert "editor-still-works" in terminal.screen.lines(), terminal.screen.text()
    widget.write_text(WIDGET.replace("first:", "updated:"))
    terminal.wait_for("updated:changed:parent", timeout=30)
    assert all(path.stat().st_mtime_ns == modified for path, modified in cache_times.items())
    widget.write_text(WIDGET.replace('fn render(&mut self, context: &sopra::Context) -> sopra::Text {',
        'fn render(&mut self, context: &sopra::Context) -> sopra::Text { std::thread::sleep(std::time::Duration::from_secs(10));'))
    terminal.wait_for("widget error", timeout=30)
    assert "100 ms" in (directory / "cache/sopra/widgets/errors.log").read_text()
    submit(terminal, "echo timeout-isolated")
    assert "timeout-isolated" in terminal.screen.lines(), terminal.screen.text()
    widget.write_text(WIDGET.replace("first:", "updated:"))
    terminal.wait_for("updated:changed:parent", timeout=30)


def check_shutdown(terminal: TerminalSession, server_pid: str) -> None:
    submit(terminal, "exit")
    terminal.read(0.3)
    try:
        os.kill(int(server_pid), 0)
    except ProcessLookupError:
        pass
    else:
        raise AssertionError("editor worker survived its shell")


def exercise_shared_git_sdk(directory: Path) -> None:
    work = directory / "shared-git"
    config = work / "config/sopra"
    widgets = config / "widgets"
    widgets.mkdir(parents=True)
    source = (ROOT / "src/prompt/widgets/builtins/git.rs").read_text()
    (widgets / "git_clone.rs").write_text(source +
        "\nfn main() -> std::io::Result<()> { sopra::run(Git) }\n")
    (config / "config.toml").write_text(
        "[prompt]\nleft = '$git / [yellow]$git_clone[/yellow]'\nright=''\n"
        "[completion]\nenabled=false\n[editor]\ncursor_animation=false\n"
        "[widgets.git]\nlabel=''\n[widgets.git_clone]\nlabel=''\n")
    subprocess.run(("git", "init", "--quiet", "--initial-branch=sdk-branch", str(work)), check=True)
    env = {"XDG_CONFIG_HOME": str(config.parent), "XDG_CACHE_HOME": str(directory / "cache"),
           "SOPRA_REPO_ROOT": str(ROOT), "ZDOTDIR": str(ROOT / "examples/isolated"),
           "SOPRA_BIN": str(ROOT / "target/debug/sopra-worker")}
    with TerminalSession(("zsh", "-di"), rows=12, columns=100, cwd=str(work), env=env) as terminal:
        terminal.wait_for("sdk-branch [1?] / sdk-branch [1?]", timeout=30)
        assert b"\x1b[33m" in terminal.output, terminal.output


def main() -> None:
    subprocess.run(["cargo", "build", "--quiet", "--locked"], cwd=ROOT, check=True)
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="sopra-worker-") as temporary:
        exercise(Path(temporary))
        exercise_shared_git_sdk(Path(temporary))
    print(f"persistent worker: ok (context, handoff, resize, widgets, hot reload, shutdown; {time.monotonic() - started:.1f}s)")


if __name__ == "__main__":
    main()
