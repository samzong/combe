# Claude Code status line

Copyable status line for Claude Code. Shows the model, working directory, Git branch and dirty state, and 5-hour/7-day **used** percentages. Saves the supplied `rate_limits` object with an `updated_at` timestamp to a local file that Combe reads.

Requires Bash, Git, and `jq` (`brew install jq` if missing).

## Install

Run from the Combe repository root:

```sh
claude_dir="${CLAUDE_CONFIG_DIR:-$HOME/.claude}"
mkdir -p "$claude_dir"
cp -i examples/claude/statusline.sh "$claude_dir/statusline.sh"
```

Merge this field into that directory's `settings.json`, preserving other settings. It replaces any existing status line command:

```json
{
  "statusLine": {
    "type": "command",
    "command": "bash \"${CLAUDE_CONFIG_DIR:-$HOME/.claude}/statusline.sh\""
  }
}
```

Claude Code supplies JSON on stdin and renders stdout; see the [official status line documentation](https://code.claude.com/docs/en/statusline).

## Snapshot behavior

The script atomically replaces `$CLAUDE_CONFIG_DIR/rate-limits.json`, defaulting to `~/.claude/rate-limits.json`. The directory must already exist. Missing quota data or a failed write leaves the previous snapshot intact. Values can become stale; the script does not query a usage API. Combe must use the same config directory to read the snapshot.

The script preserves all supplied quota fields but cannot provide a separate Fable limit when Claude Code omits it. Combe displays remaining percentages; this script displays used percentages.
