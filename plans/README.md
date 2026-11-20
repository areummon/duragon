# Animation plans

Written by an animation audit at commit 550d3e3. The audit compared Duragon's motion against riddled.ai, measured live.

| # | Plan | Severity | Status |
| --- | --- | --- | --- |
| 001 | [Page-width drag feels direct](001-page-width-drag.md) | HIGH | DONE |
| 002 | [Keyboard surfaces open instantly](002-keyboard-surfaces-instant.md) | HIGH | DONE |
| 003 | [Chat open choreography](003-chat-open-choreography.md) | MEDIUM | DONE |
| 004 | [Popovers settle; hover parity](004-popover-settle-and-hover.md) | MEDIUM | DONE |
| 005 | [Mind picker groups and footer](005-mind-picker-groups.md) | LOW | DONE |

## Order and dependencies

Run them in numeric order.

- 001 and 003 both edit `Page.svelte`.
- 001, 004 and 005 all edit `NoteHeaderRow.svelte`.

Running them in order on one branch avoids conflicts. 002 is independent.

## Considered and left alone

- **`ease-in` in `InkDrops.svelte` and `Blot.svelte`**: this is the ink-drying fade, a physical, decorative curve and not UI response.
- **Sidebar drawer**: it already uses Riddled's measured 380 ms drawer curve.
- **The reduced-motion rule in `motion.css`**: it zeroes every transition, including opacity. Gentler-not-zero would be better, but the change touches every component. Revisit separately.
