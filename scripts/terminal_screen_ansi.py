"""ANSI/VT parser that drives the character grid without owning a PTY."""
from __future__ import annotations

import codecs
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .terminal_screen import VirtualTerminal


class TerminalParser:
    def __init__(self, screen: "VirtualTerminal") -> None:
        self.screen = screen
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")
        self.state = "text"
        self.sequence = ""
        self.osc = ""
        self.responses: list[bytes] = []

    def feed(self, data: bytes) -> tuple[bytes, ...]:
        self.responses = []
        for character in self.decoder.decode(data):
            self._consume(character)
        return tuple(self.responses)

    def _consume(self, character: str) -> None:
        handlers = {
            "text": self._text,
            "escape": self._escape,
            "csi": self._csi,
            "osc": self._osc_text,
            "osc_escape": self._osc_escape,
        }
        handlers[self.state](character)

    def _text(self, character: str) -> None:
        if character in "\x1b\x9b\x9d":
            self.state = {"\x1b": "escape", "\x9b": "csi", "\x9d": "osc"}[character]
            self.sequence = ""
        elif character == "\n":
            self.screen._linefeed()
        elif character == "\r":
            self.screen.column, self.screen.wrap_pending = 0, False
        elif character == "\b":
            self.screen.column = max(0, self.screen.column - 1)
            self.screen.wrap_pending = False
        elif character == "\t":
            self.screen.column = min(self.screen.columns - 1, ((self.screen.column // 8) + 1) * 8)
            self.screen.wrap_pending = False
        elif ord(character) >= 0x20 and character != "\x7f":
            self.screen._print(character)

    def _escape(self, character: str) -> None:
        self.state = "text"
        if character == "[":
            self.state = "csi"
        elif character == "]":
            self.state, self.osc = "osc", ""
        elif character == "7":
            self.screen._saved_cursor = (self.screen.row, self.screen.column)
        elif character == "8":
            self.screen.row, self.screen.column = self.screen._saved_cursor
        elif character == "M":
            self.screen._reverse_index()
        elif character == "D":
            self.screen._linefeed()
        elif character == "E":
            self.screen.column = 0
            self.screen._linefeed()
        elif character == "c":
            self.screen.reset()

    def _csi(self, character: str) -> None:
        if "@" <= character <= "~":
            self._handle_csi(self.sequence, character)
            self.sequence, self.state = "", "text"
        else:
            self.sequence += character

    def _osc_text(self, character: str) -> None:
        if character == "\x07":
            self._handle_osc(self.osc)
            self.state = "text"
        elif character == "\x1b":
            self.state = "osc_escape"
        else:
            self.osc += character

    def _osc_escape(self, character: str) -> None:
        if character == "\\":
            self._handle_osc(self.osc)
            self.state = "text"
        else:
            self.osc += "\x1b" + character
            self.state = "osc"

    def _handle_osc(self, value: str) -> None:
        responses = {
            "10;?": b"\x1b]10;rgb:ffff/ffff/ffff\x1b\\",
            "11;?": b"\x1b]11;rgb:0000/0000/0000\x1b\\",
            "12;?": b"\x1b]12;#00ff00\x07",
        }
        if value in responses:
            self.responses.append(responses[value])

    def _handle_csi(self, raw: str, final: str) -> None:
        private = raw[:1] if raw[:1] in "?>!" else ""
        parameters = raw[1:] if private else raw
        values = [self._number(part) for part in parameters.replace(":", ";").split(";")] if parameters else []
        value = self._param(values, 0, 1)
        s = self.screen
        if final in "ABCDEFGHf`drsu":
            s.wrap_pending = False
        if final == "A":
            s.row = max(s.scroll_top, s.row - value)
        elif final == "B":
            s.row = min(s.scroll_bottom, s.row + value)
        elif final == "C":
            s.column = min(s.columns - 1, s.column + value)
        elif final == "D":
            s.column = max(0, s.column - value)
        elif final in "EF":
            s.row = min(s.scroll_bottom, s.row + value) if final == "E" else max(s.scroll_top, s.row - value)
            s.column = 0
        elif final in "G`":
            s.column = min(s.columns - 1, max(0, value - 1))
        elif final == "d":
            s.row = min(s.rows - 1, max(0, value - 1))
        elif final in "Hf":
            s.row = min(s.rows - 1, max(0, value - 1))
            s.column = min(s.columns - 1, max(0, self._param(values, 1, 1) - 1))
        elif final == "J":
            s._erase_display(self._param(values, 0, 0))
        elif final == "K":
            s._erase_line(self._param(values, 0, 0))
        elif final == "r":
            s.scroll_top = max(0, self._param(values, 0, 1) - 1)
            s.scroll_bottom = min(s.rows - 1, self._param(values, 1, s.rows) - 1)
            s.row, s.column = 0, 0
        elif final in "su":
            if final == "s":
                s._saved_cursor = (s.row, s.column)
            else:
                s.row, s.column = s._saved_cursor
        elif final in "ST":
            s._scroll(value, final == "S")
        elif final in "@P":
            s._edit_cells(value, final == "@")
        elif final in "LM":
            s._edit_lines(value, final == "L")
        elif final in "hl" and private:
            self._set_mode(parameters, final == "h")
        elif final == "n" and value == 6:
            self.responses.append(f"\x1b[{s.row + 1};{s.column + 1}R".encode())
        elif final == "c":
            self.responses.append(b"\x1b[?62;c")

    @staticmethod
    def _number(value: str) -> int | None:
        try:
            return int(value) if value else None
        except ValueError:
            return None

    @staticmethod
    def _param(values: list[int | None], index: int, default: int) -> int:
        if index >= len(values) or values[index] is None:
            return default
        return values[index]

    def _set_mode(self, parameters: str, enabled: bool) -> None:
        modes = {int(part) for part in parameters.split(";") if part.isdigit()}
        if modes & {47, 1047, 1049}:
            self.screen._set_alternate(enabled)
        if 25 in modes:
            self.screen.cursor_visible = enabled
