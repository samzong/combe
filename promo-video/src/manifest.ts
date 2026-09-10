export const scenes = [
  { id: 'workspace', start: 8, duration: 6 },
  { id: 'sidebar', start: 14, duration: 5 },
  { id: 'worktree', start: 19, duration: 8 },
  { id: 'splits', start: 27, duration: 14 },
  { id: 'resize', start: 41, duration: 4 },
  { id: 'tabs', start: 45, duration: 5 },
  { id: 'overview', start: 50, duration: 5 },
  { id: 'appearance', start: 55, duration: 4 },
] as const;

type ShotId = typeof scenes[number]['id'];
export type Shot = { src: string; shortcuts?: { at: number; keys: string }[] };
export type Manifest = { hero: string; shots: Record<ShotId, Shot>; recap: string[] };
export type FilmProps = { slogan: string; manifest: Manifest | null };

export const readManifest = (value: unknown): Manifest => {
  const manifest = value as Manifest;
  if (!manifest || typeof manifest.hero !== 'string' || !manifest.hero || !manifest.shots || !Array.isArray(manifest.recap) || manifest.recap.length < 8 || manifest.recap.some((src) => typeof src !== 'string' || !src)) {
    throw new Error('Manifest requires hero, all eight shots, and at least eight recap images');
  }
  for (const scene of scenes) {
    const shot = manifest.shots[scene.id];
    if (!shot || typeof shot.src !== 'string' || !shot.src) throw new Error(`Missing shot: ${scene.id}`);
    if (shot.shortcuts !== undefined && (!Array.isArray(shot.shortcuts) || shot.shortcuts.some((key) => !Number.isFinite(key.at) || key.at < 0 || key.at >= scene.duration || typeof key.keys !== 'string' || !key.keys))) {
      throw new Error(`Invalid shortcut timing: ${scene.id}`);
    }
  }
  return manifest;
};
