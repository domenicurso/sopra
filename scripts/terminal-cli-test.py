#!/usr/bin/env python3
"""Exercise the public CLI and worker lifecycle through an integrated Zsh."""
from __future__ import annotations
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
import time
from terminal_session import TerminalSession

ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "target/debug/sopra-worker"
WIDGET = r'''
struct Demo { count: usize }
impl sopra::Widget for Demo {
    fn render(&mut self, context: &sopra::Context) -> sopra::Text {
        if self.count == 0 {
            // The test supplies a writable path for this instance's identity.
            if let Err(error) = std::fs::write(context.option("pid_file", ""), std::process::id().to_string()) {
                return error.to_string().into();
            }
        }
        self.count += 1;
        format!("{}:{}", context.option("label", "widget"), self.count).into()
    }
}
fn main() -> std::io::Result<()> { sopra::run(Demo { count: 0 }) }
'''


def configuration(directory: Path, label: str = "READY", widget: str = "first") -> str:
    return (f"[prompt]\nleft='{label} $ '\nright='$demo'\ntransient='$ '\n"
            "[completion]\nenabled=false\n[editor]\ncursor_animation=false\nautosuggestions=false\n"
            f"[widgets.demo]\nlabel='{widget}'\npid_file='{directory / 'widget.pid'}'\n")


def run_cli(args: list[str], env: dict[str, str], success: bool = True) -> str:
    result = subprocess.run([str(BINARY), *args], env={**os.environ, "_SOPRA_SERVER_SOCKET": "", **env},
                            text=True, capture_output=True, timeout=15)
    assert (result.returncode == 0) == success, (args, result.stdout, result.stderr)
    return result.stdout + result.stderr


def submit(terminal: TerminalSession, command: str, directory: Path, success: bool = True) -> str:
    result = directory / "result"
    status = directory / "status"
    result.unlink(missing_ok=True)
    status.unlink(missing_ok=True)
    terminal.send_text(f"{command} > {shlex.quote(str(result))} 2>&1; print -r -- $? > {shlex.quote(str(status))}")
    terminal.key("ENTER")
    deadline = time.monotonic() + 15
    while not status.exists() and time.monotonic() < deadline:
        terminal.read(0.05)
    terminal.read(0.2)
    assert status.exists(), terminal.screen.text()
    output = result.read_text()
    assert (status.read_text().strip() == "0") == success, (command, output, terminal.screen.text())
    return output


def offline(directory: Path, env: dict[str, str]) -> None:
    assert "config.toml" in run_cli(["--help"], env)
    assert "restart" in run_cli([], env)
    assert run_cli(["config", "path"], env).strip() == str(directory / "config/sopra/config.toml")
    assert "valid" in run_cli(["config", "check"], env)
    assert "syntax_highlighting = true" in run_cli(["config", "show"], env)
    listing = run_cli(["widgets", "list"], env)
    assert "git\tbuilt-in\tavailable" in listing and "demo\t" in listing, listing
    assert "no Sopra session" in run_cli(["refresh"], env, False)
    assert "no Sopra session" in run_cli(["restart"], env, False)
    assert "unknown command" in run_cli(["refresh", "extra"], env, False)


def refresh(terminal: TerminalSession, directory: Path, env: dict[str, str]) -> str:
    config = directory / "config/sopra/config.toml"
    terminal.wait_for("first:", timeout=30)
    first = (directory / "widget.pid").read_text()
    config.write_text(configuration(directory, "UPDATED", "second"))
    assert "refreshed" in submit(terminal, "sopra refresh", directory)
    terminal.wait_for("second:", timeout=10)
    assert (directory / "widget.pid").read_text() == first
    assert "UPDATED" in submit(terminal, "sopra config show", directory)
    config.write_text("[prompt]\nleft='[red]broken[/blue]'\n")
    submit(terminal, "sopra refresh", directory, False)
    submit(terminal, "sopra restart", directory, False)
    assert "UPDATED" in submit(terminal, "sopra config show", directory)
    assert (directory / "widget.pid").read_text() == first
    assert "expected [/red]" in run_cli(["config", "check"], env, False)
    config.write_text("[prompt]\nleft='$missing_widget'\n")
    assert "unknown prompt widget" in run_cli(["config", "check"], env, False)
    submit(terminal, "sopra refresh", directory, False)
    config.write_text(configuration(directory, "UPDATED", "second"))
    return first


def reconciliation(terminal: TerminalSession, directory: Path, first: str) -> None:
    config = directory / "config/sopra/config.toml"
    (config.parent / "widgets/extra.rs").write_text(WIDGET)
    source = configuration(directory, "UPDATED", "second")
    config.write_text(source.replace("right='$demo'", "right='$demo / $extra'") +
                      f"[widgets.extra]\nlabel='extra'\npid_file='{directory / 'extra.pid'}'\n")
    submit(terminal, "sopra refresh", directory)
    terminal.wait_for("extra:", timeout=30)
    extra = int((directory / "extra.pid").read_text())
    assert (directory / "widget.pid").read_text() == first
    config.write_text(source)
    submit(terminal, "sopra refresh", directory)
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        try:
            os.kill(extra, 0)
        except ProcessLookupError:
            return
        terminal.read(0.05)
    raise AssertionError("removed widget process survived refresh")


def edit_and_restart(terminal: TerminalSession, directory: Path, first: str) -> None:
    editor = directory / "editor tool"
    editor.write_text("#!/bin/sh\n[ \"$1\" = '--wait' ] || exit 2\n"
                      "sed 's/UPDATED/EDITED/g' \"$2\" > \"$2.new\" && mv \"$2.new\" \"$2\"\n")
    editor.chmod(0o755)
    editor_command = shlex.quote(shlex.quote(str(editor)) + " --wait")
    assert "refreshed" in submit(terminal, f"VISUAL={editor_command} sopra config edit", directory)
    assert "EDITED" in submit(terminal, "sopra config show", directory)
    assert (directory / "widget.pid").read_text() == first
    assert "connected" in submit(terminal, "sopra doctor", directory)
    assert "connected" in submit(terminal, f"{shlex.quote(str(BINARY))} doctor", directory)
    assert "restarted" in submit(terminal, "sopra restart", directory)
    terminal.wait_for("second:", timeout=10)
    assert (directory / "widget.pid").read_text() != first
    assert "EDITED" in submit(terminal, "sopra config show", directory)
    assert "sopra restart" in submit(terminal, "fc -l -10", directory)


def busy_and_errors(terminal: TerminalSession, directory: Path, env: dict[str, str]) -> None:
    socket = submit(terminal, 'print -r -- "$_SOPRA_SERVER_SOCKET"', directory).strip()
    connected = {**env, "_SOPRA_SERVER_SOCKET": socket}
    terminal.send_text("echo unsubmitted-buffer")
    terminal.read(0.2)
    assert "worker is editing" in run_cli(["refresh"], connected, False)
    assert "worker is editing" in run_cli(["restart"], connected, False)
    assert "editing" in run_cli(["doctor"], connected)
    assert "unsubmitted-buffer" in terminal.screen.text(), terminal.screen.text()
    terminal.key("CTRL-U")
    widget = directory / "config/sopra/widgets/demo.rs"
    widget.write_text("invalid Rust source")
    terminal.wait_for("widget error", timeout=30)
    report = submit(terminal, "sopra widgets list", directory)
    assert "error:" in report and "failed to compile" in report, report
    widget.write_text(WIDGET)
    terminal.wait_for("second:", timeout=30)


def exercise(directory: Path) -> None:
    config = directory / "config/sopra"
    (config / "widgets").mkdir(parents=True)
    (config / "widgets/demo.rs").write_text(WIDGET)
    (config / "config.toml").write_text(configuration(directory))
    shell = directory / "shell"
    shell.mkdir()
    (shell / ".zshrc").write_text(f"source '{ROOT}/zsh/editor.zsh'\n")
    env = {"XDG_CONFIG_HOME": str(config.parent), "XDG_CACHE_HOME": str(directory / "cache"),
           "ZDOTDIR": str(shell), "SOPRA_BIN": str(BINARY)}
    offline(directory, env)
    with TerminalSession(("zsh", "-di"), rows=24, columns=140, cwd=str(directory), env=env) as terminal:
        first = refresh(terminal, directory, env)
        reconciliation(terminal, directory, first)
        edit_and_restart(terminal, directory, first)
        busy_and_errors(terminal, directory, env)


def main() -> None:
    subprocess.run(["cargo", "build", "--quiet", "--locked"], cwd=ROOT, check=True)
    with tempfile.TemporaryDirectory(prefix="sopra-cli-") as temporary:
        exercise(Path(temporary))
    print("CLI: ok (config, refresh rollback, retained widgets, restart, diagnostics, busy requests)")


if __name__ == "__main__":
    main()
