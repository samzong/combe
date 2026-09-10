import { readManifest, scenes } from './manifest.ts';

const shots = Object.fromEntries(scenes.map((scene) => [scene.id, { src: `captures/${scene.id}.mp4`, shortcuts: [{ at: 0, keys: '⌘ B' }] }]));
const valid = { hero: 'captures/hero.png', shots, recap: Array(8).fill('captures/recap.png') };
readManifest(valid);
for (const invalid of [{}, { ...valid, recap: [] }, { ...valid, shots: { ...shots, workspace: { src: 'captures/workspace.mp4', shortcuts: [{ at: 9, keys: '⌘ B' }] } } }]) {
  let rejected = false;
  try { readManifest(invalid); } catch { rejected = true; }
  if (!rejected) throw new Error('Invalid manifest accepted');
}
console.log('Manifest validation passed');
