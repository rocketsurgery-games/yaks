#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = ["pyte"]
# ///
"""Capture the Python (curses) yaks TUI as plain-text grids, for differential
testing against the Rust `yaks tui --headless` harness.

The curses TUI has no pure render() to snapshot, so we run it under a real
pseudo-terminal, scrape the emitted VT stream with pyte (a headless VT100
emulator), and dump the resulting character grid. The stdin protocol mirrors
the Rust harness so one script drives both:

    key <name>     one key: a char, or Enter/Esc/Tab/BackTab/Space/Backspace/
                   Up/Down/Left/Right/Home/End/PageUp/PageDown/Delete; C- = Ctrl
    type <text>    type the rest of the line verbatim
    snapshot       re-emit the current screen
    resize <w> <h> resize the pty (SIGWINCH)
    quit           send 'q' and exit

Each action emits a framed grid identical in shape to the Rust harness (minus
the state header, which is internal to the Rust App and not observable here):

    === frame N · WxH · (python) ===
    <rows...>
    === end ===

Usage:
    printf 'key j\\nquit\\n' | uv run tools/py_tui_capture.py \\
        --yak /path/to/yaks/scripts/yak.py --herd /path/to/herd --size 72x12
"""

from __future__ import annotations

import argparse
import fcntl
import os
import select
import signal
import struct
import subprocess
import sys
import termios
import time

import pyte

# name -> bytes sent to the curses app (xterm-ish encodings).
_KEYS = {
    "Enter": b"\r",
    "Esc": b"\x1b",
    "Tab": b"\t",
    "BackTab": b"\x1b[Z",
    "Space": b" ",
    "Backspace": b"\x7f",
    "Up": b"\x1b[A",
    "Down": b"\x1b[B",
    "Right": b"\x1b[C",
    "Left": b"\x1b[D",
    "Home": b"\x1b[H",
    "End": b"\x1b[F",
    "PageUp": b"\x1b[5~",
    "PageDown": b"\x1b[6~",
    "Delete": b"\x1b[3~",
}


def base36(i: int) -> str:
    if i < 10:
        return chr(ord("0") + i)
    if i < 36:
        return chr(ord("a") + i - 10)
    return "#"


def _describe(key: tuple) -> str:
    fg, bg, bold, reverse, underscore, italics = key
    if fg == "default" and bg == "default" and not (bold or reverse or underscore or italics):
        return "default"
    parts = []
    if fg != "default":
        parts.append(f"fg={fg}")
    if bg != "default":
        parts.append(f"bg={bg}")
    if bold:
        parts.append("bold")
    if reverse:
        parts.append("reversed")
    if underscore:
        parts.append("underline")
    if italics:
        parts.append("italic")
    return "+".join(parts)


def key_bytes(spec: str) -> bytes:
    ctrl = spec.startswith("C-")
    name = spec[2:] if ctrl else spec
    if name in _KEYS:
        raw = _KEYS[name]
    elif len(name) == 1:
        raw = name.encode("utf-8")
    else:
        return b""
    if ctrl and len(raw) == 1:
        # Control-map a single letter (C-c -> 0x03).
        return bytes([raw[0] & 0x1F])
    return raw


class PyTui:
    def __init__(self, yak: str, herd: str, cols: int, rows: int, launch: list[str], style: bool):
        self.cols, self.rows = cols, rows
        self.style = style
        self.screen = pyte.Screen(cols, rows)
        self.stream = pyte.ByteStream(self.screen)
        self.master, slave = os.openpty()
        self._set_winsize(slave)
        env = dict(os.environ, TERM="xterm-256color", LINES=str(rows), COLUMNS=str(cols))
        self.errfile = open(os.path.join(os.getenv("TMPDIR", "/tmp"), f"pytui-err-{os.getpid()}.log"), "w+")
        self.dead = False
        self.proc = subprocess.Popen(
            [*launch, yak, "tui"],
            stdin=slave,
            stdout=slave,
            stderr=self.errfile,
            cwd=herd,
            env=env,
            close_fds=True,
            start_new_session=True,
        )
        os.close(slave)

    def _set_winsize(self, fd: int) -> None:
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", self.rows, self.cols, 0, 0))

    def drain(self, idle: float = 0.2, maximum: float = 5.0) -> None:
        """Feed pty output to pyte until the app goes quiet for `idle` seconds."""
        deadline = time.time() + maximum
        while time.time() < deadline:
            r, _, _ = select.select([self.master], [], [], idle)
            if not r:
                return
            try:
                data = os.read(self.master, 65536)
            except OSError:
                return
            if not data:
                return
            self.stream.feed(data)

    def send(self, data: bytes) -> None:
        if data and not self.dead:
            try:
                os.write(self.master, data)
            except OSError:
                self.dead = True

    def resize(self, cols: int, rows: int) -> None:
        self.cols, self.rows = cols, rows
        self.screen.resize(rows, cols)
        self._set_winsize(self.master)
        try:
            os.killpg(os.getpgid(self.proc.pid), signal.SIGWINCH)
        except (ProcessLookupError, PermissionError):
            pass

    def wait_ready(self, maximum: float = 15.0) -> None:
        """Drain until the app paints something (non-blank screen) or timeout."""
        deadline = time.time() + maximum
        while time.time() < deadline:
            self.drain(idle=0.3, maximum=1.0)
            if any(row.strip() for row in self.screen.display):
                # One more short drain to let the first full frame settle.
                self.drain(idle=0.3, maximum=1.5)
                return
            if self.proc.poll() is not None:
                return

    def _row_width(self, y: int) -> int:
        row = self.screen.buffer[y]
        w = 0
        for x in range(self.cols):
            if row[x].data != " ":
                w = x + 1
        return w

    def _style_layer(self) -> tuple[list[str], list[str]]:
        """Aligned base36 style grid + legend, mirroring the Rust --style output.
        Selection/focus/links are attribute-encoded (reverse video etc.), so this
        is what makes them visible in the diff."""
        keys: list[tuple] = []
        rows_out = []
        for y in range(self.rows):
            row = self.screen.buffer[y]
            w = self._row_width(y)
            line = ""
            for x in range(w):
                c = row[x]
                key = (c.fg, c.bg, c.bold, c.reverse, c.underscore, c.italics)
                if key in keys:
                    idx = keys.index(key)
                else:
                    idx = len(keys)
                    keys.append(key)
                line += base36(idx)
            rows_out.append(line)
        legend = [f"{base36(i)}={_describe(k)}" for i, k in enumerate(keys)]
        return rows_out, legend

    def dump(self, n: int) -> str:
        out = [f"=== frame {n} · {self.cols}x{self.rows} · (python) ==="]
        for line in self.screen.display:
            out.append(line.rstrip())
        if self.style:
            rows_out, legend = self._style_layer()
            out.append("--- styles ---")
            out.extend(rows_out)
            out.append("legend: " + "  ".join(legend))
        out.append("=== end ===")
        return "\n".join(out)

    def close(self) -> None:
        try:
            self.send(b"q")
            time.sleep(0.15)
        except OSError:
            pass
        try:
            self.proc.terminate()
            self.proc.wait(timeout=2)
        except Exception:
            try:
                self.proc.kill()
            except Exception:
                pass
        try:
            os.close(self.master)
        except OSError:
            pass


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--yak", required=True, help="path to the Python yaks scripts/yak.py")
    ap.add_argument("--herd", default=".", help="working dir containing .yaks/ (default: cwd)")
    ap.add_argument("--size", default="80x24", help="WxH terminal size")
    ap.add_argument("--style", action="store_true",
                    help="also emit an aligned base36 style grid + legend")
    ap.add_argument("--launch", default="uv run",
                    help="command prefix to run the Python TUI (default: 'uv run', "
                         "which resolves yak.py's pyyaml via its PEP 723 header)")
    args = ap.parse_args()

    cols, rows = (int(v) for v in args.size.lower().split("x"))
    tui = PyTui(args.yak, os.path.abspath(args.herd), cols, rows, args.launch.split(), args.style)
    n = 0
    frames: list[str] = []
    try:
        tui.wait_ready()  # wait for the first curses paint
        frames.append(tui.dump(n))
        n += 1
        for line in sys.stdin:
            line = line.rstrip("\r\n")
            if not line:
                continue
            if line == "quit":
                break
            if line == "snapshot":
                tui.drain()
                frames.append(tui.dump(n))
                n += 1
                continue
            if line.startswith("resize "):
                _, w, h = line.split()
                tui.resize(int(w), int(h))
                tui.drain()
                frames.append(tui.dump(n))
                n += 1
                continue
            if line.startswith("type "):
                for ch in line[5:]:
                    tui.send(ch.encode("utf-8"))
                tui.drain()
                frames.append(tui.dump(n))
                n += 1
                continue
            if line.startswith("key "):
                tui.send(key_bytes(line[4:].strip()))
                tui.drain()
                frames.append(tui.dump(n))
                n += 1
                continue
            print(f"! unknown action: {line}", file=sys.stderr)
            if tui.dead:
                break
    finally:
        tui.close()

    sys.stdout.write("\n".join(frames) + "\n")
    tui.errfile.seek(0)
    err = tui.errfile.read().strip()
    if err:
        sys.stderr.write("\n[child stderr]\n" + err + "\n")


if __name__ == "__main__":
    main()
