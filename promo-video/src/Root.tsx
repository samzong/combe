import { Composition, cancelRender, continueRender, delayRender, staticFile } from 'remotion';
import { CombeFilm } from './CombeFilm';
import { type FilmProps, readManifest } from './manifest';

const fontReady = delayRender('Load Stack Sans Text');
new FontFace('Stack Sans Text', `url("${staticFile('fonts/stack-sans/StackSansText.ttf')}")`, { weight: '300 700' }).load().then((font) => {
  document.fonts.add(font);
  continueRender(fontReady);
}).catch(cancelRender);

export const Root = () => (
  <Composition
    id="CombeFilm"
    component={CombeFilm}
    durationInFrames={2160}
    fps={30}
    width={1920}
    height={1080}
    defaultProps={{ slogan: 'A worktree-aware terminal for Macs.', manifest: null } satisfies FilmProps}
    calculateMetadata={async ({ props }) => {
      const response = await fetch(staticFile('captures/manifest.json'));
      if (!response.ok) throw new Error('Missing captures/manifest.json');
      return { props: { ...props, manifest: readManifest(await response.json()) } };
    }}
  />
);
