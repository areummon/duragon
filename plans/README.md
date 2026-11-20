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
- **The reduced-motion rule in `motion.css`**: it zeroed every transition, including opacity. Plan 006 addresses it.

# Second round

Written at commit 10c3e97, from an opportunity sweep and an accessibility audit.

| # | Plan | Severity | Status |
| --- | --- | --- | --- |
| 006 | [Reduced motion keeps fades, drops movement](006-reduced-motion-keeps-fades.md) | MEDIUM | DONE |
| 007 | [The chat's Copy button confirms](007-chat-copy-confirms.md) | MEDIUM | DONE (2fc4659) |
| 008 | [Thread and drawing dialogs open like the others](008-dialogs-pop-like-customize.md) | LOW | DONE (f72b87b) |
| 009 | [Your sent chat message settles in](009-sent-message-inks-in.md) | LOW | DONE (6a532bf) |
| 010 | [Icon buttons answer a press](010-icon-buttons-press.md) | LOW | DONE (bd7e364) |
| 011 | [Sidebar menus fade between their views](011-menu-views-fade.md) | LOW | DONE (0402945) |

## Order and dependencies

Run 006 first. 008 and 009 rely on it to turn `pop` and `ink-in` into plain fades under reduced motion. 010 adds its own reduced-motion lines, into the block 006 creates when it exists.

- 006, 007, 009 and 010 all edit `ChatPanel.svelte`.
- 006 and 010 both edit `TopBar.svelte`.
- 006 and 011 both edit `Sidebar.svelte`.

Running them in numeric order on one branch avoids conflicts.
