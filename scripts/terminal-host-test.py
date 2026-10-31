#!/usr/bin/env python3
"""Plain hosted terminal scrollback, draft retention, and rapid resize tests."""
from __future__ import annotations

from terminal_host import hosted, scroll_up, submit, wait_until


def test_scrollback() -> None:
    with hosted() as (terminal, root):
        submit(terminal, "for i in {1..60}; do print row-$i; done")
        terminal.send_text("echo unfinished draft")
        terminal.read(0.2)
        live = terminal.screen.text()
        assert "row-60" in live and "echo unfinished draft" in live, live
        scroll_up(terminal)
        earlier = terminal.screen.text()
        assert "row-36" in earlier, earlier
        assert "rprompt" not in earlier and "unfinished draft" not in earlier, earlier
        terminal.send(b"\x1b[<65;10;24M")
        terminal.read(0.1)
        assert terminal.screen.text() == live, terminal.screen.text()
        terminal.key("PAGEUP")
        terminal.read(0.1)
        assert "row-20" in terminal.screen.text(), terminal.screen.text()
        terminal.key("PAGEDOWN")
        terminal.read(0.1)
        assert terminal.screen.text() == live, terminal.screen.text()
        scroll_up(terminal)
        terminal.send_text(" preserved")
        terminal.read(0.1)
        assert "echo unfinished draft preserved" in terminal.screen.text()
        terminal.key("CTRL-U")
        submit(terminal, "touch submitted")
        wait_until(terminal, lambda: (root / "submitted").exists())
        submit(terminal, "exit 7")
        wait_until(terminal, lambda: not terminal.alive)
        assert terminal.returncode == 7, terminal.returncode
        assert b"\x1b[?1049l" in terminal.output


def test_live_mouse_selection() -> None:
    with hosted() as (terminal, _):
        terminal.send_text("alpha beta gamma")
        terminal.read(0.2)
        row = next(index for index, line in enumerate(terminal.screen.lines()) if line.startswith("$ alpha")) + 1
        terminal.send(f"\x1b[<0;9;{row}M\x1b[<32;13;{row}M\x1b[<0;13;{row}m")
        terminal.read(0.1)
        terminal.key("CTRL-C")
        terminal.read(0.1)
        assert b"\x1b]52;c;YmV0YQ==" in terminal.output
        terminal.send_text("replacement")
        terminal.read(0.1)
        assert "$ alpha replacement gamma" in terminal.screen.text(), terminal.screen.text()


def test_resize() -> None:
    with hosted() as (terminal, _):
        terminal.send_text("echo retained")
        terminal.read(0.1)
        for width in list(range(120, 39, -2)) + list(range(40, 121, 2)):
            terminal.resize(24, width)
            terminal.read(0.002)
        terminal.read(0.4)
        screen = terminal.screen.text()
        assert "echo retained" in screen, screen
        prompts = [line for line in terminal.screen.lines() if "host" in line]
        assert len(prompts) == 1, screen
        assert prompts[0].endswith("rprompt") and len(prompts[0]) == 120, screen


if __name__ == "__main__":
    test_scrollback()
    test_live_mouse_selection()
    test_resize()
    print("Plain hosted scrollback, draft retention, and rapid resize passed")
