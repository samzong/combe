# Combe terms

Use these words in code, commits, and docs. If a new domain word sticks, add it here instead of inventing a synonym.

| Term | Meaning |
| --- | --- |
| Combe | Product name. A valley you sit in. |
| Repo | A filesystem path the user registered. Not discovered by walking the disk. |
| Worktree | One row from `git worktree list --porcelain` for a registered git repo. |
| Folder workspace | A registered path that is not a git checkout. Shown as a single row. |
| Workspace | The selected worktree or folder workspace. The shell cwd of a tab. |
| Pin | A persisted flag on a workspace path. Survives refresh. |
| Catalog | The merge of `state.json` + git porcelain + folder fallback. Missing registered paths are skipped. |
| Sidebar | The left catalog list. Folded with ⌘B. |
| Session mark | Sidebar dot on a worktree row. Green when that workspace has a tab this app run; dim when it does not. |
| Tab | One split tree, opened on one workspace. |
| Pane | One node of a tab's split tree. Either an `NSSplitView` or a surface. |
| Surface | One libghostty surface: its own Metal layer, PTY, VT state, and font stack. The leaf of a split tree. |
| Split | A cut that reparents the focused surface into a new pane beside a fresh sibling. |
| Chrome | Everything AppKit draws: window, sidebar, tab bar, status line, splits. Never the terminal. |
| Status line | Bottom chrome row on the right pane. Present only while a quota chip is shown. |
| Quota | One status-line block of locally recorded CLI subscription snapshots. Each name is followed by remaining percent of its tightest 5h/7d window. A click opens one NSPopover with every available provider's windows. |
| Habits | Compiled-in preferences in `crates/combe/src/habits.rs`. There is no config file. |
| Occlusion | A hidden tab's surfaces are told to stop drawing via `ghostty_surface_set_occlusion`. |
| State file | `~/Library/Application Support/combe/state.json` |

Out of vocabulary: agent session, workbench, desk, execution host, orcad, theme, setting, preference pane.
