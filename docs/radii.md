# Corner radius inventory

This inventory records every Combe-owned rounded surface and path, identifies system-owned shapes, and gives stable IDs for visual decisions. Native values are points; prototype values are unscaled CSS pixels. The four corners currently use the same declared values within each component. Tabs use an18pt glass and hover contour, with a16pt focus path inset2. Other native shapes retain their existing values.

**Current contract: Tab glass and hover are18pt; Tab focus is16pt.** The prototype uses a direct `radius-row=16` token. Other controls keep their existing shapes.

## Inspect

Open [design.html](design.html), enable **Show spacing**, select **Corner radii**, then an R ID. The overlay marks one corner's radius and the component outline; the value applies to all four corners. Switching to a list, Find or quota item reveals that scene. Turn Show spacing off to remove the inspector. Native system dialogs are documented without opening them automatically.

The prototype inspector measures its current uniform circular CSS corners, including percentage circles and oversized-radius limits. It does not inspect native view frames, SF Symbol ink, arbitrary elliptical CSS, or per-corner overrides. Transparent targets may declare a CSS radius without painting a background; their entries say so. R12 measures the shortcut badge's reserved shape even when its opacity is zero.

## Radius ownership

| Scope | Current shape | Boundary |
| --- | --- | --- |
| Tab title | Glass and hover18; focus16, inset2, stroke2 | The Tab instance overrides the shared ClickView default. Frames and click areas are unchanged. |
| List rows and repo headings | Native requested16; prototype `radius-row16` | Other ClickViews retain the16pt default and14pt focus path. |
| Other surfaces | Window34/fullscreen0, sidebar18, quota14, Find12, circles3/9, terminal0 | Sidebar and quota preserve their radii throughout expansion. System controls retain AppKit ownership. |

## Current values

| ID | Surface / path | Native pt | Prototype px | Recommendation |
| --- | --- | --- | --- | --- |
| R01 | Window content clip | 34; 0 in fullscreen | CSS 34px; rendered circular radius 34 | Keep34 / fullscreen0 |
| R02 | Sidebar glass: chip / transient / pinned | 18 | CSS 18px; rendered circular radius 18 | Keep18 in all three states |
| R03 | Workspace trigger background | No independent fill | CSS 18px; rendered circular radius 18 | Keep no independent fill |
| R04 | Repo-heading highlight | Requested16; rx16 / ry15 at height30 | CSS 16px; rendered circular radius 15 | Keep current shape |
| R05 | Workspace-row selected / hover fill | 16 | CSS 16px; rendered circular radius 16 | Keep16 |
| R06 | Active-tab glass | 18 | CSS 18px; rendered circular radius 18 | Keep18 |
| R07 | Tab hover highlight | 18 | CSS 18px; rendered circular radius 18 | Keep18 with active-tab glass |
| R08 | Tab Close target background | No independent fill | CSS 50%; rendered circular radius 11 | Keep no independent fill |
| R09 | New-tab target background | No independent fill | CSS 18px; rendered circular radius 18 | Keep no independent fill |
| R10 | Add / Pin button shape | AppKit; no explicit Combe radius | CSS 50%; rendered circular radius 14 | Keep native button ownership |
| R11 | Session dot | Circle, diameter6 / radius3 | CSS 50%; rendered circular radius 3 | Keep circle, radius3 |
| R12 | Command shortcut badge | Circle, diameter18 / radius9 | CSS 9px; rendered circular radius 9 | Keep circle, radius9 |
| R13 | Quota glass: closed / expanded | 14 | CSS 14px; rendered circular radius 14 | Keep14 in both states |
| R14 | Quota summary button background | No independent fill | CSS 14px; rendered circular radius 14 | Keep no independent fill |
| R15 | Find bar glass | 12 | CSS 12px; rendered circular radius 12 | Keep12 |
| R16 | Find Previous / Next / Close hover | Requested16; rx11 / ry16 at22×38 | CSS 0px; rendered circular radius 0 | Keep for now; any new shape needs a focused visual decision |
| R17 | ClickView keyboard focus ring | Tab16; other controls request14; inset2; stroke2 | Tab outline2, offset−3 on18pt host; other CSS outlines follow their host | Keep per-control focus shapes |
| R18 | Terminal area / pane / split surfaces | 0 independent corner radius | CSS 0px; rendered circular radius 0 | Keep0 |
| R19 | Native search-field bezel | AppKit owns the shape | CSS 6px; rendered circular radius 6 | Keep NSSearchField |
| R20 | Traffic lights | AppKit owns the shape | CSS 50%; rendered circular radius 6 | Keep system controls |
| R21 | Close / quit confirmation surface | AppKit NSAlert | Illustrative CSS18; inspect the Close confirmation scene separately | Keep system dialog |
| R22 | Confirmation action buttons | AppKit NSAlert buttons | Illustrative CSS7; inspect the Close confirmation scene separately | Keep system buttons |
| R23 | Picker / menus / scrollbars / symbol curves | AppKit owns internal geometry | Not numerically simulated | Keep system ownership |
| R24 | Prototype toast | No native counterpart | CSS10 | Exclude from native radius scale |
| R25 | Prototype selectors / inspection controls | Outside the native app | CSS6 | Exclude from native radius scale |
| R26 | Measurement labels | Outside the native app | SVG rx3 | Exclude from native radius scale |

## Definitions and sources

- **R01** — Native custom content clipping, not a replacement for system window controls. The prototype right-side wrapper inherits34 but paints no separate rounded surface. [window.rs:47,699–705,2221–2233](../crates/combe/src/window.rs#L47).
- **R02** — The outer sidebar layer and material use the same radius. A36pt chip is a capsule; expansion keeps its corner radius. [window.rs:48,707–715](../crates/combe/src/window.rs#L48).
- **R03** — Native hover highlight is disabled. CSS18 is a transparent button shape, not a second visible glass surface. Focus is listed under R17. [window.rs:719–739](../crates/combe/src/window.rs#L719).
- **R04** — AppKit limits the two arc axes independently; the uniform CSS16 corners scale to15 at height30. This is a current renderer difference, not two design tokens. [chrome_view.rs:13,43–53; window.rs:1883–1889](../crates/combe/src/chrome_view.rs#L13).
- **R05** — Rows are34pt high. Selection and hover share this path. HTML uses the radius-row16 token for both list rows and repo headings. [Source](../crates/combe/src/chrome_view.rs).
- **R06** — Only the active native tab has this glass sibling. It is not the same layer as the hover highlight. [window.rs:1753–1763](../crates/combe/src/window.rs#L1753).
- **R07** — Tab titles set their ClickView radius to18, matching the glass. Other ClickViews retain16. [Source](../crates/combe/src/chrome_view.rs).
- **R08** — Native Close is20×36 and disables hover fill. The transparent HTML22×22 button declares50%;11 is not a native radius. Focus is R17. [window.rs:1766–1779](../crates/combe/src/window.rs#L1766).
- **R09** — Native36×36 target disables hover fill. HTML declares18 on a transparent button. Focus is R17. [window.rs:1783–1793](../crates/combe/src/window.rs#L1783).
- **R10** — Both are28×28 borderless NSButtons. That does not establish a native14pt circle. The prototype explicitly uses50%. [window.rs:820–837](../crates/combe/src/window.rs#L820).
- **R11** — Drawn with an oval in a square frame. A size-derived circle is not a general rounded-rectangle token. [chrome_view.rs:14,66–80](../crates/combe/src/chrome_view.rs#L14).
- **R12** — The badge background exists only while shortcut hints are shown. Its geometry is reserved while hidden; selecting this ID measures that reserved shape. [chrome_view.rs:85–94](../crates/combe/src/chrome_view.rs#L85).
- **R13** — The same glass panel grows upward. A28pt chip is a capsule; changing only the expanded radius would introduce a separate shape transition. [quota_panel.rs:69](../crates/combe/src/quota_panel.rs#L69).
- **R14** — The native summary disables hover fill; the14pt visible surface belongs to R13. HTML14 on the transparent trigger is not another painted layer. [quota_panel.rs:392–408](../crates/combe/src/quota_panel.rs#L392).
- **R15** — The material has radius12, but it is a sibling of the field and action buttons. Its clipping does not clip those sibling controls. Native height38; prototype height is content-derived. [find_bar.rs:117–122](../crates/combe/src/find_bar.rs#L117).
- **R16** — All three native actions use the shared ClickView fill. HTML buttons have no rounded hover surface. This is not the12pt exterior glass. [find_bar.rs:155–177; chrome_view.rs:43–53](../crates/combe/src/find_bar.rs#L155).
- **R17** — The focus path uses the instance corner radius minus2. Tabs use16; all other ClickViews retain14 before per-axis bounds clipping. Frames, hit tests and focus actions are unchanged. [Source](../crates/combe/src/chrome_view.rs).
- **R18** — No rounded terminal cards. Panes and split containers rely on the outer window clip; split dividers have no rounded surface. [surface.rs; split.rs; docs/DESIGN.md:66](../crates/combe/src/surface.rs).
- **R19** — Combe creates NSSearchField without setting its corner radius. HTML input6 is an illustrative browser control. [find_bar.rs:128–140](../crates/combe/src/find_bar.rs#L128).
- **R20** — The HTML12×12 circle resolves to radius6. It is not a fixed Combe system-button radius. [window.rs:2365–2380](../crates/combe/src/window.rs#L2365).
- **R21** — The HTML confirmation radius18 is a visual placeholder, not a native NSAlert constant. [window.rs:1017–1028](../crates/combe/src/window.rs#L1017).
- **R22** — HTML7 is illustrative. Combe does not assign a radius to native confirmation buttons. [window.rs:1017–1028](../crates/combe/src/window.rs#L1017).
- **R23** — NSOpenPanel, menu items, native scrollbars and SF Symbol ink have no Combe radius token. Text glyph curves, including the selected checkmark, are not rounded component backgrounds. [window.rs:install_menu,dispatch(Click::AddRepo),icon_button](../crates/combe/src/window.rs).
- **R24** — The repo-picker placeholder toast exists only in the interactive reference. It does not define a native notification component. [docs/design.html:.toast](design.html).
- **R25** — Appearance, Component and measurement controls belong to the reference page, not the Combe window. [docs/design.html:select,input,.spacing-controls button](design.html).
- **R26** — Annotation labels exist only in the inspector. Shadow blur and stroke width are separate quantities, not corner radii. [docs/design.spacing.js:draw](design.html).

## Native focus and small controls

`NSBezierPath` bounds its x and y corner radii independently. These are the axes of each corner oval, not four unrelated corner values. This behavior is distinct from the prototype's proportional scaling of uniform circular CSS corners. Do not apply the NSBezier rule to `CALayer.cornerRadius` or system controls. [Apple NSBezierPath reference](https://developer.apple.com/documentation/appkit/nsbezierpath/init(roundedrect:xradius:yradius:)?language=objc).

| ClickView role | Visible fill rx / ry | Focus rx / ry after inset2 |
| --- | --- | --- |
| Workspace trigger,162×36 | None | 14 /14;9 /14 at the minimum22pt trigger width |
| Tab,180×36 | 18 /18 on hover; glass18 behind it | 16 /16 |
| Tab Close,20×36 | None | 8 /14 |
| New tab,36×36 | None | 14 /14 |
| Repo heading,height30 | 16 /15 on hover | 14 /13 |
| Workspace row,height34 | 16 /16 on hover or selection | 14 /14 |
| Find action,22×38 | 11 /16 on hover | 9 /14 |
| Quota summary,height28 | None; glass14 behind it | 14 /12 at normal summary widths |

The native corner settings are34,18,16,14,12, plus zero and circles derived from diameters6 and18. Other values in the table are constrained path geometry or system/prototype-only shapes. The sidebar's3/24 shadow blur values are not corner radii.

## Verification

Only the Tab drawing radius and prototype list token differ from the other control defaults. Frames, hit targets, hover timers, keyboard actions, clipping owners and terminal surfaces retain their existing behavior. `ClickView::hitTest` uses the view hierarchy rather than the rounded path.

Native acceptance covers active/inactive Tab hover and keyboard focus in light and dark appearances, plus unchanged row and Find controls. The current change passes `make check` and all116 prototype assertions. The owner verified Tab hover, selection and keyboard focus in light and dark appearances, plus row, Find, Tab click and Close behavior.
