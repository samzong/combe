# Combe terms

Use these words in code, commits, and docs. If a new domain word sticks, add it here instead of inventing a synonym.

| Term | Meaning |
| --- | --- |
| Combe | Product name. A valley you sit in. |
| Repo | A filesystem path the user registered. Not discovered by walking the disk. |
| Worktree | One row from `git worktree list --porcelain` for a registered git repo. |
| Folder workspace | A registered path that is not a git checkout. Shown as a single row. |
| Workspace | The selected worktree, folder workspace, or Home workspace. New tabs open with that path as the shell cwd. |
| Home workspace | Built-in folder workspace at `$HOME`. Not a registered repo. Forced folder even if `$HOME` is a git checkout. Injected first in the sidebar catalog unless a catalog row already owns that path. Label `home`. One workspace row, no repo heading. |
| Catalog | The merge of `state.json` + git porcelain + folder fallback + the Home workspace when `$HOME` is not already a row. Missing registered paths are skipped. |
| Entry | An external open request: a file URL, an `ssh:` or `x-man-page:` URL, or the Finder service. `crates/combe/src/entry.rs` resolves it into a workspace to open or a shell line the user confirms. |
| Hop | `combe <path>` from a shell. Raises Combe and opens a new tab on the catalog row that owns the path. An unregistered path opens a tab of the Home workspace whose shell starts there, and registers nothing. A hop from a shell already inside Combe does nothing. |
| Sidebar | The catalog shown as a workspace chip, transient panel, or pinned sidebar. ⌘B toggles pinned and chip states. |
| Session mark | Sidebar dot on a worktree row. Green when that workspace has a tab this app run; dim when it does not. |
| Tab | One split tree, opened on one workspace. |
| Pane | One node of a tab's split tree. Either an `NSSplitView` or a surface. |
| Surface | One libghostty surface: its own Metal layer, PTY, VT state, and font stack. The leaf of a split tree. |
| Split | A cut that reparents the focused surface into a new pane beside a fresh sibling. |
| Chrome | Everything AppKit draws: window, sidebar, tab bar, status line, splits. Never the terminal. |
| Status line | Bottom chrome row on the right pane. Present only while a quota chip is shown. |
| Quota | One status-line block of locally recorded CLI subscription snapshots. Each name is followed by remaining percent of its tightest 5h/7d window. Hover or activation expands the chip upward with every available provider's windows. |
| Habits | Compiled-in preferences in `crates/combe/src/habits.rs`. There is no config file. |
| Occlusion | A hidden tab's surfaces are told to stop drawing via `ghostty_surface_set_occlusion`. |
| State file | `~/Library/Application Support/combe/state.json` |

Out of vocabulary: agent session, workbench, desk, execution host, orcad, theme, setting, preference pane.
