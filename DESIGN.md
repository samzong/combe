# Combe

Combe is a worktree-aware terminal: a curated repo list on the left, real Ghostty terminals on the right. Click a worktree, land in that directory.

Private tool. macOS on Apple Silicon only. Not a product, not published, no configuration files.

## Product

A heavy terminal user keeps many git checkouts and linked worktrees. The missing piece is a directory of those trees, not another multiplexer, editor, or agent overlay.

The user curates the repo list by hand. Combe reads `git worktree list --porcelain` for each registered repo and shows every tree git already knows.

A worktree is a session, not a shortcut. Each row owns its own set of tabs; selecting a row swaps in that worktree's tabs and returns to the one it was left on. New tabs belong to the selected worktree, and a worktree never drops below one tab.

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

One window. `FullSizeContentView` with a transparent titlebar and hidden title, so a single 40pt row holds the traffic lights, the sidebar toggle, the tab pills, and `+`.

```text
NSWindow
  contentView (NSView)
    NSSplitView (vertical)
      NSView                      sidebar
        header spacer (40pt)
        NSScrollView              repo groups, worktree rows, "+ Add repo"
      NSView                      right pane
        tab bar (40pt)            the selected worktree's tabs, per-tab close, "+"
        content                   one root per tab, every worktree's
          NSView (tab root)
            NSSplitView …         nested panes
              SurfaceView         one libghostty surface
    sidebar toggle button         overlay, trailing edge of the sidebar
```

The sidebar toggle lives on the content view rather than inside the sidebar, so collapsing the sidebar does not take the control away with it. While the sidebar is open the button tracks its trailing edge; collapsed, it falls back to a leading inset — 78pt to clear the traffic lights, 8pt in full screen where there are none.

Chrome geometry is a layout pass, not a one-shot. The content view overrides `layout`, and the split view delegate answers `splitViewDidResizeSubviews:`; between them every window resize, full-screen transition, divider drag, and collapse re-runs the same placement. Sidebar visibility is read back from `isSubviewCollapsed`, never cached, because a drag can collapse the pane without the app asking.

The delegate pins the sidebar's width when the window resizes, clamps a drag to 160–420pt, and widens the divider's hit area. The split view draws no divider while the sidebar is collapsed, and the layout pass invalidates it, because the chrome paints no background and would otherwise leave the old divider standing in the 40pt top bar.

Nothing in the chrome paints its own background. The window is filled with `habits::BACKGROUND`, every chrome view is transparent, and Ghostty is configured with the same color, so the sidebar, the tab bar and the terminal are one surface separated only by a hairline divider. Chrome text uses `habits::FONT_SIZE`, the same number the terminal font is set to.

Every tab of every worktree lives in the same content view. Only the active tab of the selected worktree is unhidden; every surface everywhere else gets `ghostty_surface_set_occlusion(false)` and stops drawing. The selection is one pointer per worktree plus the current worktree, so the sidebar highlight and the tab bar cannot disagree.

A tab is named by the focused pane's terminal title, and falls back to the worktree's own name until a title arrives. Ghostty's shell integration writes the running command while a command runs and a shortened path at the prompt, so a pane running `claude` names its tab `claude`; splitting a tab means the name follows whichever pane holds focus. Titles reach the app through `GHOSTTY_ACTION_SET_TITLE` on the runtime action callback, which resolves back to the pane with `ghostty_surface_userdata`. There is no process-name API in `ghostty.h`, and none is invented: a shell without integration keeps the worktree name.

Splitting reparents the focused surface into a fresh `NSSplitView` and adds a sibling. Closing removes the leaf and, when a pane is left with a single child, collapses that pane into its parent. Ghostty asks for a close through `close_surface_cb`, which queues the surface and drains it on the main queue.

Command-modified keys belong to the app and never reach the PTY. Control sequences always reach the PTY.

| Key | Action |
| --- | --- |
| Cmd-Q | Quit |
| Cmd-T | New tab |
| Cmd-W | Close pane, or the tab when it is the last pane, except the worktree's last tab |
| Cmd-D | Split right |
| Cmd-Shift-D | Split down |
| Cmd-Alt-Left / Right | Previous / next tab |
| Cmd-B | Fold or unfold the sidebar |
| Cmd-C / Cmd-V | Copy / paste through `ghostty_surface_binding_action` |
| Ctrl-Cmd-F | Toggle full screen, the standard `toggleFullScreen:` item in a View menu |

## CLI

One binary, two entry points, separated by `argv[0]`. Running it as `<something>.app/Contents/MacOS/Combe` with no arguments opens the window; that is what the Dock, Finder, and `open -a` do. Every other invocation is the CLI, and no arguments means help.

`argv[0]` rather than `current_exe()`, because `make install` symlinks `combe` onto `PATH` and `current_exe()` resolves that symlink back into the bundle.

`list`, `add`, `remove`, and `cleanup` read and write `state.json` and never touch AppKit. `cleanup` drops registered repo paths and pins whose directory no longer exists, which is the recovery path for a checkout deleted outside Combe.

## Catalog

A **repo** is a path the user added. A **worktree** is one record from `git worktree list --porcelain` for that repo. A **folder workspace** is a registered path that is not a git checkout: one row, no branch.

1. Load `state.json`
2. For each repo, `git -C <path> worktree list --porcelain`
3. If the registered path is not a directory, skip it and keep the rest. Leave it in `state.json`.
4. If `git rev-parse --git-dir` fails, emit one folder row
5. Merge pins

Identity is the resolved worktree path. Pins are a set of those paths.

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
  sidebar.rs           catalog adapter
  habits.rs            every preference, compiled in
```

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
