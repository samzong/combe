# Combe

Read [CONTEXT.md](CONTEXT.md) and [DESIGN.md](docs/DESIGN.md) before changing behavior. Code wins if they disagree; update the docs in the same change.

## Scope

A curated worktree list plus real Ghostty terminals. Do not add agents, an editor, a browser, SSH, a settings GUI, a theme system, a command palette, or cloud sync.

## Interaction reference

- Orca is the interaction reference for Combe. Its local source is at `/Users/x/git/tmp/orca`.
- When adding or changing a feature, inspect Combe's existing implementation first, then the corresponding Orca code. Use user-provided screenshots to clarify interaction and visual intent.
- When a request is ambiguous, use Orca's actual behavior to offer concrete recommendations; do not invent product semantics.
- Prefer existing Combe code and native AppKit capabilities. Borrow established interactions from Orca instead of designing them from scratch.
- Orca is a reference, not a requirement to copy its implementation. Preserve Combe's lightweight scope and stack; do not import Orca's Electron architecture, extra features, or complexity.

## Stack

- Target: macOS on Apple Silicon only. The build asserts `aarch64-apple-darwin`.
- Chrome: AppKit through `objc2`. No GPUI, winit, wgpu, WebView, OpenGL, or Electron.
- Terminal: every leaf is a complete `libghostty` surface. Never write a VT parser, glyph atlas, or renderer.
- API surface: whatever `vendor/ghostty/include/ghostty.h` declares. Do not invent bindings.
- Catalog: user's `git` binary, `worktree list --porcelain`, Git 2.25 floor.
- Persist only repo paths. Every other preference is a constant in `crates/combe/src/habits.rs`.

## Standard shortcuts

Standard macOS key equivalents live on menu items. `setMainMenu` replaces the system template; missing roles stay dead. Command-modified keys belong to the app and must not reach the PTY. Do not hand-roll a main menu without the items below.

| Key | Action |
| --- | --- |
| Cmd-H | `hide:` |
| Opt-Cmd-H | `hideOtherApplications:` |
| Cmd-M | `performMiniaturize:` |
| Cmd-W | Close the focused pane, or the tab when it is the last pane. Asks first when a foreground process still runs there. The last tab of a workspace ends that session. If another workspace still has tabs, switch to it; if none remain, close the window (`performClose:`). |
| Opt-Cmd-W | Close every titled window. |
| Cmd-Q | `terminate:`. `applicationShouldTerminate:` asks first when `ghostty_app_needs_confirm_quit` is true; `windowShouldClose:` asks the same way for the red button and `performClose:`. |

After a menu change, verify Cmd-H, Opt-Cmd-H, Cmd-M, Cmd-Q, Cmd-W, and Opt-Cmd-W.

## Verify

```sh
make check
make test
make run
```

`make check` is the quality gate: `cargo audit`, `fmt --check`, clippy, and `cargo test --workspace`. `make test` is the test step only.

Chrome changes need a look at the running window, not just a green build.

## Style

No comments in code. Name files after the domain object. English in anything that will be committed.

Before adding or expanding a module, identify its business responsibility, state ownership, and dependency direction. Keep related state and behavior together; separate responsibilities with independent reasons to change. File length triggers inspection, not mandatory splitting. A split must reduce the context needed to understand and modify a feature, without merely moving code, exposing internal state, or adding unnecessary abstractions.
