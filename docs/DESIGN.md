# Combe

Combe is a worktree-aware terminal: a curated repo list on the left, real Ghostty terminals on the right. Click a worktree, land in that directory.

Built for personal use, exclusively on Apple Silicon Macs. Private GitHub Releases, no configuration files.

## Product

Combe is a lightweight native terminal organized around workspaces. Its mission is to make switching projects mean returning to the tabs, splits, and running tools left there during the current app session, without finding windows or rebuilding layouts.

### Philosophy

**Restraint, without compromise.**

- Restraint in scope: build for the owner's actual daily work on macOS. Do not add features for hypothetical users or pursue a general-purpose terminal product.
- No compromise on experience: every retained feature should be dependable, easy to use, and consistent with native macOS interactions. A small feature set is no excuse for a poor experience.
- Do fewer things, and do the needed things well. CLI tools such as Claude Code and Codex run inside terminals; Combe does not become an IDE or an agent management platform.

### Workspaces

A heavy terminal user keeps many git checkouts and linked worktrees. The missing piece is a directory of those trees, not another multiplexer, editor, or agent overlay.

The user curates the repo list by hand. Combe reads `git worktree list --porcelain` for each registered repo and shows every tree git already knows.

A worktree is a session, not a shortcut. Each row owns its own set of tabs; selecting a row swaps in that worktree's tabs and returns to the one it was left on. New tabs belong to the selected worktree. Closing the last tab of a workspace ends that session. If another workspace still has tabs, the window switches to it; if none remain, the window closes.

Repos are never discovered by walking the disk.

## Non-goals

- Agents, chat overlays, command palettes
- In-app editor, browser, diffs, PR/issue chrome
- SSH, WSL, remote hosts, a PTY daemon that survives app updates
- A settings GUI, a theme market, cloud sync, user configuration files
- Creating or deleting worktrees (that stays with `git` / `gmc`)
- A hand-written VT parser, glyph atlas, or renderer

## Stack

| Layer | Choice | Why |
| --- | --- | --- |
| Shell chrome | AppKit via `objc2` | Window, sidebar, tabs, and splits are the host's job. No GPUI, winit, wgpu, WebView, OpenGL, or Electron. |
| Terminal leaf | Full `libghostty` surface | Metal renderer, PTY, VT, and CoreText in one embedded surface. Ghostty owns its `IOSurfaceLayer` and drives its own frames. |
| Ghostty build | `vendor/ghostty` submodule, built by `zig`, linked statically | `crates/ghostty-sys` runs `zig build -Dapp-runtime=none` and bindgen over `include/ghostty.h`. Nothing is invented beyond that header. |
| Git | User's `git` binary | `worktree list --porcelain` is the catalog. Git 2.25 is the floor. |
| State | `~/Library/Application Support/combe/state.json` | Registered repo paths and pins. Nothing else persists. |
| Preferences | `crates/combe/src/habits.rs` | Font, colors, padding, shell, window size. Compiled in, not read from disk. |

Rejected:

| Option | Why not |
| --- | --- |
| GPUI (the original v0) | A second GPU surface from full libghostty does not compose into it, and pinning the crate meant tracking Zed. |
| `libghostty-vt` + own renderer | Means writing a glyph atlas and a Metal pipeline that Ghostty already ships and maintains. |
| Ghostty the application | Owns its own window, tabs, and splits. The point of Combe is the shell around them. |
| Walk `$HOME` for `.git` | Slow, noisy, and not how the UX works. |

## Window

One window. `FullSizeContentView` with a transparent titlebar and hidden title, so a single 40pt row holds the traffic lights, the sidebar toggle, the tab pills, and `+`. Spotlight, Raycast, or the Dock activating an already-running Combe goes through `applicationShouldHandleReopen:hasVisibleWindows:`, which deminiaturizes if needed and calls `makeKeyAndOrderFront` so the system can switch to the space that holds the window.

```text
NSWindow
  contentView (NSView)
    NSSplitView (vertical)
      NSView                      sidebar
        header spacer (40pt)
        NSScrollView              collapsible repo groups and worktree rows
      NSView                      right pane
        tab bar (40pt)            the selected worktree's tabs, per-tab close, "+"
        content                   one root per tab, every worktree's
          NSView (tab root)
            NSSplitView …         nested panes
              SurfaceView         one libghostty surface
        status line (24pt)        one quota block; hidden when no local quota snapshot
    add repo and sidebar toggle   overlay, trailing edge of the sidebar
```

The add-repo `+` and sidebar toggle live on the content view rather than inside the sidebar, so both remain available when the sidebar collapses. While the sidebar is open the buttons track its trailing edge; collapsed, they fall back to a leading inset — 78pt to clear the traffic lights, 8pt in full screen where there are none. The tab bar leaves room for both buttons.

Clicking a repo heading collapses or expands its worktree rows without closing terminals or changing the selected workspace. Collapsed groups are keyed by registered repo path and retained only for the current app run; all groups start expanded. Each worktree row shows a session mark: green when that workspace has a tab this app run, dim when it does not. The mark is not persisted and is not a pin.

Chrome geometry is a layout pass, not a one-shot. The content view overrides `layout`, and the split view delegate answers `splitViewDidResizeSubviews:`; between them every window resize, full-screen transition, divider drag, and collapse re-runs the same placement. Sidebar visibility is read back from `isSubviewCollapsed`, never cached, because a drag can collapse the pane without the app asking.

The delegate pins the sidebar's width when the window resizes, clamps a drag to 160–420pt, and widens the divider's hit area. The split view draws no divider while the sidebar is collapsed, and the layout pass invalidates it, because the chrome paints no background and would otherwise leave the old divider standing in the 40pt top bar.

The window and every terminal follow the macOS light or dark appearance, including changes while Combe is running. Both palettes are compiled into `habits.rs`; there is no appearance setting. The content view observes effective-appearance changes, updates the window background and Ghostty configuration, and forwards the color scheme to every terminal, including hidden tabs. New surfaces inherit the current scheme. Shells and split trees stay alive during the change.

Every chrome view is transparent, and Ghostty uses the same background as the window, so the sidebar, tab bar and terminal are one surface separated only by a hairline divider. Chrome text and selection highlights use adaptive AppKit colors. Chrome text uses `habits::FONT_SIZE`, the same number the terminal font is set to.

The status line sits under the terminals, not under the sidebar. Combe reads local quota snapshots only: Claude's `~/.claude/rate-limits.json` (or `$CLAUDE_CONFIG_DIR/rate-limits.json`) and Codex's `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl` (default `~/.codex`). It never reads credentials, accesses Keychain, queries usage endpoints, starts a CLI, or installs hooks. Claude's externally configured statusline writer supplies the `rate_limits` object with an `updated_at` epoch timestamp, which Combe ignores. The Claude file is read whole up to 64 KiB; larger files are unavailable. Missing or invalid snapshots hide that provider.

Codex discovery enumerates session file metadata in the background, including old dates for resumed sessions. It examines at most eight recently modified files and at most the last 1 MiB of each, skips incomplete lines, and selects the newest timestamped `event_msg` / `token_count` snapshot for the `codex` limit bucket (or older records without a limit ID). Unchanged files reuse their parsed in-memory snapshots. Windows are classified by their duration, not by primary/secondary position; unsupported durations are omitted. A quota event outside the read budget is unavailable rather than triggering an unbounded history scan.

One block lists each available provider and the remaining percent of its tighter 5h/7d window. A click opens one NSPopover with the windows, including Claude Fable if the local payload supplies it. These are last-recorded snapshots, not a claim about the current login: neither source establishes account identity, and other devices or account switches can make values stale. Passing a reset time does not manufacture a fresh percentage. Combe persists no quota data. Startup reads once; while focused, it checks every 15 minutes, and activation checks only after five minutes since the previous check. Both intervals are compiled into `habits.rs`.

Every tab of every worktree lives in the same content view. Only the active tab of the selected worktree is unhidden; every surface everywhere else gets `ghostty_surface_set_occlusion(false)` and stops drawing. The selection is one pointer per worktree plus the current worktree, so the sidebar highlight and the tab bar cannot disagree. Each tab remembers its focused pane. Closing a background pane preserves the current input focus.

A tab is named by the focused pane's terminal title, and falls back to the worktree's own name until a title arrives. Ghostty's shell integration writes the running command while a command runs and a shortened path at the prompt, so a pane running `claude` names its tab `claude`; splitting a tab means the name follows whichever pane holds focus. Titles reach the app through `GHOSTTY_ACTION_SET_TITLE` on the runtime action callback, which resolves back to the pane with `ghostty_surface_userdata`. There is no process-name API in `ghostty.h`, and none is invented: a shell without integration keeps the worktree name.

Splitting reparents the focused surface into a fresh `NSSplitView` and adds a sibling. Closing removes the leaf and, when a pane is left with a single child, collapses that pane into its parent. Ghostty asks for a close through `close_surface_cb`, which queues the surface and drains it on the main queue.

Zooming moves the focused surface above its hidden split tree and leaves a placeholder at its original position. Toggling again restores that position without recreating terminals. Zoom is retained per tab; closing a pane or adding a split restores the tree first. A tab with one surface is unchanged.

Command-modified keys belong to the app and never reach the PTY. Control sequences always reach the PTY. Releasing a selection on a surface copies that text to the system clipboard (`copy-on-select`). Cmd-C still copies. Terminal clipboard reads are denied by default; ordinary paste remains available, while unsafe pastes are denied with the system alert sound. Cmd-click on a terminal URL sends `GHOSTTY_ACTION_OPEN_URL`; Combe opens `http`, `https`, and `mailto` in the default handler.

| Key | Action |
| --- | --- |
| Cmd-Q | Quit |
| Cmd-H | Hide Combe |
| Opt-Cmd-H | Hide other applications |
| Cmd-M | Minimize the window |
| Cmd-T | New tab |
| Cmd-W | Close pane, or the tab when it is the last pane. The last tab of a workspace ends that session; the window closes only when no sessions remain |
| Opt-Cmd-W | Close every titled window |
| Cmd-D | Split right |
| Cmd-Shift-D | Split down |
| Cmd-Shift-Return | Zoom the focused split, or restore its layout |
| Cmd-Alt-Left / Right | Previous / next tab |
| Cmd-B | Fold or unfold the sidebar |
| Cmd-C / Cmd-V | Copy / paste through `ghostty_surface_binding_action` |
| Ctrl-Cmd-F | Toggle full screen, the standard `toggleFullScreen:` item in a View menu |

## CLI

One binary, two entry points, separated by `argv[0]`. Running it as `<something>.app/Contents/MacOS/Combe` with no arguments opens the window; that is what the Dock, Finder, and `open -a` do. Every other invocation is the CLI, and no arguments means help.

`argv[0]` rather than `current_exe()`, because `make install` symlinks `combe` onto `PATH` and `current_exe()` resolves that symlink back into the bundle.

`list`, `add`, `remove`, and `cleanup` read and write `state.json` and never touch AppKit. `cleanup` drops registered repo paths and pins whose directory no longer exists, which is the recovery path for a checkout deleted outside Combe.

Both the GUI and CLI stop state edits when reading the state file fails. Saves atomically replace the file, preserving the previous contents if writing fails.

## Catalog

A **repo** is a path the user added. A **worktree** is one record from `git worktree list --porcelain` for that repo. A **folder workspace** is a registered path that is not a git checkout: one row, no branch.

1. Load `state.json`
2. For each repo, `git -C <path> worktree list --porcelain`
3. If the registered path is not a directory, skip it and keep the rest. Leave it in `state.json`.
4. If `git rev-parse --git-dir` fails, emit one folder row
5. Merge pins

Identity is the resolved worktree path. Pins are a set of those paths.

The window refreshes the catalog on startup, when the app becomes active, and after adding repos or opening a new tab. Tab selection and session marks reuse that catalog without running Git.

`crates/combe-catalog` has no AppKit dependency. `cargo test -p combe-catalog` is the fast loop.

## Layout

```text
crates/ghostty-sys     zig build + bindgen over vendor/ghostty
crates/combe-catalog   state.json, git porcelain, folder fallback
crates/combe
  main.rs              CLI or GUI, GHOSTTY_RESOURCES_DIR, NSApplication
  cli.rs               list, add, remove, cleanup
  ghostty.rs           ghostty_init, app lifecycle, runtime callbacks
  surface.rs           one NSView per libghostty surface
  split.rs             NSSplitView tree: leaf, divide, close, collapse
  tabs.rs              tabs keyed by worktree, active tab per worktree
  window.rs            window, chrome, menu, sidebar, dispatch
  chrome_view.rs       shared clickable and flipped AppKit views
  sidebar.rs           catalog adapter
  quota.rs             Claude and Codex subscription windows
  quota_panel.rs       quota cache, refresh scheduling, status chip, and popover
  habits.rs            every preference, compiled in
```

The quota panel owns its cached usage, polling state, and views. The window mounts it, supplies callbacks for current window activity and layout, and uses its height when positioning terminal content. Shared chrome views dispatch clicks through callbacks without owning workspace or quota state.

## Phases

1. **Window.** Window, sidebar, one working zsh. Done.
2. **Desk.** Tabs, draggable splits, correct focus, hidden tabs stop drawing. Done.
3. **Habits (current).** Pin toggling from the sidebar, worktree history back and forward, a default split layout in `habits.rs`.
4. **Stop.** No further features.

## Decisions

- AppKit shell over any Rust GUI framework: splits and tabs are exactly what AppKit is good at, and libghostty needs a real `NSView` host.
- A complete libghostty surface per leaf over a VT library plus a home-grown renderer.
- Static linking: `libghostty-internal.a` means no dylib to bundle, but `GHOSTTY_RESOURCES_DIR` is baked at compile time and must be relocated into the app bundle.
- User-curated repos over disk scans.
- Git porcelain over libgit2.
- Preferences as Rust constants over a config file.
