const { Terminal } = require('@xterm/headless');
const readline = require('node:readline');

const terminal = new Terminal({
  cols: 120,
  rows: 12,
  scrollback: 2000,
  allowProposedApi: true,
});
let responses = [];
terminal.onData(data => responses.push(Buffer.from(data).toString('base64')));

async function handle(request) {
  if (request.op === 'feed') {
    await new Promise(resolve => terminal.write(Buffer.from(request.data, 'base64'), resolve));
  } else if (request.op === 'resize') {
    terminal.resize(request.columns, request.rows);
  }
  const screen = terminal.buffer.active;
  const lines = [];
  const paintedCursors = [];
  if (request.op === 'snapshot') {
    for (let row = 0; row < screen.length; row++) {
      const line = screen.getLine(row);
      lines.push(line.translateToString(true));
      for (let column = 0; column < terminal.cols; column++) {
        const cell = line.getCell(column);
        if (cell.isBgRGB() || (cell.isInverse() && cell.isBold())) paintedCursors.push([column, row]);
      }
    }
  }
  const result = {
    responses,
    lines,
    base: screen.baseY,
    cursor: [screen.cursorX, screen.cursorY],
    paintedCursors,
  };
  responses = [];
  process.stdout.write(`${JSON.stringify(result)}\n`);
}

const input = readline.createInterface({ input: process.stdin });
let pending = Promise.resolve();
input.on('line', line => {
  pending = pending.then(() => handle(JSON.parse(line))).catch(error => {
    process.stderr.write(`${error.stack}\n`);
    process.exitCode = 1;
    input.close();
  });
});
