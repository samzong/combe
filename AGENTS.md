# Combe

Read [CONTEXT.md](CONTEXT.md), [DESIGN.md](docs/DESIGN.md), and [the interactive prototype](docs/design.html) before changing behavior. Approved design decisions define the target; live code and runtime establish current behavior. Resolve mismatches explicitly. When a request is ambiguous, converge on those sources; do not invent product semantics.

Before changing GUI code, update the affected prototype tokens, components, and interaction states, then the design contract. Implement that contract in AppKit and verify the real window in light and dark appearances. Keep the HTML and design document in the same change; neither may be replaced by a scratch mockup. Use user-provided screenshots to clarify interaction and visual intent.

## Scope

A curated worktree list plus real Ghostty terminals. Do not add agents, an editor, a browser, an SSH client or any other remote transport, a settings GUI, a theme system, a command palette, or cloud sync.

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

Prefer existing Combe code and native AppKit capabilities. Inspect Combe's existing implementation first when adding or changing a feature.

Before adding or expanding a module, identify its business responsibility, state ownership, and dependency direction. Keep related state and behavior together; separate responsibilities with independent reasons to change. File length triggers inspection, not mandatory splitting. A split must reduce the context needed to understand and modify a feature, without merely moving code, exposing internal state, or adding unnecessary abstractions.
