import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { scenes } from '../src/manifest.ts';

const root = fileURLToPath(new URL('..', import.meta.url));
const out = resolve(root, 'public/captures');
mkdirSync(out, { recursive: true });
const local = resolve(root, '../.local/promo-video');
mkdirSync(local, { recursive: true });
const source = (name) => resolve(local, `production/raw-${name}.mov`);
const run = (args) => {
  const result = spawnSync('ffmpeg', ['-y', '-v', 'error', ...args], { stdio: 'inherit' });
  if (result.status !== 0) throw new Error(`FFmpeg failed: ${result.status}`);
};
const edits = {
  workspace: [['navigation', 3.8, 7.8, 2], ['navigation', 10.4, 13.4, 1.5], ['pickup', 233.6, 236.7, 2], ['pickup', 238.8, 242.8, 3.5]],
  sidebar: [['navigation', 28.8, 32.8, 3], ['navigation', 34.6, 38.0, 2.3], ['navigation', 41.2, 44.0, 1.7]],
  worktree: [['navigation', 56.4, 60.8, 2], ['navigation', 62.2, 66.3, 2.2], ['pickup', 238.8, 243.8, 5.8]],
  splits: [['pickup', 100.8, 104.8, 2.5], ['pickup', 106.3, 109.8, 2.5], ['pickup', 125, 131, 4], ['pickup', 136.2, 140.2, 2.5], ['pickup', 142, 147, 3], ['pickup', 151, 156, 2.5]],
  resize: [['pickup', 169.3, 175.3, 6]],
  tabs: [['pickup', 177.6, 181.6, 4], ['pickup', 183.9, 187.9, 4]],
  overview: [['pickup', 189, 193, 4], ['pickup', 218.8, 221.8, 3]],
  appearance: [['pickup', 77.5, 81.5, 3], ['pickup', 88, 92, 3]],
};
const shortcuts = {
  sidebar: [{ at: 0.9, keys: '⌘ B' }, { at: 3.8, keys: '⌘ B' }, { at: 6, keys: '⌘ B' }],
  splits: [{ at: 3.4, keys: '⌘ D' }, { at: 9.7, keys: '⌘ ⇧ D' }],
  tabs: [{ at: 1.3, keys: '⌘ T' }],
  overview: [{ at: 1.25, keys: '⌘ ⇧ \\' }, { at: 6, keys: '⌘ B' }],
};
const manifest = { hero: 'captures/hero.png', shots: {}, recap: [] };
for (const [name, segments] of Object.entries(edits)) {
  const pacing = scenes.find((scene) => scene.id === name).duration / segments.reduce((sum, part) => sum + part[3], 0);
  let elapsed = 0;
  let frames = 0;
  for (const segment of segments) {
    elapsed += segment[3] * pacing * 30;
    const next = Math.round(elapsed);
    segment[3] = (next - frames) / 30;
    frames = next;
  }
  for (const shortcut of shortcuts[name] ?? []) shortcut.at *= pacing;
  const inputs = segments.flatMap(([recording, start, end]) => ['-ss', String(start), '-t', String(end - start), '-i', source(recording)]);
  const filters = segments.map(([, start, end, duration], index) => `[${index}:v]setpts=${duration / (end - start)}*(PTS-STARTPTS),fps=30,setsar=1,tpad=stop_mode=clone:stop_duration=1,trim=end_frame=${Math.round(duration * 30)}[v${index}]`);
  filters.push(`${segments.map((_, index) => `[v${index}]`).join('')}concat=n=${segments.length}:v=1:a=0[out]`);
  run([...inputs, '-filter_complex', filters.join(';'), '-map', '[out]', '-an', '-c:v', 'libx264', '-preset', 'veryfast', '-crf', '17', '-pix_fmt', 'yuv420p', '-threads', '4', '-movflags', '+faststart', resolve(out, `${name}.mp4`)]);
  manifest.shots[name] = { src: `captures/${name}.mp4`, ...(shortcuts[name] ? { shortcuts: shortcuts[name] } : {}) };
  const duration = segments.reduce((sum, part) => sum + part[3], 0);
  run(['-ss', String(name === 'appearance' ? 2 : duration - 0.5), '-i', resolve(out, `${name}.mp4`), '-frames:v', '1', resolve(out, `${name}.png`)]);
  manifest.recap.push(`captures/${name}.png`);
  console.log(`${name}: ${duration}s`);
}
run(['-ss', '156', '-i', source('pickup'), '-frames:v', '1', resolve(out, 'hero.png')]);
writeFileSync(resolve(out, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
writeFileSync(resolve(local, 'edit-decisions.json'), JSON.stringify({ sourceSize: [2400, 1560], fps: 30, edits, shortcuts }, null, 2) + '\n');
