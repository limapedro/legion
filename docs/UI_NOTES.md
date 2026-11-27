# UI notes — JARVIS HUD in `cmd`

Research index for the interface track. Each experiment lives in
`experiments/exp_NNN/` with a `HYPOTHESIS.md` written before and `FINDINGS.md`
written after, negative results included. Keep/drop decisions are logged in
[`ARCHITECTURE_LOG.md`](../ARCHITECTURE_LOG.md).

## Planned experiments

| Track | Question | Folder |
| --- | --- | --- |
| Layout | Which pane arrangement works at small/medium/wide widths? | TBD |
| Aesthetic | Progress rings, palette, boot sequence, spinners, streaming text | TBD |
| Swarm visualization | Live agent states, token flow, ownership graph at 8+ agents | TBD |
| Voice waveform | Mock waveform/orb widget for future voice mode | TBD |
| Performance | Frame time, idle CPU, redraw cost | TBD |
| Fallbacks | No-color, no-Unicode, reduced-motion | TBD |

## Constraints already known

- Must run in classic `cmd.exe` and Windows Terminal; detect the host and
  degrade gracefully when a feature is unsupported.
- Relevant Windows console features: `ENABLE_VIRTUAL_TERMINAL_PROCESSING`,
  alternate screen buffer, truecolor vs 256 colors, Unicode/box drawing,
  braille and half-block sub-cell graphics, cursor control, mouse input,
  resize events, synchronized output, keyboard handling.
- Target: low CPU, low memory, no GPU reliance; the UI must never make the
  machine feel busy.

Deeper notes will be added from the study sub-agent report in `docs/study/`.
