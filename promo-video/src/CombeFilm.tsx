import { scenes, type Shot, type FilmProps } from './manifest';
import type { CSSProperties, ReactNode } from 'react';
import { Audio, Video } from '@remotion/media';
import { AbsoluteFill, CanvasImage, Easing, Freeze, Sequence, interpolate, staticFile, useCurrentFrame } from 'remotion';

const fps = 30;
const ink = '#0A0A09';
const stage = '#202224';
const paper = '#F2F0E9';
const lime = '#D8FF38';
const ease = { extrapolateLeft: 'clamp', extrapolateRight: 'clamp', easing: Easing.bezier(0.16, 1, 0.3, 1) } as const;

const media: CSSProperties = { width: '100%', height: '100%', objectFit: 'contain' };
type Focus = readonly [number, number, number, number, number];
const guidance: Record<typeof scenes[number]['id'], readonly Focus[]> = {
  workspace: [[0, 1, 3, 25, 49], [2.5, 1, 3, 25, 49], [2.8, 1, 40, 25, 18]],
  sidebar: [[0, 1, 3, 25, 49]],
  worktree: [[0, 27, 5, 72, 18], [3.3, 27, 5, 72, 18], [4.2, 1, 38, 25, 19], [6.8, 1, 38, 25, 19]],
  splits: [[0, 1, 5, 98, 92], [1.6, 1, 5, 98, 92], [2.8, 51, 5, 48, 92], [7, 51, 5, 48, 92], [8.2, 51, 48, 48, 49], [12.5, 51, 48, 48, 49]],
  resize: [[0, 45, 5, 12, 92], [0.8, 45, 5, 12, 92], [1.9, 33, 5, 12, 92], [2.5, 39, 42, 60, 15], [3.2, 39, 49, 60, 15]],
  tabs: [[0, 27, 0, 71, 9], [3.8, 27, 0, 71, 9]],
  overview: [[0, 1, 4, 98, 83], [1.5, 1, 4, 98, 83], [2.7, 1, 4, 25, 45], [3.6, 1, 4, 25, 45]],
  appearance: [[0, 0, 0, 100, 100]],
};

const Dissolve = ({ children, frames = 12 }: { children: ReactNode; frames?: number }) => {
  const frame = useCurrentFrame();
  return <AbsoluteFill style={{ background: stage, opacity: interpolate(frame, [0, frames], [0, 1], { extrapolateRight: 'clamp' }) }}>{children}</AbsoluteFill>;
};

const Window = ({ src, video = false, entrance = false, focus, duration = 1, focusRange }: { src: string; video?: boolean; entrance?: boolean; focus?: readonly Focus[]; duration?: number; focusRange?: readonly [number, number] }) => {
  const frame = useCurrentFrame();
  const start = focusRange ? focusRange[0] * fps : 12;
  const end = focusRange ? focusRange[1] * fps : duration - 1;
  const emphasis = focus ? interpolate(frame, [start, start + 12, end - 12, end], [0, 1, 1, 0], ease) : 0;
  const stops = focus?.map((point) => point[0] * fps);
  const rect = focus?.[0].slice(1).map((_, index) => focus.length === 1 ? focus[0][index + 1] : interpolate(frame, stops!, focus.map((point) => point[index + 1]), ease));
  return (
    <AbsoluteFill style={{ alignItems: 'center', justifyContent: 'center', opacity: entrance ? interpolate(frame, [0, 20], [0, 1], ease) : 1 }}>
      <div style={{ position: 'relative', width: 840 * 2400 / 1560, height: 840, borderRadius: 32, overflow: 'hidden', boxShadow: '0 28px 45px #0008', transform: `scale(${entrance ? interpolate(frame, [0, 35], [0.96, 1], ease) : 1 + emphasis * 0.08})` }}>
        {video ? <Video src={staticFile(src)} muted style={media} /> : <CanvasImage src={staticFile(src)} style={media} />}
        {rect && <div style={{ position: 'absolute', left: `${rect[0]}%`, top: `${rect[1]}%`, width: `${rect[2]}%`, height: `${rect[3]}%`, borderRadius: 14, boxShadow: '0 0 0 2000px #0009', outline: '1px solid #D8FF3838', opacity: emphasis }} />}
      </div>
    </AbsoluteFill>
  );
};

const Hero = ({ src }: { src: string }) => {
  const frame = useCurrentFrame();
  return (
    <AbsoluteFill style={{ opacity: interpolate(frame, [78, 118], [1, 0], ease), filter: `blur(${interpolate(frame, [78, 116], [0, 24], ease)}px)` }}>
      <Window src={src} entrance />
    </AbsoluteFill>
  );
};

const Brand = ({ slogan, end = false }: { slogan: string; end?: boolean }) => {
  const frame = useCurrentFrame();
  return (
    <AbsoluteFill style={{ background: stage, justifyContent: 'center', alignItems: 'center', opacity: interpolate(frame, [0, 12], [0, 1], ease) }}>
      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', translate: `0 ${interpolate(frame, [0, 35], [18, 0], ease)}px` }}>
        <CanvasImage src={staticFile('brand/combe.svg')} style={{ width: 180, height: 180, borderRadius: 34, objectFit: 'contain' }} />
        <div style={{ color: paper, fontSize: 104, fontWeight: 400, lineHeight: 1.05, letterSpacing: '-4px', marginTop: 24 }}>combe</div>
        {slogan && <div style={{ color: '#C3C5C2', fontSize: 32, marginTop: 24, maxWidth: 1100, textAlign: 'center' }}>{slogan}</div>}
        {end && <div style={{ display: 'flex', alignItems: 'center', gap: 32, marginTop: 60 }}>
          <CanvasImage src={staticFile('brand/github-qr.png')} style={{ width: 150, height: 150, background: '#fff', padding: 10 }} />
          <div style={{ color: paper, fontSize: 30, letterSpacing: '-0.6px' }}>github.com/samzong/combe</div>
        </div>}
      </div>
    </AbsoluteFill>
  );
};

const Shortcut = ({ keys }: { keys: string }) => {
  const frame = useCurrentFrame();
  return <div style={{ position: 'absolute', right: 76, bottom: 48, background: ink, border: `1px solid ${lime}88`, color: lime, borderRadius: 12, padding: '15px 25px', fontSize: 31, fontWeight: 500, boxShadow: '0 8px 24px #0008', opacity: interpolate(frame, [0, 4, 35, 44], [0, 1, 1, 0], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' }) }}>{keys}</div>;
};

const Demo = ({ shot, scene }: { shot: Shot; scene: typeof scenes[number] }) => <AbsoluteFill>
  <Freeze frame={scene.duration * fps - 1} active={(frame) => frame >= scene.duration * fps}>
    <Window src={shot.src} video focus={guidance[scene.id]} duration={scene.duration * fps} focusRange={scene.id === 'workspace' ? [1.1, 5.9] : scene.id === 'sidebar' ? [0.9, 2.1] : scene.id === 'worktree' ? [1.4, 7.9] : undefined} />
  </Freeze>
  {shot.shortcuts?.map((shortcut, index) => <Sequence key={index} from={Math.round(shortcut.at * fps)} durationInFrames={45}><Shortcut keys={shortcut.keys} /></Sequence>)}
</AbsoluteFill>;

const Statement = () => {
  const frame = useCurrentFrame();
  const phrase = 'A terminal, Only built for me.';
  const length = Math.floor(interpolate(frame, [6, 90], [0, phrase.length], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' }));
  return <AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center', background: stage }}>
    <div style={{ color: paper, fontSize: 58, letterSpacing: '-1.2px', whiteSpace: 'pre' }}>
      {phrase.slice(0, length)}<span style={{ color: lime, opacity: Math.floor(frame / 15) % 2 ? 0 : 1 }}>▍</span>
    </div>
  </AbsoluteFill>;
};

export const CombeFilm = ({ slogan, manifest }: FilmProps) => {
  if (!manifest) throw new Error('Load the capture manifest before previewing or rendering');
  return (
    <AbsoluteFill style={{ background: stage, fontFamily: '"Stack Sans Text", system-ui, sans-serif' }}>
      <Audio src={staticFile('audio/combe-score.wav')} volume={(frame) => interpolate(frame, [0, 24, 2100, 2160], [0, 0.8, 0.8, 0], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' })} />
      <Sequence name="Dark window reveal" durationInFrames={120}><Hero src={manifest.hero} /></Sequence>
      <Sequence name="Combe" from={120} durationInFrames={87}><Brand slogan={slogan} /></Sequence>
      <Sequence name="Worktree-aware terminal" from={195} durationInFrames={57}>
        <Dissolve><Window src={manifest.hero} entrance /></Dissolve>
      </Sequence>
      {scenes.map((scene) => <Sequence key={scene.id} name={scene.id} from={scene.start * fps} durationInFrames={scene.duration * fps + 12} premountFor={12}><Dissolve><Demo shot={manifest.shots[scene.id]} scene={scene} /></Dissolve></Sequence>)}
      {manifest.recap.slice(0, 8).map((src, index) => <Sequence key={index} name={`Recap ${index + 1}`} from={1770 + index * 12} durationInFrames={24}><Dissolve frames={index === 0 ? 12 : 5}><Window src={src} /></Dissolve></Sequence>)}
      <Sequence name="Only built for me" from={1866} durationInFrames={192}><Dissolve><Statement /></Dissolve></Sequence>
      <Sequence name="Open source" from={2046} durationInFrames={114}><Dissolve><Brand slogan={slogan} end /></Dissolve></Sequence>
    </AbsoluteFill>
  );
};
