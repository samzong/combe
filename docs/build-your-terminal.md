# Build your terminal

Clone, `make`, edit [habits.rs](../crates/combe/src/habits.rs). Product behavior is [DESIGN.md](DESIGN.md). Words are [CONTEXT.md](../CONTEXT.md).

## Run from source

```sh
git clone https://github.com/samzong/combe.git
cd combe
make run
```

The first `make` that needs `vendor/ghostty/build.zig` runs `git submodule update --init` and checks Zig against `vendor/ghostty/build.zig.zon`. Missing `zig` with Homebrew on `PATH` runs `brew install zig`. Later `make` calls skip that recipe.

`make build` is a debug binary. `make install` copies `build/Combe.app` to `/Applications` and links `combe` onto `PATH`.

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

`make install` overwrites `/Applications/Combe.app` until `APP_NAME` changes.

```sh
combe add ~/git/example
combe list
```
