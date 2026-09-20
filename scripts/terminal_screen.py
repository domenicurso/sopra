"""Character-grid state for the separate interactive terminal session tool."""
from __future__ import annotations

import unicodedata
from dataclasses import dataclass

try:
    from .terminal_screen_ansi import TerminalParser
except ImportError:
    from terminal_screen_ansi import TerminalParser


def _width(character: str) -> int:
    if unicodedata.combining(character) or unicodedata.category(character)[0] == "C":
        return 0
    return 2 if unicodedata.east_asian_width(character) in "WF" else 1


@dataclass
class _State:
    cells: list[list[str]]
    row: int
    column: int
    scroll_top: int
    scroll_bottom: int
    wrap_pending: bool


class VirtualTerminal:
    """Render common ANSI output into a readable, cursor-aware character grid."""

    def __init__(self, rows: int, columns: int) -> None:
        self.rows = max(1, rows)
        self.columns = max(1, columns)
        self._cells = self._blank()
        self._saved_main: _State | None = None
        self.row = self.column = 0
        self.scroll_top, self.scroll_bottom = 0, self.rows - 1
        self.cursor_visible = True
        self.wrap_pending = False
        self._saved_cursor = (0, 0)
        self._parser = TerminalParser(self)

    def _blank(self) -> list[list[str]]:
        return [[" "] * self.columns for _ in range(self.rows)]

    def feed(self, data: bytes) -> tuple[bytes, ...]:
        return self._parser.feed(data)

    def resize(self, rows: int, columns: int) -> None:
        rows, columns = max(1, rows), max(1, columns)
        self._cells = self._fit(self._cells, rows, columns)
        if self._saved_main is not None:
            self._saved_main.cells = self._fit(self._saved_main.cells, rows, columns)
            self._saved_main.row = min(self._saved_main.row, rows - 1)
            self._saved_main.column = min(self._saved_main.column, columns - 1)
            self._saved_main.scroll_top = 0
            self._saved_main.scroll_bottom = rows - 1
        self.rows, self.columns = rows, columns
        self.row = min(self.row, rows - 1)
        self.column = min(self.column, columns - 1)
        self.scroll_top, self.scroll_bottom = 0, rows - 1
        self.wrap_pending = False

    @staticmethod
    def _fit(old: list[list[str]], rows: int, columns: int) -> list[list[str]]:
        cells = [[" "] * columns for _ in range(rows)]
        for row in range(min(rows, len(old))):
            cells[row][: min(columns, len(old[row]))] = old[row][:columns]
        return cells

    def lines(self, trim: bool = True) -> list[str]:
        lines = ["".join(row) for row in self._cells]
        return [line.rstrip() for line in lines] if trim else lines

    def text(self, trim: bool = True) -> str:
        return "\n".join(self.lines(trim))

    def _print(self, character: str) -> None:
        width = _width(character)
        if width == 0:
            if self.column and self._cells[self.row][self.column - 1].strip():
                self._cells[self.row][self.column - 1] += character
            return
        if self.wrap_pending:
            self.column = 0
            self._linefeed()
        if width == 2 and self.column == self.columns - 1:
            self.column = 0
            self._linefeed()
        self._cells[self.row][self.column] = character
        if width == 2 and self.column + 1 < self.columns:
            self._cells[self.row][self.column + 1] = ""
        if self.column + width >= self.columns:
            self.column, self.wrap_pending = self.columns - 1, True
        else:
            self.column += width

    def _linefeed(self) -> None:
        self.wrap_pending = False
        if self.row == self.scroll_bottom:
            del self._cells[self.scroll_top]
            self._cells.insert(self.scroll_bottom, [" "] * self.columns)
        else:
            self.row = min(self.rows - 1, self.row + 1)

    def _reverse_index(self) -> None:
        if self.row == self.scroll_top:
            self._cells.insert(self.scroll_top, [" "] * self.columns)
            del self._cells[self.scroll_bottom + 1]
        else:
            self.row -= 1

    def _erase_display(self, mode: int) -> None:
        if mode in (2, 3):
            self._cells = self._blank()
        elif mode == 1:
            for row in self._cells[: self.row]:
                row[:] = [" "] * self.columns
            self._cells[self.row][: self.column + 1] = [" "] * (self.column + 1)
        else:
            self._cells[self.row][self.column :] = [" "] * (self.columns - self.column)
            for row in self._cells[self.row + 1 :]:
                row[:] = [" "] * self.columns

    def _erase_line(self, mode: int) -> None:
        if mode == 1:
            start, end = 0, self.column + 1
        elif mode == 2:
            start, end = 0, self.columns
        else:
            start, end = self.column, self.columns
        self._cells[self.row][start:end] = [" "] * (end - start)

    def _scroll(self, count: int, up: bool) -> None:
        for _ in range(min(count, self.scroll_bottom - self.scroll_top + 1)):
            if up:
                del self._cells[self.scroll_top]
                self._cells.insert(self.scroll_bottom, [" "] * self.columns)
            else:
                del self._cells[self.scroll_bottom]
                self._cells.insert(self.scroll_top, [" "] * self.columns)

    def _edit_cells(self, count: int, insert: bool) -> None:
        count = min(count, self.columns - self.column)
        row = self._cells[self.row]
        if insert:
            row[self.column :] = ([" "] * count + row[self.column :])[: self.columns - self.column]
        else:
            row[self.column :] = (row[self.column + count :] + [" "] * count)[: self.columns - self.column]

    def _edit_lines(self, count: int, insert: bool) -> None:
        count = min(count, self.scroll_bottom - self.row + 1)
        for _ in range(count):
            if insert:
                self._cells.insert(self.row, [" "] * self.columns)
                del self._cells[self.scroll_bottom + 1]
            else:
                del self._cells[self.row]
                self._cells.insert(self.scroll_bottom, [" "] * self.columns)

    def _set_alternate(self, enabled: bool) -> None:
        if enabled and self._saved_main is None:
            self._saved_main = _State(self._cells, self.row, self.column, self.scroll_top, self.scroll_bottom, self.wrap_pending)
            self._cells = self._blank()
            self.row = self.column = self.scroll_top = 0
            self.scroll_bottom, self.wrap_pending = self.rows - 1, False
        elif not enabled and self._saved_main is not None:
            state, self._saved_main = self._saved_main, None
            self._cells = state.cells
            self.row, self.column = state.row, state.column
            self.scroll_top, self.scroll_bottom = state.scroll_top, state.scroll_bottom
            self.wrap_pending = state.wrap_pending
            self.row = min(self.row, self.rows - 1)
            self.column = min(self.column, self.columns - 1)

    def reset(self) -> None:
        self._set_alternate(False)
        self._cells = self._blank()
        self.row = self.column = self.scroll_top = 0
        self.scroll_bottom, self.wrap_pending = self.rows - 1, False
