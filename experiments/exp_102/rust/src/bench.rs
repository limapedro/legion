//! Cell-grid diff + escape-encode benchmark for a 120x50 HUD.
//! Usage: bench <frames> <changed_cells_per_frame> <write:0|1>
//! Results go to stdout; timing summary goes to stderr.
use std::fmt::Write as _;
use std::io::Write as _;
use std::time::Instant;

const W: usize = 120;
const H: usize = 50;
const N: usize = W * H;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Cell {
    glyph: u32,
    fg: u8,
    bg: u8,
    flags: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Self { glyph: ' ' as u32, fg: 0, bg: 0, flags: 0 }
    }
}

fn encode(front: &[Cell], back: &[Cell], out: &mut String) {
    out.clear();
    out.push_str("\x1b[?2026h");
    let (mut last_fg, mut last_bg, mut last_flags) = (255u8, 255u8, 255u8);
    for y in 0..H {
        let mut in_run = false;
        for x in 0..W {
            let i = y * W + x;
            if front[i] == back[i] {
                if in_run {
                    in_run = false;
                }
                continue;
            }
            if !in_run {
                let _ = write!(out, "\x1b[{};{}H", y + 1, x + 1);
                in_run = true;
                // force style to be emitted for the first cell of a run
                last_fg = 255;
                last_bg = 255;
                last_flags = 255;
            }
            let c = back[i];
            if c.fg != last_fg {
                let _ = write!(out, "\x1b[38;5;{}m", c.fg);
                last_fg = c.fg;
            }
            if c.bg != last_bg {
                let _ = write!(out, "\x1b[48;5;{}m", c.bg);
                last_bg = c.bg;
            }
            if c.flags != last_flags {
                let _ = write!(out, "\x1b[{}m", if c.flags == 1 { 1 } else { 0 });
                last_flags = c.flags;
            }
            out.push(char::from_u32(c.glyph).unwrap_or('?'));
        }
    }
    out.push_str("\x1b[0m\x1b[?2026l");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let frames: usize = args.get(0).and_then(|s| s.parse().ok()).unwrap_or(600);
    let changes: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(3);
    let write: bool = args.get(2).map(|s| s == "1").unwrap_or(false);

    let mut front = vec![Cell::default(); N];
    let mut back = vec![Cell::default(); N];
    let mut out = String::with_capacity(N * 24);
    let mut state: u64 = 0x9e3779b97f4a7c15;
    let mut total_ns: u128 = 0;
    let mut total_bytes = 0usize;
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();

    for f in 0..frames {
        for k in 0..changes {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let i = (state as usize) % N;
            back[i].glyph = 'A' as u32 + ((f + k) % 26) as u32;
            back[i].fg = ((state >> 11) % 16) as u8;
        }
        let t0 = Instant::now();
        encode(&front, &back, &mut out);
        if write {
            let _ = lock.write_all(out.as_bytes());
        }
        total_ns += t0.elapsed().as_nanos();
        total_bytes += out.len();
        // Promote back to front so the next diff contains only this frame's changes.
        front.copy_from_slice(&back);
    }
    let mean_ms = total_ns as f64 / frames as f64 / 1_000_000.0;
    eprintln!(
        "frames={} changes/frame={} write={} mean_ms={:.4} total_bytes={} bytes/frame={:.0}",
        frames,
        changes,
        write,
        mean_ms,
        total_bytes,
        total_bytes as f64 / frames as f64
    );
}
