<img src="packaging/badge.svg" width="96" height="96" alt="Combe">

# Combe

A worktree-aware terminal for Macs, written for personal use.

[![Watch the introduction](https://img.youtube.com/vi/RaGCmdP5sD4/maxresdefault.jpg)](https://www.youtube.com/watch?v=RaGCmdP5sD4)

## Install

> Apple Silicon only, macOS 13 Ventura or later.

```sh
brew install samzong/tap/combe
```

## Shortcuts

### Tabs and panes

| Key | Action |
| --- | --- |
| Cmd-T | New tab |
| Cmd-D | Split right |
| Cmd-Shift-D | Split down |
| Cmd-Shift-T | Move pane to a new tab |
| Cmd-Shift-Return | Zoom or restore pane |
| Cmd-Shift-Backslash | Tab overview |

### Moving around

| Key | Action |
| --- | --- |
| Cmd-Shift-[ / ] | Previous / next tab |
| Cmd-[ / ] | Previous / next pane |
| Cmd-Alt-Arrows | Focus neighbouring pane |
| Cmd-B | Pin or unpin sidebar |
| Cmd-1…9 | Sidebar open: workspace N. Closed: tab N (9 = last) |

### Text

| Key | Action |
| --- | --- |
| Cmd-F | Find. Return / Shift-Return cycles; Esc closes |
| Cmd-G / Cmd-Shift-G | Next / previous match |
| Cmd-C / Cmd-V | Copy / paste. Selection copies on release |

### Window

| Key | Action |
| --- | --- |
| Ctrl-Cmd-F | Full screen |
| Cmd-M | Minimize |
| Cmd-Q | Quit. Confirms if a process is running |

## Acknowledgments

[Orca](https://github.com/stablyai/orca) showed that a worktree-aware terminal can work.

Every terminal leaf is a [Ghostty](https://github.com/ghostty-org/ghostty) `libghostty` surface.

## Licenses

MIT.
