# Combe film

An editable 72-second Remotion film with fresh native macOS footage, an original instrumental score, and no subtitles or narration.

```sh
pnpm install --frozen-lockfile
pnpm check
pnpm dev
pnpm render:preview --concurrency=4
pnpm render --concurrency=4
```

Run commands in this directory. The preview is 1920×1080; the master is 3840×2160. Both use 30 fps and include audio. Outputs go to `../.local/promo-video/exports/`.

The opening and closing slogan is “A worktree-aware terminal for Macs.” The closing typing sequence reads “A terminal, Only built for me.” The QR code links to https://github.com/samzong/combe.

English titles use the same Stack Sans Text font as the Tokener film, bundled with its OFL license. Terminal windows remain centered with visible margins throughout. Twelve-frame dissolves connect the demo scenes. Gentle push-ins and a moving spotlight guide attention to the sidebar, command output, new panes, dividers, and tabs before returning to the full window. The eight-shot recap takes 3.2 seconds with five-frame dissolves; the typing sequence retains a three-second reading hold.

`src/CombeFilm.tsx` owns framing, transitions, brand scenes, typing, and shortcut overlays. `src/manifest.ts` owns the scene timeline. `public/captures/manifest.json` maps the timeline to captured clips. `scripts/edit.mjs` defines the source ranges and playback timing.

`public/audio/combe-score.wav` is the 48 kHz stereo score used by the film. `scripts/score.py` regenerates the original synthesis using NumPy and FFmpeg, writing a compressed listening copy and measurements to `../.local/promo-video/audio/`. It uses no third-party samples.

To rebuild the edited clips from the local recordings, run `node scripts/edit.mjs`. Raw captures, the filming app, and inspection artifacts live in `../.local/promo-video/production/`; the edit report goes to `../.local/promo-video/edit-decisions.json`. The filming repositories remain in `.local/demo/` to preserve their worktree paths, with a link from `../.local/promo-video/demo`. These local production inputs are not required for rendering the supplied clips. Capture resolution is 2400×1560; native window footage is scaled inside the 4K composition.

Startup waits are shortened through cuts and speed changes. Split resizing, tab selection, Overview, pinning, and appearance changes are captured from the running app. The terminal tools are shown in their real idle interfaces; no AI request was submitted for filming.
