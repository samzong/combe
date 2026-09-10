# Combe

Combe is a worktree-aware terminal: a curated repo list on the left, real Ghostty terminals on the right. Click a worktree, land in that directory.

Built for personal use, exclusively on Apple Silicon Macs. Private GitHub Releases, no configuration files.

## Interface contract

[design.html](design.html) is the permanent, interactive GUI reference. Open it directly in a browser, or serve this directory for the interaction checks. Its CSS tokens and component states define the visual system; this document defines behavior and ownership. Terminal output and quota values in the HTML are samples.

The prototype's **Show spacing** inspector and [spacing inventory](spacing.md) identify native source values, live prototype measurements, and proposed changes by stable IDs. Recommendations remain proposals until approved; they do not change the current geometry contract. Select an area and measurement to inspect individual edges, including zero gaps and state-dependent empty space.

The inspector's **Corner radii** area and [radius inventory](radii.md) separate visible surfaces, transparent targets, focus rings, circles, and system-owned shapes. R01–R26 record current geometry: Tab glass and hover share 18 pt corners, Tab focus uses 16 pt, and list rows retain 16 pt. Other native shapes keep their existing values.

Before changing the GUI, update the relevant tokens, components, and interaction states in `design.html`, then update this document. Implement the same contract in AppKit and inspect the running window in both appearances. A screenshot or a passing build alone does not establish interaction parity. User-approved product decisions define the target; code and runtime establish what is implemented. Resolve discrepancies before calling a change complete.

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

The catalog always includes a Home workspace at `$HOME` unless a row already owns that path. Home is a folder workspace, not a registered repo: it is not written to `state.json`, and `$HOME` is not scanned for git worktrees until the user adds it. Its label is `~`. The workspace chip shows `~` while Home is selected. Startup opens the first catalog row, so a new window lands on Home.

Repos are never discovered by walking the disk.

## Non-goals

- Agents, chat overlays, command palettes
- In-app editor, browser, diffs, PR/issue chrome
- An SSH client, WSL, remote hosts, a PTY daemon that survives app updates
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
| State | `~/Library/Application Support/combe/state.json` | Registered repo paths. Nothing else persists. |
| Preferences | `crates/combe/src/habits.rs` | Font, colors, padding, shell, window size. Compiled in, not read from disk. |

Rejected:

| Option | Why not |
| --- | --- |
| GPUI (the original v0) | A second GPU surface from full libghostty does not compose into it, and pinning the crate meant tracking Zed. |
| `libghostty-vt` + own renderer | Means writing a glyph atlas and a Metal pipeline that Ghostty already ships and maintains. |
| Ghostty the application | Owns its own window, tabs, and splits. The point of Combe is the shell around them. |
| Walk `$HOME` for `.git` | Slow, noisy, and not how the UX works. |

## Window

One window, with a transparent titlebar and hidden title. Native traffic lights remain native window controls. The terminal and window share one background; terminal panes have no independent card, shadow, blur, or rounded border. Only split dividers separate terminal surfaces.

The sidebar is one glass surface with three states: a workspace chip, a transient catalog, and a pinned sidebar. The chip expands leftward under the traffic lights and downward into the catalog. Its right edge, selected-workspace text, add-repo button, and pin button stay fixed throughout the transition. The workspace trigger has no separate hover background. Traffic lights and tabs stay in their window positions through opening, closing, pinning, and unpinning. Tabs remain outside the sidebar's right edge in all states.

| Component | Geometry and appearance |
| --- | --- |
| Window content | 12 pt outer inset; 34 pt outer corners, square in full screen; 60 pt top chrome allocation |
| Workspace chip | 36 pt high; 18 pt corner radius; default left edge 84 pt and right edge 312 pt |
| Transient catalog | Default 300 pt wide, 12 pt from the top and left; height follows the list and is capped by the window; the same 18 pt corner radius as the chip |
| Pinned sidebar | Same header and width as the transient catalog; extends to 12 pt above the bottom; reserves terminal space |
| Header controls | 28 pt circular hit targets; 4 pt between add and pin, 4 pt from pin to the glass right edge; workspace text has a 12 pt leading inset within its trigger; add and pin remain available when the catalog is closed |
| Tabs | 36 pt high, 180 pt nominal width, 18 pt glass and hover corners; keyboard focus follows a 16 pt path inset by 2 pt with a 2 pt stroke; 12 pt gaps; active tab has the restrained glass treatment; long titles truncate; close and new-tab symbols have no separate fill or border; titles and symbols accept clicks across their full visible height |
| Repo heading | 30 pt high, folder symbol, trailing disclosure indicator, 12 pt medium system text |
| Workspace row | 34 pt high with 2 pt vertical spacing; 16 pt corners; session dot at 32 pt, label at 48 pt, and selected checkmark |
| Shortcut hint | 18 pt circle, 11 pt tabular numeral centered horizontally and vertically using its measured text height, subtle fill; replaces the checkmark without moving the row |
| Quota chip | 28 pt high, 14 pt corners; content-sized summary with 8 pt horizontal text insets on one glass surface; no separate summary hover fill |
| Quota details | 304 pt wide; the same glass surface and 14 pt corners as the chip; expands upward with a fixed bottom-left and fixed summary; 16 pt top and side insets, 12 pt bottom inset; 8 pt column and heading-to-row gaps, 30 pt rows, 16 pt between providers |
| Find bar | Native search field, match counter, previous/next and close controls; compact rounded surface in the focused pane's top-right |

The 12 pt gap also separates traffic lights from the chip, the chip from the first tab, and adjacent tabs. The default sidebar width is 300 pt; dragging its glass right border resizes a pinned sidebar within 160–420 pt. The resize cursor and hit area occupy the 4 pt immediately outside that border, without overlapping the catalog. After pinning, moving onto the border shows the resize cursor without another click. Resizing changes its right edge and available label width. It does not change width during ordinary window resizes. Narrow headers truncate text before displacing the action buttons. The custom sidebar split view explicitly forwards `mouseDown:` to `NSSplitView`; AppKit owns divider tracking and constraints. Catalog overflow scrolls. Full screen hides the system traffic lights without leaving dead controls.

Glass is confined to navigation and small control surfaces. Use native AppKit material that follows effective appearance and accessibility settings; do not change Ghostty's opacity or layer ownership to simulate glass. Preserve the deployment floor. Reduce Transparency makes materials opaque; Reduce Motion removes travel while keeping hover-intent delays.

The outer content view clips the shared background to the window corners while retaining native titled-window controls. The sidebar's shadow sits outside its clipped material. Its glass and catalog viewport animate together for 380 ms with the prototype's easing curve, keeping the first heading stationary. Scrollbars are enabled only when content exceeds the final viewport. Catalog padding is 8 pt at the top, 6 pt horizontally and 12 pt at the bottom; adjacent repo groups have a 16.5 pt gap including a subtle separator.

### Sidebar interaction

The app starts with the workspace chip. Hovering it for 150 ms expands the catalog. Passing across it sooner, dragging with a mouse button held, beginning a drag, losing window focus, or pressing Escape cancels pending entry. Explicit click or keyboard activation opens immediately. The panel stays open while the pointer crosses between its header and list.

Selecting a workspace keeps the catalog open and returns that workspace's existing tabs and focused pane. After entering the list, returning to the chip for 150 ms folds it; a quick crossing does not. Leaving the transient panel schedules closure after 250 ms, and re-entering cancels it. Keyboard focus inside the catalog protects it from pointer-exit closure. Escape, an outside click, or moving keyboard focus outside dismisses the transient panel. Pinned panels survive those dismissals. Pinning reserves space for terminal content; unpinning returns to the chip. Cmd-B toggles pinned and chip states.

Repo headings collapse their rows without changing the selected workspace or closing terminals. Collapse state and session marks last only for the app run. A green dot means that workspace still owns a tab; an inactive dot means it does not. The workspace chip has no session dot.

The row under the pointer uses the same neutral fill as the selected workspace, with appearance-adaptive text; selection is independent. Keep each control's tracking area alive while AppKit updates its visible rectangle, so geometry changes preserve paired enter and exit events.

While the catalog is transient or pinned, holding Command displays numbers for the first nine visible workspace rows. Cmd-1 through Cmd-9 selects those rows and preserves the panel. Collapsed repo rows do not receive numbers; numbering follows displayed order. Unassigned numbers are consumed. With the catalog closed, existing Ghostty tab-number bindings apply. Command keys must never reach the PTY.

### Appearance and typography

The default is Follow System. The native View menu also offers Light and Dark; an override lasts only for the current run and is not written to the state file. The prototype's Appearance selector exposes the same three choices outside the simulated app window.

| Role | Dark | Light |
| --- | --- | --- |
| Window and terminal background | `#161719` | `#f5f5f7` |
| Primary terminal text | `#e6edf3` | `#24292f` |
| Cursor | `#2f81f7` | `#0969da` |
| Terminal selection | `#e6edf3` with dark text | `#b6e3ff` with dark text |
| Chrome primary text | `#f1f1f2` | `#24292f` |
| Active tab title | `#e6e7e9` | `#292d33` |
| Secondary text and navigation symbols | `#9c9ea3` | `#656970` |
| Header action symbols and shortcut hints | `#c3c4c7` | `#555960` |
| Selected and hovered workspace | White at 7.1% | Black at 3.1% |
| Shortcut hint fill | White at 5.1% | Black at 2.4% |
| Live session mark | `#65c888` | `#248247` |
| Inactive session mark | `#85878d` | `#85878d` |

Native material supplies blur and accessibility fallback. An appearance-adaptive neutral tint, diagonal highlight and thin white edge provide the prototype's glass hierarchy: chips and active tabs stay light, while expanded catalog and quota panels gain body. Light glass has a brighter edge; dark glass keeps a restrained highlight. Chrome colors resolve through the effective appearance without changing the window or terminal background. Quota warning thresholds and system warning colors remain unchanged.

Chrome uses the system font: 12 pt for chips, tabs, repo headings and quota values; 13 pt for workspace rows and semibold provider headings. Percentages and shortcut hints use tabular digits. Terminal fonts remain Fira Code with the explicit Noto Sans Mono CJK SC codepoint fallback, at the compiled 13 pt default. Appearance changes never change font family, size, weight, spacing, or CJK fallback. Both ANSI palettes remain compiled in `habits.rs`.

Every terminal surface has 8 pt of base padding on all four sides, including split and zoomed panes. Ghostty balances the remaining space around its whole-cell grid. Outer terminal layout stays unchanged. The quota summary starts at the terminal area's left edge, with the same 8 pt base text inset.

Effective-appearance changes update the window background, native material and semantic text colors, Ghostty configuration, and every surface, including hidden tabs and zoomed panes. Shells, scrollback, split trees, selection ownership and user-adjusted font size survive. New surfaces inherit the effective appearance. Check Latin, CJK, bold text, ANSI colors, cursor and selection in both modes; no hard-coded light text may remain on light glass.

The status line sits under the terminals, not under the sidebar. Combe reads local quota snapshots only: Claude's `~/.claude/rate-limits.json` (or `$CLAUDE_CONFIG_DIR/rate-limits.json`) and Codex's `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl` (default `~/.codex`). It never reads credentials, accesses Keychain, queries usage endpoints, starts a CLI, or installs hooks. Claude's externally configured statusline writer supplies the `rate_limits` object with an `updated_at` epoch timestamp, which Combe ignores. The Claude file is read whole up to 64 KiB; larger files are unavailable. Missing or invalid snapshots hide that provider.

Codex discovery enumerates session file metadata in the background, including old dates for resumed sessions. It examines at most eight recently modified files and at most the last 1 MiB of each, skips incomplete lines, and selects the newest timestamped `event_msg` / `token_count` snapshot for the `codex` limit bucket (or older records without a limit ID). Unchanged files reuse their parsed in-memory snapshots. Windows are classified by their duration, not by primary/secondary position; unsupported durations are omitted. A quota event outside the read budget is unavailable rather than triggering an unbounded history scan.

One block lists each available provider and the remaining percent of its tighter 5h/7d window. Hover or explicit activation grows the same chip upward to show the windows, including Claude Fable if the local payload supplies it. Provider headings identify rows; do not add a Window / Remaining / Resets in header. Each row presents window, remaining percent, and reset interval, with an accessible label explaining the values. The details use the same 150 ms entry, 250 ms exit, drag cancellation, keyboard-focus protection and Escape behavior as the transient sidebar. Used quota at 60 percent is orange and at 80 percent is red. If only one provider is available, show only that provider; if none is available, remove the chip and its reserved status height. These are last-recorded snapshots, not a claim about the current login: neither source establishes account identity, and other devices or account switches can make values stale. Passing a reset time does not manufacture a fresh percentage. Combe persists no quota data. Startup reads once; while focused, it checks every 15 minutes, and activation checks only after five minutes since the previous check. Both intervals are compiled into `habits.rs`.

Every tab of every worktree lives in the same content view. Only the active tab of the selected worktree is unhidden; every surface everywhere else gets `ghostty_surface_set_occlusion(false)` and stops drawing. The selection is one pointer per worktree plus the current worktree, so the sidebar highlight and the tab bar cannot disagree. Each tab remembers its focused pane. Closing a background pane preserves the current input focus.

A tab is named by the focused pane's terminal title, and falls back to the worktree's own name until a title arrives. Ghostty's shell integration writes the running command while a command runs and a shortened path at the prompt, so a pane running `claude` names its tab `claude`; splitting a tab means the name follows whichever pane holds focus. Titles reach the app through `GHOSTTY_ACTION_SET_TITLE` on the runtime action callback, which resolves back to the pane with `ghostty_surface_userdata`. There is no process-name API in `ghostty.h`, and none is invented: a shell without integration keeps the worktree name.

Splitting reparents the focused surface into a fresh `NSSplitView` and adds a sibling. Closing removes the leaf and, when a pane is left with a single child, collapses that pane into its parent. Ghostty asks for a close through `close_surface_cb`, which queues the surface and drains it on the main queue.

### Tab Overview

The grid button at the right of the tab bar and Cmd-Shift-Backslash toggle an overview of the current workspace only. The View menu exposes the same command. Each card contains the whole tab's last rendered terminal image, including its visible split arrangement or zoomed pane, and its title. Images are static, may lag behind background output, live only in memory, and are released when the overview closes. A surface without a rendered image uses its terminal background until it has a frame.

Clicking a card enters that tab. Return enters the tab focused before opening the overview, or a card reached through normal keyboard focus. Escape returns without switching tabs. There is no arrow-key navigation, reordering, closing, or cross-workspace aggregation in the overview. Other unmodified keys must not reach the terminal while the overview is open. Command shortcuts dismiss the overview before following the existing application command path.

The overview enters and exits with a 180 ms ease-in-out crossfade. Reduce Motion switches immediately. Repeated toggles replace the current transition; focus and tab selection update immediately without waiting for animation. The overview covers the terminal area without reparenting or resizing terminal surfaces. Its opaque background matches the window background in both appearances and redraws when the effective appearance changes. The sidebar and tab bar retain their positions. Cards use 8 pt padding, 24 pt grid gaps and outer padding, a 20 pt title line, and the selected-row neutral fill. The grid scrolls vertically as needed and follows light and dark appearance. Opening or dismissing it preserves sessions, split geometry and zoom; Escape restores the prior responder when it is still available.

Zooming moves the focused surface above its hidden split tree and leaves a placeholder at its original position. Toggling again restores that position without recreating terminals. Zoom is retained per tab; closing a pane or adding a split restores the tree first. A tab with one surface is unchanged.

Command-modified keys belong to the app and never reach the PTY. Control sequences always reach the PTY. Releasing a selection on a surface copies that text to the system clipboard (`copy-on-select`). Cmd-C still copies. Terminal clipboard reads are denied by default; ordinary paste remains available, while unsafe pastes are denied with the system alert sound. Cmd-click on a terminal URL sends `GHOSTTY_ACTION_OPEN_URL`; Combe opens `http`, `https`, and `mailto` in the default handler.

Search lives in libghostty. `GHOSTTY_ACTION_START_SEARCH` adds a find bar as a subview in the top-right corner of that surface; every edit sends `search:<needle>` through `ghostty_surface_binding_action`, and `SEARCH_TOTAL` / `SEARCH_SELECTED` feed the match counter. `END_SEARCH` removes the bar and returns focus to the surface.

| Key | Action |
| --- | --- |
| Cmd-Q | Quit. Asks first when any terminal still runs a foreground process |
| Cmd-H | Hide Combe |
| Opt-Cmd-H | Hide other applications |
| Cmd-M | Minimize the window |
| Cmd-T | New tab |
| Cmd-W | Close pane, or the tab when it is the last pane. Asks first when the pane, or any pane of the tab, still runs a foreground process. The last tab of a workspace ends that session; the window closes only when no sessions remain |
| Opt-Cmd-W | Close every titled window |
| Cmd-D | Split right |
| Cmd-Shift-D | Split down |
| Cmd-Shift-Backslash | Toggle Tab Overview |
| Cmd-Shift-Return | Zoom the focused split, or restore its layout |
| Cmd-Shift-[ / ] | Previous / next tab |
| Cmd-1 … Cmd-9 | With the catalog open: select visible workspace N. With it closed: Ghostty tab N / last-tab bindings |
| Cmd-Alt-Arrows | Focus the neighbouring split in that direction |
| Cmd-[ / ] | Focus the previous / next split, through Ghostty's `goto_split` binding |
| Cmd-F | Open the find bar on the focused pane, through Ghostty's `start_search`. Return / Shift-Return step through matches, Escape or × closes it |
| Cmd-G / Cmd-Shift-G | Next / previous match |
| Cmd-B | Pin the sidebar, or return to the workspace chip |
| Cmd-C / Cmd-V | Copy / paste through `ghostty_surface_binding_action`; while a text field such as the find bar has focus they act on that field |
| Ctrl-Cmd-F | Toggle full screen, the standard `toggleFullScreen:` item in a View menu |

## CLI

One binary, two entry points, separated by `argv[0]`. Running it as `<something>.app/Contents/MacOS/Combe` with no arguments opens the window; that is what the Dock, Finder, and `open -a` do. Every other invocation is the CLI, and no arguments means help.

`argv[0]` rather than `current_exe()`, because `make install` symlinks `combe` onto `PATH` and `current_exe()` resolves that symlink back into the bundle.

`list`, `add`, `remove`, and `cleanup` read and write `state.json` and never touch AppKit. `cleanup` drops registered repo paths whose directory no longer exists, which is the recovery path for a checkout deleted outside Combe.

Both the GUI and CLI stop state edits when reading the state file fails. Saves atomically replace the file, preserving the previous contents if writing fails.

## System entry points

`Info.plist` registers Combe with LaunchServices so other applications can hand it work. macOS has no default-terminal role: browser and mail are privileged LaunchServices roles, terminals are not. What exists instead:

- Document types. `public.directory` opens the folder as a workspace. `public.unix-executable` and `.command`, `.sh`, `.zsh`, `.bash` open a tab in the file's parent. Both claim `Alternate` rank, so Combe appears in Finder's Open With and Get Info without displacing the current handler.
- URL schemes. `ssh:` and `x-man-page:`. `telnet:` is not claimed, because macOS ships no `telnet` binary and the handler would always fail.
- One service, "New Combe Tab Here", for the Finder right-click menu and any application that sends a file path.

`crates/combe/src/entry.rs` is the boundary. It turns a path or URL into either a workspace to open or a shell line to type, rejects hosts, users, man pages, and sections outside a character whitelist, quotes every interpolated value, and separates options from operands with `--`. It has no AppKit dependency, so `cargo test -p combe` covers the injection cases directly.

Executable file paths containing ASCII control characters are rejected before constructing shell input, because terminal line editors interpret those characters before shell quoting applies. Anything that would execute a command asks first. The tab appears only after the user confirms an alert showing the exact line. That line is then written into the new surface as `initial_input`, not run as the surface command, so the tab stays an ordinary long-lived workspace session afterwards.

## Catalog

A **repo** is a path the user added. A **worktree** is one record from `git worktree list --porcelain` for that repo. A **folder workspace** is a registered path that is not a git checkout: one row, no branch. A **Home workspace** is a built-in folder workspace at `$HOME`.

1. Load `state.json`
2. For each repo, `git -C <path> worktree list --porcelain`
3. If the registered path is not a directory, skip it and keep the rest. Leave it in `state.json`.
4. If `git rev-parse --git-dir` fails, emit one folder row
5. Skip worktrees Git reports as `prunable`; their directory is gone
6. If `$HOME` is a directory and no row already owns that path, prepend a folder workspace labelled `~`. Do not write it to `state.json`. Do not scan `$HOME` for git worktrees unless the user registered it.

Identity is the resolved worktree path. `list`, `add`, `remove`, and `cleanup` still operate only on registered repos.

The window loads the catalog synchronously on startup so the first workspace can open at once. Every later refresh, when the app becomes active, after adding repos or opening a new tab, or when the user clicks an empty part of the sidebar, runs Git on a background thread and applies the result on the main queue; a refresh requested during a scan runs once more after it. Tab selection and session marks reuse that catalog without running Git.

`crates/combe-catalog` has no AppKit dependency. `cargo test -p combe-catalog` is the fast loop.

## Layout

```text
crates/ghostty-sys     zig build + bindgen over vendor/ghostty
crates/combe-catalog   state.json, git porcelain, folder fallback, Home workspace
crates/combe
  main.rs              CLI or GUI, GHOSTTY_RESOURCES_DIR, NSApplication
  cli.rs               list, add, remove, cleanup
  entry.rs             external file, URL, and service requests
  ghostty.rs           ghostty_init, app lifecycle, runtime callbacks
  surface.rs           one NSView per libghostty surface
  split.rs             NSSplitView tree: leaf, divide, close, collapse
  tabs.rs              tabs keyed by worktree, active tab per worktree
  overview.rs          static tab images and overview grid
  window.rs            window, chrome, menu, sidebar, dispatch
  chrome_view.rs       shared clickable views, native glass, and symbols
  sidebar.rs           catalog adapter
  quota.rs             Claude and Codex subscription windows
  quota_panel.rs       quota cache, refresh scheduling, chip, and expanding details
  habits.rs            every preference, compiled in
```

The quota panel owns its cached usage, polling state, and views. The window mounts it, supplies callbacks for current window activity and layout, and uses its height when positioning terminal content. Shared chrome views dispatch clicks through callbacks without owning workspace or quota state.

## GUI acceptance

Preserve these capabilities when changing chrome. Prototype states demonstrate appearance and interaction intent; native verification proves terminal and system behavior.

| Capability | Required verification |
| --- | --- |
| Workspace sessions | Switch away and back; tab, split, focused pane, title, shell PID and output survive; repo collapse does not close sessions |
| Tabs | Create, select, close and navigate by key; keep workspace ownership; closing the last workspace tab selects another live workspace, or closes the window when none remain |
| Splits and zoom | Split right and down, nest and resize, move focus, zoom and restore, close a leaf and promote its sibling; preserve PTYs and focus |
| Catalog | Add multiple repos with the native directory picker; include folder workspaces; Home is present without adding a repo; adding `$HOME` as a repo replaces the built-in row; refresh on activation without blocking input |
| Hover panels | Check entry, return-to-chip and exit delays; fast pointer sweeps, drag, outside click, Escape, blur, keyboard focus, pinning and resizing |
| Workspace shortcuts | Command hints, visible-row numbering after repo collapse, Cmd-1 through Cmd-9, and closed-catalog tab shortcuts; no Command input reaches the PTY |
| Quota | Both providers, each provider alone, missing snapshots, 5h/7d and Fable rows, warning colors and expired reset time; no network or credential access |
| Search and clipboard | Open find, type, step both directions, close and restore terminal focus; copy, ordinary paste, copy-on-select and URL opening remain available |
| Window and process lifetime | Reopen, minimize, full screen, close and quit confirmation with a foreground process; background child exit preserves current focus |
| Appearance | System/light/dark in the same process, including hidden and zoomed surfaces; typography and terminal state remain stable |

Run `make check` and inspect the real AppKit window. After menu changes verify Cmd-H, Opt-Cmd-H, Cmd-M, Cmd-Q, Cmd-W and Opt-Cmd-W, including confirmation cancellation. Serve this directory, open `design.html`, and run the prototype checks in the browser console:

```js
const checks = await import("./design.check.js");
for (const name of ["default", "checkQuota", "checkShortcuts", "checkHoverIntent", "checkAppearanceAndComponents", "checkSpacing"]) {
  console.log(name, await checks[name]());
}
```

Inspect both appearances and the component selector, including Typography. User-visible wording and visual taste require the owner's pass.

## Decisions

- AppKit shell over any Rust GUI framework: splits and tabs are exactly what AppKit is good at, and libghostty needs a real `NSView` host.
- A complete libghostty surface per leaf over a VT library plus a home-grown renderer.
- Static linking: `libghostty-internal.a` means no dylib to bundle, but `GHOSTTY_RESOURCES_DIR` is baked at compile time and must be relocated into the app bundle.
- User-curated repos over disk scans.
- Git porcelain over libgit2.
- Preferences as Rust constants over a config file.
- `initial_input` plus a confirmation alert over `config.command`: libghostty always runs `command` through `/bin/sh -c`, and an external URL must never reach a shell without the user seeing the line.
