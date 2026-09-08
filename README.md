<img src="packaging/badge.svg" width="96" height="96" alt="Combe">

# Combe

A worktree-aware terminal: curate the git checkouts you care about, see every worktree git already knows, open a real Ghostty terminal in the one you click.

See [DESIGN.md](DESIGN.md) for the stack and the cut line. [CONTEXT.md](CONTEXT.md) has the words this repo uses.

> Only built for me. macOS on Apple Silicon.

## Build

```sh
git submodule update --init      # vendor/ghostty
brew install zig                 # must match .minimum_zig_version in vendor/ghostty/build.zig.zon
make run
```

`crates/ghostty-sys` builds Ghostty with `zig build` and links `libghostty-internal.a` statically, so the first build is slow and later ones are not. Xcode with the Metal toolchain (`xcodebuild -downloadComponent MetalToolchain`) is required.

```sh
make check                   # audit, fmt, clippy, tests
make test                    # cargo test --workspace
make run                     # build and run build/Combe.app
make app                     # build/Combe.app
make install                 # /Applications/Combe.app and combe on PATH
```

`make install` is ad-hoc signed. It copies the app and symlinks `combe` into `/opt/homebrew/bin`, `/usr/local/bin`, or `~/.local/bin` (override with `BINDIR`). The bundle carries Ghostty's `share/ghostty` resources, so it keeps working after `cargo clean`.

## Release

Push a `vMAJOR.MINOR.PATCH` tag to build an Apple Silicon app and publish it as `Combe-<tag>-macos-arm64.zip` on GitHub Releases:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The tagged commit must contain the release workflow. Actions runs `make check`, builds Ghostty and Combe, and sets the app bundle version from the tag before signing. No release token or signing certificate needs to be configured; publishing uses `GITHUB_TOKEN`.

The app is ad-hoc signed and not notarized. After extracting the ZIP, move `Combe.app` to Applications. macOS may require allowing the first launch in System Settings > Privacy & Security. Releases inherit the repository's private visibility.

## Use

The sidebar lists registered repos and their worktrees. The `+` button beside the sidebar toggle opens a folder picker. Click a repo heading to collapse or expand its worktrees; groups start expanded on launch. Click a worktree row to open or focus a tab whose shell starts there. Combe never scans the disk for projects.

State lives at `~/Library/Application Support/combe/state.json`. Pins are read from it and shown as a star; toggling a pin from the window is not implemented yet.

## CLI

`make install` links the same binary onto `PATH` as `combe`. Run from a terminal it is a CLI. The window opens from the Dock, Finder, or `open -a Combe`.

```sh
combe                        # help
combe list                   # registered repos and their worktrees, pins marked *
combe add <path>...          # register repos
combe remove <path>...       # unregister repos
combe cleanup                # drop registered paths that no longer exist on disk
```

Exit codes: `0` success, `1` a path failed, `2` a usage error.

The window and terminals automatically follow the macOS light or dark appearance, without restarting shells. There is no appearance setting or configuration file. Palettes, font, padding, shell, and window size are compiled into `crates/combe/src/habits.rs`.

| Key | Action |
| --- | --- |
| Cmd-Q | Quit |
| Cmd-T | New tab |
| Cmd-W | Close pane, or the tab when it is the last pane |
| Cmd-D | Split right |
| Cmd-Shift-D | Split down |
| Cmd-Alt-Left / Right | Previous / next tab |
| Cmd-B | Fold or unfold the sidebar |
| Cmd-C / Cmd-V | Copy / paste |
| Ctrl-Cmd-F | Toggle full screen |

Everything else, control sequences included, goes to the terminal.
