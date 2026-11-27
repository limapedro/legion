"""Python equivalent of the Rust cell-grid diff benchmark + startup harness.

Usage:
  python bench.py startup <runs> -- <command> [args...]
  python bench.py bench <frames> <changes> <write:0|1>
  python bench.py idle <seconds>
"""
import statistics
import subprocess
import sys
import time

W, H = 120, 50
N = W * H


def startup(runs: int, command: list[str]) -> None:
    samples = []
    for _ in range(runs):
        t0 = time.perf_counter()
        subprocess.run(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        samples.append((time.perf_counter() - t0) * 1000.0)
    print(
        f"startup runs={runs} median_ms={statistics.median(samples):.3f} "
        f"min_ms={min(samples):.3f} max_ms={max(samples):.3f}"
    )


def encode(front: list[int], back: list[int]) -> str:
    out = ["\x1b[?2026h"]
    last_fg = last_bg = 255
    for y in range(H):
        in_run = False
        for x in range(W):
            i = y * W + x
            if front[i] == back[i]:
                in_run = False
                continue
            if not in_run:
                out.append(f"\x1b[{y + 1};{x + 1}H")
                in_run = True
                last_fg = last_bg = 255
            cell = back[i]
            glyph = cell & 0xFFFF
            fg = (cell >> 16) & 0xFF
            if fg != last_fg:
                out.append(f"\x1b[38;5;{fg}m")
                last_fg = fg
            out.append(chr(glyph))
    out.append("\x1b[0m\x1b[?2026l")
    return "".join(out)


def bench(frames: int, changes: int, write: bool) -> None:
    front = [32] * N
    back = [32] * N
    state = 0x9E3779B97F4A7C15
    total_ns = 0
    total_bytes = 0
    for f in range(frames):
        for k in range(changes):
            state ^= (state << 13) & 0xFFFFFFFFFFFFFFFF
            state ^= state >> 7
            state ^= (state << 17) & 0xFFFFFFFFFFFFFFFF
            i = state % N
            back[i] = (ord("A") + (f + k) % 26) | (((state >> 11) % 16) << 16)
        t0 = time.perf_counter_ns()
        s = encode(front, back)
        if write:
            sys.stdout.buffer.write(s.encode("utf-8"))
        total_ns += time.perf_counter_ns() - t0
        total_bytes += len(s)
        front[:] = back
    print(
        f"frames={frames} changes/frame={changes} write={int(write)} "
        f"mean_ms={total_ns / frames / 1e6:.4f} total_bytes={total_bytes} "
        f"bytes/frame={total_bytes / frames:.0f}",
        file=sys.stderr,
    )


def main() -> None:
    mode = sys.argv[1]
    if mode == "startup":
        runs = int(sys.argv[2])
        sep = sys.argv.index("--")
        startup(runs, sys.argv[sep + 1 :])
    elif mode == "bench":
        bench(int(sys.argv[2]), int(sys.argv[3]), sys.argv[4] == "1")
    elif mode == "idle":
        time.sleep(float(sys.argv[2]))
    else:
        raise SystemExit(f"unknown mode: {mode}")


if __name__ == "__main__":
    main()
