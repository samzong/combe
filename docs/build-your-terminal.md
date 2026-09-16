# Build your terminal

Clone, `make`, edit [habits.rs](../crates/combe/src/habits.rs). Product behavior is [DESIGN.md](DESIGN.md). Words are [CONTEXT.md](../CONTEXT.md).

## Run from source

```sh
git clone https://github.com/samzong/combe.git
cd combe
make run
```

The first `make` that needs `vendor/ghostty/build.zig` runs `git submodule update --init` and checks Zig against `vendor/ghostty/build.zig.zon`. Missing `zig` with Homebrew on `PATH` runs `brew install zig`. Later `make` calls skip that recipe.

`make build` is a debug binary. `make install` copies `build/Combe.app` to `/Applications` and links `combe` onto `PATH`. About reads the version from the running bundle; `make app` stamps it from `Cargo.toml`.

## Habits

Preferences are constants in `habits.rs`. Rebuild after edits. No config file.

| Constant | What it sets |
| --- | --- |
| `SHELL` | Surface command. Default `/bin/zsh -l` |
| `FONT_FAMILY`, `FONT_FAMILY_CJK`, `FONT_SIZE` | Terminal fonts |
| `BACKGROUND`, `FOREGROUND`, `PALETTE`, `LIGHT_PALETTE` | Dark and light terminal colors |
| `PADDING_X`, `PADDING_Y` | Terminal padding |
| `CURSOR_STYLE`, `CURSOR_BLINK`, `COPY_ON_SELECT` | Cursor and copy-on-select |
| `SCROLLBACK_LINES` | Scrollback |
| `WINDOW_WIDTH`, `WINDOW_HEIGHT` | First window size |
| `SIDEBAR_WIDTH`, `SIDEBAR_VISIBLE` | Default sidebar |
| `ALLOW_OSC52_READ` | Terminal clipboard read. Default off |
| `CHROME_*`, `GLASS_*` | Chrome colors and glass |

## A second app next to brew Combe

A source build uses the same bundle id `com.samzong.combe` and `~/Library/Application Support/combe/state.json` as `brew install samzong/tap/combe`. They share registered repos.

A separate product name needs all three:

- `packaging/Info.plist`: `CFBundleName`, `CFBundleIdentifier`
- `scripts/package_app.sh`: `APP_NAME`, `BUNDLE_ID`
- `crates/combe-catalog/src/store.rs`: the `combe` directory under `dirs::data_dir()`

`make install` overwrites `/Applications/Combe.app` and the `combe` link in Homebrew's bin, the same paths `brew install` owns, until `APP_NAME` changes.

```sh
combe add ~/git/example
combe list
```

## Release

Version lives in `[workspace.package]` of `Cargo.toml` and `packaging/Info.plist`. `cargo-release` bumps both, commits, tags `vX.Y.Z`, and pushes. The tag workflow packages the app and publishes the GitHub Release. The Homebrew tap watches that release.

Commit or stash everything first. `cargo-release` refuses a dirty tree, including staged files.

```sh
cargo install cargo-release
make release-patch              # dry-run
make release-patch EXECUTE=1    # bump, commit, tag, push
```

`release-minor` and `release-major` work the same way. The commit subject must be `chore(release): bump to vX.Y.Z`; the tag workflow refuses anything else.

If `Cargo.toml` is behind already-published tags, a patch bump will collide. Skip to the next free version:

```sh
cargo release 0.2.7
cargo release 0.2.7 --execute
```

Users pick up the cask with `brew upgrade --cask samzong/tap/combe` after the tap updates.
