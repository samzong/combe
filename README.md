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
| Cmd-Shift-T | Move the focused pane into its own tab |
| Cmd-Shift-Return | Zoom the focused pane, or restore its layout |
| Cmd-Shift-Backslash | Toggle the tab overview |

### Moving around

| Key | Action |
| --- | --- |
| Cmd-Shift-[ / ] | Previous / next tab |
| Cmd-[ / ] | Previous / next pane |
| Cmd-Alt-Arrows | Focus the neighbouring pane in that direction |
| Cmd-B | Pin the sidebar, or return it to the workspace chip |
| Cmd-1 … Cmd-9 | With the sidebar open, select visible workspace N. With it closed, go to tab N, where Cmd-9 is the last tab |

### Text

| Key | Action |
| --- | --- |
| Cmd-F | Find in the focused pane. Return / Shift-Return step through matches, Escape closes |
| Cmd-G / Cmd-Shift-G | Next / previous match |
| Cmd-C / Cmd-V | Copy / paste. Releasing a selection also copies it |

### Window

| Key | Action |
| --- | --- |
| Ctrl-Cmd-F | Toggle full screen |
| Cmd-M | Minimize |
| Cmd-Q | Quit. Asks first when a terminal still runs a foreground process |

## Acknowledgments

[Orca](https://github.com/stablyai/orca) showed that a worktree-aware terminal can work.

Every terminal leaf is a [Ghostty](https://github.com/ghostty-org/ghostty) `libghostty` surface.

## Licenses

MIT.
