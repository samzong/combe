# Combe

Combe is a worktree-aware terminal: a curated repo list on the left, real Ghostty terminals on the right. Click a worktree, land in that directory.

Built for personal use, exclusively on Apple Silicon Macs. Private GitHub Releases, no configuration files.

## Interface contract

[design.html](design.html) is the permanent interactive GUI reference: its CSS tokens and component states define visuals; this document defines behavior and ownership. Open it directly, or serve this directory for interaction checks. Terminal output and quota values are samples.

The **Show spacing** inspector, [spacing inventory](spacing.md), and [radius inventory](radii.md) use stable IDs to distinguish native source values, live prototype measurements, and proposals. Select an area and measurement to inspect edges, zero gaps, and state-dependent space. **Corner radii** R01–R26 distinguish visible surfaces, transparent targets, focus rings, circles, and system-owned shapes. Recommendations require approval; current geometry is specified below.

For GUI changes, update the prototype first, then this contract, then AppKit. Verify the running window in both appearances; a screenshot or build alone cannot prove interaction parity. User-approved decisions define the target; code and runtime establish current behavior. Resolve discrepancies before completion.

## Product

Combe is a lightweight native terminal organized around workspaces. Its mission is to make switching projects mean returning to the tabs, splits, and running tools left there during the current app session, without finding windows or rebuilding layouts.

### Philosophy

**Restraint, without compromise.**

- Restraint in scope: build for the owner's actual daily work on macOS. Do not add features for hypothetical users or pursue a general-purpose terminal product.
- No compromise on experience: every retained feature should be dependable, easy to use, and consistent with native macOS interactions. A small feature set is no excuse for a poor experience.
- Do fewer things, and do the needed things well. CLI tools such as Claude Code and Codex run inside terminals; Combe does not become an IDE or an agent management platform.
- When experience goals conflict, prioritize reliable sessions and predictable interaction, then ease of switching, then visual polish.
- New features must address recurring friction in the owner's actual work. Check whether existing tools or native capabilities already cover the need, and weigh the state, failure modes, and maintenance Combe would take on.
- Revisit design choices when repeated use shows their costs outweigh their benefits. Explicit non-goals and behavior contracts remain binding until the owner changes them.

### Workspaces

The user registers repos by hand. Combe lists their worktrees through Git; it never scans the disk for repos.

Each workspace owns its tabs and remembers its active tab. Selecting a row returns to that session; new tabs belong to it. Closing its last tab switches to another workspace with tabs, or closes the window if none remain.

The built-in Home workspace is one `home` row without a repo heading or disclosure; its chip also reads `home`. The [catalog](#catalog) injects it first unless a row owns `$HOME`. Startup opens the first row. Home is not registered or persisted, and `$HOME` is scanned for worktrees only when the user registers it.

## Non-goals

- Agents, chat overlays, command palettes
- In-app editor, browser, diffs, PR/issue chrome
- An SSH client, WSL, remote hosts, a PTY daemon that survives app updates
- A settings GUI, a theme market, cloud sync, user configuration files
- A third quota provider, quota settings, or usage fetched from the network
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

One window with a transparent titlebar, hidden title, and native traffic lights. Window and terminals share a background; panes have no independent card, shadow, blur, or rounded border. Only split dividers separate surfaces.

The sidebar's single glass surface has chip, transient catalog, and pinned states. It expands leftward under the traffic lights and downward, keeping its right edge, selected-workspace text, add and pin buttons fixed. The workspace trigger has no separate hover background. Traffic lights and tabs keep their window positions through every transition; tabs always remain beyond the sidebar's right edge.

| Component | Geometry and appearance |
| --- | --- |
| Window content | 12 pt outer inset; 34 pt outer corners, square in full screen; 60 pt top chrome allocation |
| Workspace chip | 36 pt high; 18 pt corner radius; default left edge 84 pt and right edge 312 pt |
| Transient catalog | Default 300 pt wide, 12 pt from the top and left; height follows the list and is capped by the window; the same 18 pt corner radius as the chip |
| Pinned sidebar | Same header and width as the transient catalog; extends to 12 pt above the bottom; reserves terminal space |
| Header controls | Traffic lights, workspace text, add and pin share a vertical center 30 pt below the window content top in all sidebar states; 28 pt hit targets; 4 pt between add and pin, 4 pt from pin to the glass right edge; workspace text has a 12 pt leading inset within its trigger; add and pin remain available when the catalog is closed |
| Tabs | 36 pt high, 180 pt nominal width, 18 pt glass and hover corners; keyboard focus follows a 16 pt path inset by 2 pt with a 2 pt stroke; 12 pt gaps; active tab has the restrained glass treatment; long titles truncate; overflow scrolls horizontally only; close and new-tab symbols have no separate fill or border at rest; close has a 20 pt target, 10 pt right inset, and appears on tab hover or keyboard focus without moving the title |
| Repo heading | 30 pt high, folder symbol, trailing disclosure indicator, 12 pt medium system text |
| Workspace row | 34 pt high with 2 pt vertical spacing; 16 pt corners; session dot at 32 pt, label at 48 pt, and selected checkmark |
| Shortcut hint | 18 pt circle, 11 pt tabular numeral centered horizontally and vertically using its measured text height, subtle fill; replaces the checkmark without moving the row |
| Quota chip | 28 pt high, 14 pt corners; content-sized summary with 8 pt horizontal text insets on one glass surface; no separate summary hover fill |
| Quota details | 304 pt wide; the same glass surface and 14 pt corners as the chip; expands upward with a fixed bottom-left and fixed summary; 16 pt top and side insets, 12 pt bottom inset; 8 pt column and heading-to-row gaps, 30 pt rows, 16 pt between providers |
| Find bar | Native search field, match counter, previous/next and close controls; compact rounded surface in the focused pane's top-right |

The 12 pt gap separates traffic lights, chip, and tabs. Dragging a pinned sidebar's right glass border resizes its 300 pt default width within 160–420 pt, changing the right edge and label width. Its 4 pt resize target sits immediately outside the glass, clear of the catalog, and shows the resize cursor without another click after pinning. Ordinary window resizing preserves sidebar width. Narrow headers truncate text before displacing buttons; catalog overflow scrolls. The custom split view forwards `mouseDown:` to `NSSplitView`, which owns divider tracking and constraints. Full screen hides traffic lights without leaving dead controls.

Glass is confined to navigation and small controls. Native AppKit material follows effective appearance and accessibility settings. Preserve the deployment floor and Ghostty's opacity and layer ownership. Reduce Transparency makes materials opaque; Reduce Motion removes travel but keeps hover-intent delays.

The outer content view clips the shared background to the window corners and retains native titled-window controls. The sidebar shadow stays outside its clipped material. Glass and catalog viewport animate together for 380 ms with the prototype easing, keeping the first row stationary. Scrollbars appear only when content exceeds the final viewport. Catalog padding is 8 pt top, 6 pt horizontal, 12 pt bottom; repo groups have a 16.5 pt gap including a subtle separator.

### Icons

Use image-only native buttons with monochrome, regular-weight SF Symbols: 12 pt for toolbar actions, 10 pt frames for disclosures. Add repo and new tab share `plus`; close uses `xmark`, pin `sidebar.left`, overview `square.grid.2x2`. Find uses `chevron.left`, `chevron.right`, and `xmark` with the same image configuration and circular feedback. Native symbol metrics own alignment; do not use text baselines or characters as icons. Prototype SVGs use a 12 px box and 1.5 px stroke.

### Button feedback

Add repo, pin, new tab, close tab, and overview use neutral circular hover and stronger pressed fills, without movement or scaling. Diameter is 28 pt, capped by button bounds; close uses 20 pt. Pin stays highlighted while pinned. Close appears on tab hover or keyboard focus, including focus on the close button. Reserve 38 pt right of the title to prevent text movement. Native tooltips include existing shortcuts; buttons expose accessibility labels. AppKit owns release-to-activate, drag-out cancellation, and keyboard focus.

### Sidebar interaction

Startup shows the workspace chip. Hover for 150 ms to expand the catalog; click or keyboard activation opens immediately. A faster crossing, held mouse button, drag start, window blur, or Escape cancels pending entry. Crossing between header and list keeps it open.

Workspace selection keeps the catalog open and restores that workspace's tabs and focused pane. After entering the list, hovering the chip for 150 ms folds it; a quick crossing does not. Pointer exit closes the transient panel after 250 ms unless re-entry or keyboard focus inside protects it. Escape, outside click, or keyboard focus leaving dismisses it. Pinned panels survive these dismissals and reserve terminal space; unpinning returns to the chip. Cmd-B toggles pinned and chip states.

Repo headings collapse rows without changing selection or closing terminals. Collapse state and session marks last for the app run. Green means a workspace owns a tab; dim means it does not. The chip has no session dot; Home has no collapsible heading.

Hovered and selected rows share the neutral fill and appearance-adaptive text; hover does not select. Keep tracking areas alive during AppKit visible-rectangle updates to preserve paired enter/exit events across geometry changes.

In either open catalog state, holding Command numbers the first nine visible workspace rows in display order, excluding collapsed rows. Cmd-1 through Cmd-9 selects them without dismissing the panel; unassigned numbers are consumed. With the catalog closed, Ghostty tab-number bindings apply.

### Appearance and typography

The View menu offers Follow System (default), Light, and Dark. Overrides last for the current run only. The prototype exposes the same choices outside its simulated window.

| Role | Dark | Light |
| --- | --- | --- |
| Window and terminal background | `#161719` | `#f5f5f7` |
| Primary terminal text | `#e6edf3` | `#24292f` |
| Cursor | Block `#2f81f7` | Block `#0969da` |
| Terminal selection | `#e6edf3` with dark text | `#b6e3ff` with dark text |
| Chrome primary text | `#f1f1f2` | `#24292f` |
| Active tab title | `#e6e7e9` | `#292d33` |
| Secondary text and navigation symbols | `#9c9ea3` | `#656970` |
| Header action symbols and shortcut hints | `#c3c4c7` | `#555960` |
| Selected and hovered workspace | White at 7.1% | Black at 3.1% |
| Shortcut hint fill | White at 5.1% | Black at 2.4% |
| Live session mark | `#65c888` | `#248247` |
| Inactive session mark | `#85878d` | `#85878d` |

Native material supplies blur and accessibility fallback. An appearance-adaptive neutral tint, diagonal highlight, and thin white edge keep chips and active tabs light and expanded catalog and quota panels fuller. Light glass has a brighter edge; dark glass a restrained highlight. Chrome colors follow effective appearance independently of window and terminal backgrounds. Quota thresholds and system warning colors do not vary by appearance.

Chrome uses the system font: 12 pt for chips, tabs, repo headings, and quota values; 13 pt for workspace rows and semibold provider headings. Percentages and shortcut hints use tabular digits. Terminals default to 13 pt Fira Code with explicit Noto Sans Mono CJK SC codepoint fallback. Ghostty shell integration retains title and prompt marking and keeps the block cursor at the prompt. Both ANSI palettes are compiled in `habits.rs`.

Every surface, including split and zoomed panes, has 8 pt base padding on all sides; Ghostty balances whole-cell grid remainders. The quota summary starts at the terminal area's left edge with the same 8 pt text inset.

Appearance changes update window background, native material, semantic text colors, Ghostty configuration, and all surfaces, including hidden and zoomed panes. Preserve shells, scrollback, split trees, selection ownership, and font family, size (including user adjustments), weight, spacing, and CJK fallback. New surfaces inherit effective appearance. Check Latin, CJK, bold, ANSI colors, cursor, and selection in both modes; light glass must not retain hard-coded light text.

### Quota

The status line sits under terminals only. Sources are local Claude `~/.claude/rate-limits.json` (or `$CLAUDE_CONFIG_DIR/rate-limits.json`) and Codex `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl` (default `~/.codex`). Combe never reads credentials, accesses Keychain, queries usage endpoints, starts a CLI, or installs hooks. Claude's externally configured statusline writer supplies `rate_limits`; its `updated_at` epoch timestamp is ignored. Read the whole file up to 64 KiB. Larger, missing, or invalid snapshots make that provider unavailable.

Codex discovery scans session metadata in the background, including old dates for resumed sessions. Read at most eight recently modified files, at most the last 1 MiB each; skip incomplete lines and select the newest timestamped `event_msg` / `token_count` snapshot for the `codex` bucket or legacy records without a limit ID. Reuse cached snapshots for unchanged files. Classify windows by duration, ignoring primary/secondary position; omit unsupported durations. Events outside the read budget stay unavailable.

The chip lists available providers and remaining percent of each tighter 5h/7d window. Hover or activation expands the same chip upward to show windows, including Claude Fable when supplied. Use provider headings without a Window / Remaining / Resets in header. Rows show window, remaining percent, and reset interval with accessible labels. Details share the transient sidebar's 150 ms entry, 250 ms exit, drag cancellation, keyboard-focus protection, and Escape behavior.

Used quota at 60 percent is orange, at 80 percent red. In the summary, color only the affected percentage; names, separators, and other providers keep normal text color. Hide unavailable providers; with none, remove the chip and reserved status height.

Snapshots establish no account identity or current-login claim. Other devices or account switches can make values stale; passing a reset time never produces a fresh percentage. Persist no quota data. Read once at startup, every 15 minutes while focused, and on activation only after five minutes since the last check. Both intervals are compiled in `habits.rs`.

## Terminals

All tabs share one content view. Only the selected worktree's active tab is visible; other surfaces receive `ghostty_surface_set_occlusion(false)` and stop drawing. One active-tab pointer per worktree plus the current worktree keeps sidebar and tab selection consistent. Each tab remembers its focused pane; closing a background pane preserves input focus.

A tab follows its focused pane's terminal title, falling back to the worktree name. Ghostty shell integration supplies the running command (for example, `claude`) or shortened prompt path. `GHOSTTY_ACTION_SET_TITLE` reaches the runtime action callback, which identifies the pane through `ghostty_surface_userdata`. There is no process-name API in `ghostty.h`; without shell integration, keep the worktree name.

Splitting reparents the focused surface into a new `NSSplitView` with a fresh sibling. The sibling uses the last `GHOSTTY_ACTION_PWD` directory, falling back to the workspace path. Reported directories do not change workspace selection; new tabs use the selected worktree path. Closing removes the leaf and collapses any single-child pane into its parent. `close_surface_cb` queues the surface for removal on the main queue.

Title updates change labels and accessibility text in place, preserving tab, close, and new-tab controls, hover, and in-progress mouse tracking. The prototype's **Update tab title** control demonstrates this.

### Notifications

Any program can send desktop notification text through OSC 9 or OSC 777; BEL requests attention without completion semantics. Shell integration reports duration and exit status for completed commands; those running at least five seconds qualify. Shell completion does not identify a persistent program's turn ending or a detached background job's completion. Combe installs no client hooks or plugins and changes no client configuration or Ghostty source.

Events from the focused pane in the active key window are quiet. Other qualifying events mark their source pane with a six-point blue tab dot. The workspace session dot stays blue while any pane there needs attention; repo headings also show a dot for pending child workspaces, including collapsed groups. Light/dark colors are `#0969da` / `#58a6ff`; tab titles and repo headings reserve 48 pt on the right for the dot and adjacent control. Accessibility exposes attention state. Activating a tab acknowledges all its panes, including hidden ones, and preserves remembered focus. Focusing a source pane in the active window acknowledges that pane. Other tabs remain pending.

macOS owns permission, banners, sound, and Notification Center. Request permission on the first qualifying event; denial preserves application marks. Each pane independently retains at most one pending notification, combining bursts for 250 ms and delivering at most once every five seconds. Explicit text takes precedence over command completion and BEL within a burst. Ghostty's OSC throttle can discard concurrent events before Combe's callback; application marks cannot recover them.

Notification clicks select the source workspace and tab, focusing the source pane only if visible, without restoring zoom or changing split layout. Each pane gets a fresh UUID so old notifications cannot open a different pane after restart. Closing removes its pending and delivered notifications; moving preserves identity and attention. State lives in memory; marks derive from it without rebuilding catalogs or terminal views per event.

The prototype's **Notify background tab** control demonstrates the mark and acknowledgement states. Verify the native application with all three channels, foreground suppression, denied permission, independent panes, a single-pane or zoomed tab skipping the locating cue, notification click routing that does not restore zoom, moved or closed panes, zoom, and light/dark appearances. Program text in notifications remains supplied by the terminal program; Combe's fallback wording is **Terminal needs attention**, **Command finished**, **Command succeeded**, or **Command failed**.

Entering a tab with a new pending notification cues each visible source pane once, only if more than one pane is currently visible. Single-pane and single-visible-pane zoomed tabs skip it. The cue is a 1 pt inner attention-colored border: 65 percent opacity at 180 ms, fading out over the remainder of 1.2 seconds. It preserves layout, focus, acknowledgement, and marks, passes mouse input through, and never repeats without a new notification. System-notification clicks use the same rule. Reduce Motion uses a brief static border; otherwise Core Animation owns the finite animation, without polling or repeating timers.

### Tab Overview

The tab bar's right-hand grid button, Cmd-Shift-Backslash, and View menu toggle the current workspace's overview. Cards show titles and each whole tab's last rendered image, including visible splits or zoom. Images are static, may lag behind background output, stay in memory, and are released when the overview closes. A surface without a frame shows its terminal background.

Click enters a card's tab. Return enters the previously focused tab or a card reached by normal keyboard focus. Escape returns without switching tabs and restores the prior responder if available. No arrow navigation, reordering, closing, or cross-workspace aggregation. Other unmodified keys never reach terminals; Command shortcuts dismiss the overview before normal application routing.

Use a 180 ms ease-in-out crossfade, immediate under Reduce Motion. Repeated toggles replace the transition; focus and selection update immediately. Cover terminals without reparenting or resizing them; preserve sessions, split geometry, zoom, sidebar, and tab-bar positions. The opaque background matches the window and redraws on appearance changes. Cards use selected-row neutral fill, 8 pt padding, 24 pt grid gaps and outer padding, and a 20 pt title line. The grid scrolls vertically as needed and follows both appearances.

### Zoom and pane movement

Zoom moves the focused surface above its hidden split tree, leaving a placeholder for restoration without recreating terminals. Zoom is retained per tab; toggling restores it. Closing a pane or adding a split restores the tree first. Single-surface tabs are unchanged.

Moving a pane to a new tab restores zoom first, reparents the surface without recreating its terminal, collapses the old split, and focuses the new tab in the same workspace. Single-pane tabs are unchanged. There is no inverse tab-to-split merge.

### Input and search

Command-modified keys belong to the app and never reach the PTY; Control sequences always reach the PTY. Selection release copies to the system clipboard (`copy-on-select`); Cmd-C also copies. Terminal clipboard reads default to denied. Ordinary paste remains available; unsafe paste is denied with the system alert sound. Cmd-click sends `GHOSTTY_ACTION_OPEN_URL`; only `http`, `https`, and `mailto` open in the default handler.

libghostty owns search. `GHOSTTY_ACTION_START_SEARCH` adds the surface's top-right find bar; edits send `search:<needle>` through `ghostty_surface_binding_action`. `SEARCH_TOTAL` / `SEARCH_SELECTED` feed the counter; `END_SEARCH` removes the bar and restores surface focus.

### Shortcuts

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
| Cmd-Shift-T | Move the focused pane into its own tab |
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

`argv[0]` selects between two entry points in one binary: `<something>.app/Contents/MacOS/Combe` without arguments opens the GUI (Dock, Finder, `open -a`); every other invocation is CLI, with no arguments showing help. Use `argv[0]` because `current_exe()` resolves the `make install` symlink on `PATH` back into the bundle.

`list`, `add`, `remove`, and `cleanup` operate on `state.json` without AppKit. `cleanup` removes registered paths whose directories no longer exist.

GUI and CLI stop state edits on read failure. Saves replace the file atomically, preserving old contents on write failure.

### Hop

A non-subcommand first argument is a **hop**: `combe .`, `combe ..`, `combe ~/git/combe`, `combe /abs/path`. Subcommands win; a directory named `list` is unreachable by that name. Expand leading `~`, canonicalize, and require a directory. Files, missing paths, or unreadable state report to stderr and exit nonzero without raising Combe. An uncanonicalizable name that does not look like a path gets the unknown-command error.

Open one `combe:` URL through `NSWorkspace`, using the canonical directory as its path. The shared URL-scheme handler supports both running and cold-launched Combe. Do not use `open -a Combe <dir>`: its `public.directory` handler registers the folder as a repo, which a hop must never do.

The window resolves the hop against the displayed catalog's registered rows; the CLI does not. The longest ancestor-or-equal path wins. Exclude injected Home from prefix matching: merely being under `$HOME` is not a hit.

- Hit: select that workspace; open a new tab at its root, ignoring the requested subdirectory. Do not write `state.json`.
- Miss: register nothing; select Home and open a new tab owned by `$HOME` with the requested cwd. Only this case separates tab workspace identity from cwd.

Always open a new tab; never reuse one, inject `cd` into a live surface, or infer a main worktree from an unregistered path. `combe ~` opens a plain Home tab unless `$HOME` is registered, when normal matching applies.

Every surface sets `COMBE=1` through Ghostty's `env` override after `TERM_PROGRAM=ghostty`. That mark makes hops print a message and exit without opening a URL, and refuses a second GUI launched from a Combe shell. Catalog subcommands still run.

## System entry points

`Info.plist` registers these LaunchServices entry points. macOS has no default-terminal role equivalent to browser or mail:

- Document types. `public.directory` opens the folder as a workspace. `public.unix-executable` and `.command`, `.sh`, `.zsh`, `.bash` open a tab in the file's parent. Both claim `Alternate` rank, so Combe appears in Finder's Open With and Get Info without displacing the current handler.
- URL schemes. `combe:` for the CLI hop, plus `ssh:` and `x-man-page:`. `telnet:` is not claimed, because macOS ships no `telnet` binary and the handler would always fail.
- One service, "New Combe Tab Here", for the Finder right-click menu and any application that sends a file path.

`crates/combe/src/entry.rs` resolves paths and URLs into workspaces, hops, or shell input. It whitelists characters in hosts, users, man pages, and sections, quotes interpolated values, and separates options from operands with `--`. It has no AppKit dependency; `cargo test -p combe` covers injection cases and hop resolution. Every `combe:` URL is re-canonicalized in the window process and rejected unless it names a directory.

Reject executable paths containing ASCII control characters before constructing shell input; line editors interpret them before shell quoting. Command execution requires an alert showing the exact line. Only confirmation opens the tab and writes that line as `initial_input`, leaving an ordinary long-lived workspace session. Never use it as the surface command.

## Catalog

Use the terms in [CONTEXT.md](../CONTEXT.md). Folder workspaces are single rows without a branch.

1. Load `state.json`
2. For each repo, `git -C <path> worktree list --porcelain`
3. If the registered path is not a directory, skip it and keep the rest. Leave it in `state.json`.
4. If `git rev-parse --git-dir` fails, emit one folder row
5. Skip worktrees Git reports as `prunable`; their directory is gone
6. If `$HOME` is a directory and no row already owns that path, prepend a folder workspace labelled `home` as a single catalog row. Do not write it to `state.json`. Do not scan `$HOME` for git worktrees unless the user registered it.

Identity is the resolved worktree path. `list`, `add`, `remove`, and `cleanup` still operate only on registered repos.

Startup loads the catalog synchronously. Later refreshes run Git in the background and apply results on the main queue: on activation, after adding repos or opening a tab, and on sidebar empty-area clicks. A refresh requested during a scan runs once more afterwards. Tab selection and session marks reuse the catalog without Git.

`crates/combe-catalog` has no AppKit dependency. `cargo test -p combe-catalog` is the fast loop.

## Layout

```text
crates/ghostty-sys     zig build + bindgen over vendor/ghostty
crates/combe-catalog   state.json, git porcelain, folder fallback, Home workspace
crates/combe
  main.rs              CLI or GUI, GHOSTTY_RESOURCES_DIR, NSApplication
  cli.rs               list, add, remove, cleanup, hop
  entry.rs             external file, URL, and service requests, hop resolution
  ghostty.rs           ghostty_init, app lifecycle, runtime callbacks
  notification.rs      terminal attention, bounded delivery, and native notification callbacks
  surface.rs           one NSView per libghostty surface
  split.rs             NSSplitView tree, zoom, and neighboring pane selection
  tabs.rs              tabs keyed by worktree, active tab per worktree
  tab_bar.rs           tab controls, titles, selection, and scrolling
  overview.rs          static tab images and overview grid
  window.rs            window lifecycle, workspace sessions, focus, and overall layout
  menu.rs              native menu items, shortcuts, and action targets
  chrome_view.rs       shared clickable views, native glass, and symbols
  sidebar.rs           catalog adapter
  sidebar_panel.rs     catalog refresh, sidebar controls, interaction state, and geometry
  quota.rs             Claude and Codex subscription windows
  quota_panel.rs       quota cache, refresh scheduling, chip, and expanding details
  habits.rs            every preference, compiled in
```

The quota panel owns its cached usage, polling state, and views. The window mounts it, supplies callbacks for current window activity and layout, and uses its height when positioning terminal content. Shared chrome views dispatch clicks through callbacks without owning workspace or quota state.

The sidebar panel owns its catalog snapshot, refresh scheduling, collapsed groups, hover timer, keyboard navigation, and views. The window supplies the current workspace and live session marks, handles workspace activation, and reserves space when the sidebar is pinned. The tab bar receives tab IDs, labels, and the selected ID; it emits create, select, and close actions. The window retains tab and terminal lifecycle, occlusion, close confirmation, and focus coordination. Components do not access the window's private state.

## GUI acceptance

Preserve these capabilities when changing chrome. Prototype states demonstrate appearance and interaction intent; native verification proves terminal and system behavior.

| Capability | Required verification |
| --- | --- |
| Workspace sessions | Switch away and back; tab, split, focused pane, title, shell PID and output survive; repo collapse does not close sessions |
| Tabs | Create, select, close and navigate by key; keep workspace ownership; closing the last workspace tab selects another live workspace, or closes the window when none remain; the tab strip does not move vertically |
| Splits and zoom | Split right and down, nest and resize, move focus, zoom and restore, close a leaf and promote its sibling; a new sibling inherits the focused pane's reported working directory; preserve PTYs and focus |
| Catalog | Add multiple repos with the native directory picker; include folder workspaces; Home is a single `home` row without adding a repo; adding `$HOME` as a repo replaces the built-in row; refresh on activation without blocking input |
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
- Quota is a compiled instrument, not a surface. Local snapshots of tools the owner already runs may show remaining percent. Claude and Codex are the closed provider list. It may not grow another panel, another provider, a setting, or a network.
