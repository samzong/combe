<img src="packaging/badge.svg" width="96" height="96" alt="Combe">

# Combe

A worktree-aware terminal for Apple Silicon. Catalog on the left, Ghostty surfaces on the right.

## Install

```sh
brew install samzong/tap/combe
```

Apple Silicon, macOS 13+. Not an agent IDE, not Intel, not iTerm. Lists worktrees; `gmc` is optional for creating them.

https://github.com/user-attachments/assets/34558c68-79ad-4fcd-972a-d896ed50097c

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

To compile from source, see [docs/build-your-terminal.md](docs/build-your-terminal.md).

## Acknowledgments

[Orca](https://github.com/stablyai/orca) showed that a worktree-aware terminal can work.

Every terminal leaf is a [Ghostty](https://github.com/ghostty-org/ghostty) `libghostty` surface.

## Licenses

MIT.
