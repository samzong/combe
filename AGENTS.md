# Combe

Read [CONTEXT.md](CONTEXT.md) and [DESIGN.md](DESIGN.md) before changing behavior. Code wins if they disagree; update the docs in the same change.

## Scope

A curated worktree list plus real Ghostty terminals. Do not add agents, an editor, a browser, SSH, a settings GUI, a theme system, a command palette, or cloud sync.

## Stack

- Target: macOS on Apple Silicon only. The build asserts `aarch64-apple-darwin`.
- Chrome: AppKit through `objc2`. No GPUI, winit, wgpu, WebView, OpenGL, or Electron.
- Terminal: every leaf is a complete `libghostty` surface. Never write a VT parser, glyph atlas, or renderer.
- API surface: whatever `vendor/ghostty/include/ghostty.h` declares. Do not invent bindings.
- Catalog: user's `git` binary, `worktree list --porcelain`, Git 2.25 floor.
- Persist only repo paths and pins. Every other preference is a constant in `crates/combe/src/habits.rs`.

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
