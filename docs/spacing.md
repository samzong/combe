# Spacing inventory

Use the IDs in this inventory to review spacing and request exact changes. It covers Combe-owned adjacent component boundaries, internal padding, text frames, icons, hit targets, split panes, zero gaps, and state-dependent free space. Repeated rows and tabs share one definition. Native system-control internals are identified as system-owned where Combe declares no numeric value.

**Current spacing:** terminals use 8 pt base padding; header text uses a 12 pt leading inset; Add–Pin and Pin–glass gaps are 4 pt. The quota summary has no offset from the terminal area and 8 pt horizontal text insets. Quota details use 16/16/12/16 pt padding, 8 pt column and heading gaps, and 30 pt rows. These values are implemented in the native app and prototype.

## Read the values

- Native values come from the current working tree based on `d5251d0`, inspected on 2026-09-10. They are declared or derived AppKit points, not a live native-view inspector.
- The normal-window baseline is 1200 × 780 pt root content, sidebar 300 pt, with tabs at the start of their scroll range. `W`/`H` are root dimensions; `G` is actual sidebar glass height. Closed `G=36`; transient `G=36 + capped catalog height`; pinned `G=H−24`. Full screen changes the traffic-light reservation. Narrow headers clamp label frames.
- Prototype values are a browser snapshot at 1200 × 760 unscaled CSS px, pinned sidebar, `gmc / main`, two panes, Find visible, and quota details open. P04 uses the No quota scene. The interactive inspector recalculates prototype values as the scene changes.
- `T/R/B/L` mean top/right/bottom/left; `X/Y` mean width/height. Zero means touching boxes or no additional padding. Negative means overlap or a box extending outside its parent. Flexible remaining space is not a spacing token.
- Text means its layout frame, not glyph ink. Native labels often fill an allocated frame while the HTML uses a short intrinsic-width span. Their trailing distances can differ without a corresponding padding difference. System SF Symbol ink and AppKit control internals are not assigned invented values.
- Native points and unscaled CSS pixels are comparable design units here, but physical screenshot pixels also depend on backing scale. Ghostty adds grid remainders after its explicit padding; a visible text edge need not be exactly 8 pt.

## Inspect and request a change

Open [design.html](design.html), enable **Show spacing**, select **Area**, then **Measure**. The canvas isolates that ID; the table retains every relationship in the area. **Sidebar** changes the scene. Quota and Find inspection reveal their components. **Area overview** combines that area's marks; single-ID inspection is clearer for the header. Disable Show spacing to restore the unobstructed preview.

Use `ID + edge + target value + scope`, for example:

```text
E01: T/R/B/L = 8 pt, every pane, including split and zoom.
H11: gap = 4 pt, all sidebar states.
H18: R = 4 pt, all sidebar states; keep its 28 × 28 target.
```

Review connected changes together. H04–H20 describe the same header allocation; S03/S05/S13 share list padding; E01/Q02/Q05 jointly determine terminal and quota text alignment. Do not sum multiple IDs that name the same distance.

## Spacing decisions

Use **0 / 4 / 8 / 12 / 16 pt** where it serves the layout. Keep the sidebar's 2 pt row separation, hairlines, control sizes, corner radii, and derived vertical centering. A consistent number series does not justify reducing text or hit-target space.

| Area | Current decision | Functional boundary |
| --- | --- | --- |
| Terminal | Keep 8 pt on every side and the 12 pt exterior layout | Preserve Ghostty grid balance, font and pane geometry. |
| Header | Text left inset 12; retain trigger width 162 at sidebar width 300 and its 2 pt gap to Add | H05 adds 2 pt of text space without shrinking the trigger. H11 and H18 remain 4. |
| Sidebar | Keep horizontal padding 6, row height 34, row gap 2 and existing marker positions | Preserve row hit areas and long-name capacity. |
| Quota summary | Left offset 0; horizontal text insets 8; width measured text +16 | Text capacity is unchanged. The chip hit target is 12 pt narrower; hover, click and keyboard activation require verification. Keep the existing font-space separators. |
| Quota details | Padding 16/16/12/16; gaps 8; columns 50/132/74; rows 30; provider gap 16 | Height is 28 + sum(26 + row count ×30) +16 between providers, so the final row retains bottom clearance. |
| Find | Keep the existing native layout | Preserve input width and full-height action targets. |

The native header allocation is 72 +162 +2 +28 +4 +28 +4 =300 pt. Prototype H09 remains 0; its trigger is 2 px wider. Prototype/native parity differences elsewhere remain explicitly recorded below and do not authorize changing the native controls.

## Current inventory

### Window and outer layout

| ID | Relationship | Native · pt | Prototype snapshot · px | Proposed · pt |
| --- | --- | --- | --- | --- |
| W01 | Glass → window edges | T 12 / R window − sidebar − 12 / B 12 pinned; flexible otherwise / L 12 open; 84 closed | T 12 / R 888 / B 12 / L 12 | Keep outer 12; keep closed left 84 |
| W02 | Glass → first tab | 12 | 12 | 12 |
| W03 | Glass → terminal area, pinned | 12 | 12 | 12 |
| W04 | Terminal area → window edges | T 60 / R 12 / B 40 with quota; 12 without / L sidebar + 24 pinned; 12 otherwise | T 60 / R 12 / B 40 / L 324 | Keep outer layout |
| W05 | Tab row allocation → terminal area | 0 | 0 | 0 |
| W06 | Window content size | X 1200 default / Y 780 default | X 1200 / Y 760 | Use 1200 × 780 for parity |
| W07 | Window and sidebar corner radii | 34 window / 18 sidebar; fullscreen 0 window | Window34 / sidebar18 | Keep |

Definition notes and source:

- **W01** — Sidebar width is adjustable. Right and unpinned bottom are remaining space, not spacing tokens. Normal window values; full screen uses a 12pt leading inset. [window.rs:2235–2257](../crates/combe/src/window.rs#L2235).
- **W02** — The header and tabs stay in place in all sidebar states. Measured at the start of the tab scroll range. [window.rs:2338–2340](../crates/combe/src/window.rs#L2338).
- **W03** — In transient mode the panel overlaps the terminal; a negative measurement means overlap. [window.rs:2344–2352](../crates/combe/src/window.rs#L2344).
- **W04** — This locates the complete terminal area, which can contain multiple surfaces. Each surface has separate E01 padding. [window.rs:2338–2355](../crates/combe/src/window.rs#L2338).
- **W05** — 60pt is the top row allocation; there is no extra gap after it. [window.rs:39,2344–2352](../crates/combe/src/window.rs#L39).
- **W06** — Size, not spacing. The current prototype intentionally remains 1200 × 760 during this audit. [habits.rs:47–48](../crates/combe/src/habits.rs#L47).
- **W07** — Radii are shape geometry, not empty-space tokens. [window.rs:47–48,2220–2229](../crates/combe/src/window.rs#L47).

### Workspace header and glass

| ID | Relationship | Native · pt | Prototype snapshot · px | Proposed · pt |
| --- | --- | --- | --- | --- |
| H01 | First traffic light → window | T 24 − system button height / 2 / L 20 | T 24 / L 20 | Keep center 24; align prototype |
| H02 | Traffic light → next traffic light | 20 − system button width | 8 | Keep 20pt origin pitch |
| H03 | Last traffic light → workspace trigger | 24 − system button width | 12 | Keep current origin positions |
| H04 | Workspace trigger → header | T 0 / R 66 / B 0 relative to header / L 0 closed; 72 open | T 0 / R 64 / B 0 / L 72 | Keep T 0 / R 66 / B 0 relative to header / L 0 closed; 72 open |
| H05 | Repo / branch text frame → trigger | T 9 / R 28 / B 9 / L 12 | T 9.6 / R 88.93 / B 9.6 / L 12 | Keep T 9 / R 28 / B 9 / L 12 |
| H06 | Repo / branch text frame → header | T 9 / R 94 / B 9 relative to header / L 12 closed; 84 open | T 9.6 / R 152.93 / B 9.6 / L 84 | Keep T 9 / R 94 / B 9 relative to header / L 12 closed; 84 open |
| H07 | Disclosure arrow → trigger | T 13 / R 12 / B 13 | T 13 / R 10 / B 13 | Keep native; prototype right 10 → 12 |
| H08 | Repo / branch text frame → arrow | 6 | 68.93 | Keep native 6 |
| H09 | Workspace trigger → Add button | 2 | 0 | Keep 2 |
| H10 | Repo / branch text frame → Add button | 30 = 28 + 2 | 88.93 | Keep 30 = 28 + 2 |
| H11 | Add button → Pin button | 4 | 4 | Keep4 |
| H12 | Add button → header top / bottom | T 4 / B 4 | T 4 / B 4 | 4 |
| H13 | Pin button → header top / right / bottom | T 4 / R 4 / B 4 | T 4 / R 4 / B 4 | Keep T4 R4 B4 |
| H14 | Add symbol → its button | T System / R System / B System / L System | T 6.5 / R 6.5 / B 6.5 / L 6.5 | Keep AppKit symbol layout |
| H15 | Pin symbol → its button | T System / R System / B System / L System | T 6.5 / R 6.5 / B 6.5 / L 6.5 | Keep AppKit symbol layout |
| H16 | Header and tool hit-target sizes | X 28 tool / Y 36 header / 28 tool | X 28 / Y 28 | Keep sizes |
| H17 | Add button → whole glass | T 4 / R 36 / B G − 32 / L sidebar − 64 open; sidebar − 136 closed | T 4 / R 36 / B 704 / L 236 | T4 R36 B(G−32); left derived |
| H18 | Pin button → whole glass | T 4 / R 4 / B G − 32 / L sidebar − 32 open; sidebar − 104 closed | T 4 / R 4 / B 704 / L 268 | Keep T4 R4 B(G−32); left derived |
| H19 | Repo / branch text frame → whole glass | T 9 / R 94 / B G − 27 / L 84 open; 12 closed | T 9.6 / R 152.93 / B 709.6 / L 84 | Keep T 9 / R 94 / B G − 27 / L 84 open; 12 closed |
| H20 | Repo / branch text frame → Pin button | 62 = 28 + 2 + 28 + 4 | 120.93 | Keep 62 = 28 + 2 + 28 + 4 |

Definition notes and source:

- **H01** — The prototype uses a 12px circle at y24, center30. Native centers the system button at y24. [window.rs:2365–2380](../crates/combe/src/window.rs#L2365).
- **H02** — The 8px prototype gap assumes 12px circles. Native button size belongs to AppKit. [window.rs:2370–2377](../crates/combe/src/window.rs#L2370).
- **H03** — At a 12pt button width this is 12pt. Do not treat that assumed width as a native constant. [window.rs:2291–2295,2370–2377](../crates/combe/src/window.rs#L2291).
- **H04** — The trigger is a hit target, not just the visible repo / branch text. Native width is sidebar − 138 (162 by default). Normal window reserves 72pt for traffic lights; full screen reserves 0. [window.rs:2291–2295](../crates/combe/src/window.rs#L2291).
- **H05** — Native has a full 18pt label frame. The prototype name span follows short text, so its right remainder is elastic. Native frame values assume the default 300pt sidebar; the text frame clamps at narrow widths. [chrome_view.rs:194–202](../crates/combe/src/chrome_view.rs#L194), [window.rs:719–724](../crates/combe/src/window.rs#L719).
- **H06** — These sums include the traffic-light reservation and button group. They are not four independent settings. Native frame values assume the default 300pt sidebar; the text frame clamps at narrow widths. [window.rs:2291–2295](../crates/combe/src/window.rs#L2291), [chrome_view.rs:194–202](../crates/combe/src/chrome_view.rs#L194).
- **H07** — 10 × 10. Hidden when pinned but its reserved space remains. [window.rs:2300–2308](../crates/combe/src/window.rs#L2300).
- **H08** — Native label-frame gap is fixed. Prototype short-label gap includes flexible free space. Native frame values assume the default 300pt sidebar; the text frame clamps at narrow widths. [window.rs:2303–2305](../crates/combe/src/window.rs#L2303), [chrome_view.rs:194–202](../crates/combe/src/chrome_view.rs#L194).
- **H09** — Prototype is currently 0 because its trigger is 2px wider. [window.rs:2294,2334](../crates/combe/src/window.rs#L2294).
- **H10** — This includes the arrow reservation. Actual short text leaves more visual free space. Native frame values assume the default 300pt sidebar; the text frame clamps at narrow widths. [window.rs:2294,2334](../crates/combe/src/window.rs#L2294), [chrome_view.rs:194–202](../crates/combe/src/chrome_view.rs#L194).
- **H11** — Both boxes are28 ×28. The native trigger-to-Add gap remains2; the prototype remains0. [window.rs:2316–2335](../crates/combe/src/window.rs#L2316).
- **H12** — Vertical centering is (36 − 28) / 2. [window.rs:2334–2335](../crates/combe/src/window.rs#L2334).
- **H13** — The right inset shares H18. Top and bottom are relative to the36pt header. [window.rs:2318–2319](../crates/combe/src/window.rs#L2318).
- **H14** — Only the prototype fixes SVG size at15px, centered in28px. Native SF Symbol ink/padding are system-owned. [window.rs:820–837](../crates/combe/src/window.rs#L820).
- **H15** — The prototype has 6.5px box insets. This is not a measured native glyph inset. [window.rs:820–837](../crates/combe/src/window.rs#L820).
- **H16** — Geometry, not spacing. Text, icon, hit target, and glass have different boundaries. [window.rs:40,44–45](../crates/combe/src/window.rs#L40).
- **H17** — G is actual glass height:36 closed,36 + capped catalog height transient, H−24 pinned. Normal window, default300pt sidebar; text uses native full frame, not short glyph ink. Bottom is free panel space, not header padding. [window.rs:2235–2257,2303–2334](../crates/combe/src/window.rs#L2235).
- **H18** — G is actual glass height:36 closed,36 + capped catalog height transient, H−24 pinned. Normal window, default300pt sidebar; text uses native full frame, not short glyph ink. Bottom is free panel space, not header padding. [window.rs:2235–2257,2303–2334](../crates/combe/src/window.rs#L2235).
- **H19** — G is actual glass height:36 closed,36 + capped catalog height transient, H−24 pinned. Normal window, default300pt sidebar; text uses native full frame, not short glyph ink. Bottom is free panel space, not header padding. [window.rs:2235–2257,2303–2334](../crates/combe/src/window.rs#L2235).
- **H20** — Includes the arrow reservation, the Add target and both inter-control gaps. Default300pt sidebar; short prototype text leaves flexible space. [window.rs:2303–2334](../crates/combe/src/window.rs#L2303).

### Tabs

| ID | Relationship | Native · pt | Prototype snapshot · px | Proposed · pt |
| --- | --- | --- | --- | --- |
| T01 | Tab → top row top / bottom | T 12 / B 12 | T 12 / B 12 | 12 |
| T02 | Tab size | X 180 / Y 36 | X 180 / Y 36 | Keep |
| T03 | Tab → next tab | 12 | 12 | 12 |
| T04 | Last tab → New tab button | 12 | 12 | 12 |
| T05 | Title text frame → tab | T 9 / R 28 / B 9 / L 12 | T 9.6 / R 104.48 / B 9.6 / L 14 | Native left12; keep 28 right reservation |
| T06 | Close hit target → tab | T 0 / R 4 / B 0 / L 156 | T 7 / R 8 / B 7 / L 150 | Keep full-height native target |
| T07 | Title text frame → Close hit target | 4 | 74.48 | 4 |
| T08 | Close glyph line frame → Close hit target | T9 R0 B9 L6; line18 | See note; different control structure | Keep native frame; center glyph only after review |
| T09 | New tab hit target | X 36 / Y 36 | X 36 / Y 36 | Keep |
| T10 | New tab button → window right edge | Flexible; right viewport inset12 | 456 | Keep flexible |

Definition notes and source:

- **T01** — Top row60, tab36. This already accounts for the 12px space below the tab. [window.rs:1748–1753](../crates/combe/src/window.rs#L1748).
- **T02** — Nominal size, not padding. [window.rs:40–41](../crates/combe/src/window.rs#L40).
- **T03** — Repeats for every adjacent tab; shown when there are at least two tabs. [window.rs:1780](../crates/combe/src/window.rs#L1780).
- **T04** — Same spacing definition as the tab-to-tab gap. [window.rs:1780–1785](../crates/combe/src/window.rs#L1780).
- **T05** — Native maximum label frame140 ×18. Prototype uses left14/right34 and a flex label. Glyph ink is not measured. [window.rs:1753](../crates/combe/src/window.rs#L1753), [chrome_view.rs:194–202](../crates/combe/src/chrome_view.rs#L194).
- **T06** — Native20 ×36; prototype22 ×22, right8/top7. These are distinct current implementations. [window.rs:1766–1774](../crates/combe/src/window.rs#L1766).
- **T07** — Short prototype labels leave extra elastic space. Native full label frame has a4pt gap. [window.rs:1753,1766–1774](../crates/combe/src/window.rs#L1753).
- **T08** — Prototype × is a text node at15px. Native uses a12pt font in a separate full-height20pt target. [window.rs:1766–1774](../crates/combe/src/window.rs#L1766), [chrome_view.rs:194–204](../crates/combe/src/chrome_view.rs#L194).
- **T09** — Native plus text frame has T9 R0 B9 L10; prototype centers a20px plus. [window.rs:1783–1789](../crates/combe/src/window.rs#L1783).
- **T10** — This is unused tab capacity, not a fixed margin. Overflow scrolls. [window.rs:1794–1796,2338–2340](../crates/combe/src/window.rs#L1794).

### Sidebar catalog

| ID | Relationship | Native · pt | Prototype snapshot · px | Proposed · pt |
| --- | --- | --- | --- | --- |
| S01 | Catalog viewport → glass | T 36 / R 0 / B 0 pinned / L 0 | T 36 / R 0 / B 411 / L 0 | Keep |
| S02 | Header → first repo heading | 8 | 8 | 8 |
| S03 | Catalog content padding | T 8 / R 6 / B 12 / L 6 | T 8 / R 6 / B 12 / L 6 | Keep T 8 / R 6 / B 12 / L 6 |
| S04 | Last workspace row → next repo heading | 16.5 = 8 + 0.5 + 8 | 16.5 | Keep 8 / hairline / 8 |
| S05 | Repo heading → catalog sides | R 6 / L 6 | R 6 / L 6 | Keep R 6 / L 6 |
| S06 | Repo heading text frame → heading | T 6 / R 28 / B 6 / L 34 | T 6.6 / R 229.02 / B 6.6 / L 34 | Keep current text and arrow frames |
| S07 | Folder icon → heading | T 8 / B 8 / L 12 | T 8 / B 8 / L 12 | Keep |
| S08 | Folder icon → repo title | 8 | 8 | 8 |
| S09 | Repo disclosure arrow → heading | T 10 / R 16 / B 10 | T 10 / R 12 / B 10 | Keep T 10 / R 16 / B 10 |
| S10 | Repo text frame → disclosure arrow | 2 | 207.02 | Keep 2 |
| S11 | Repo heading → first workspace row | 2 | 2 | Keep2 as dense-list exception |
| S12 | Workspace row → next workspace row | 2 | 2 | Keep2 as dense-list exception |
| S13 | Workspace row → catalog sides | R 6 / L 6 | R 6 / L 6 | Keep R 6 / L 6 |
| S14 | Workspace text frame → row | T 8 / R 34 / B 8 / L 48 | T 7.9 / R 211.02 / B 7.91 / L 48 | Keep T 8 / R 34 / B 8 / L 48 |
| S15 | Session dot → workspace row | T 14 / B 14 / L 32 | T 14 / B 14 / L 32 | Keep |
| S16 | Session dot → workspace text | 10 | 10 | Keep 10 |
| S17 | Selected marker frame → row | T 8 / R 9 / B 8 | T 10.5 / R 12 / B 10.5 | Keep T 8 / R 9 / B 8 |
| S18 | Shortcut hint → row | T 8 / R 9 / B 8 | T 8 / R 10 / B 8 | Keep T 8 / R 9 / B 8 |
| S19 | Workspace text frame → marker | 7 | 183.02 | Keep 7 |
| S20 | Last row → glass bottom | 12 + flexible remainder, pinned | 425 | Keep flexible; do not force uniform12 |
| S21 | Separator edges and focus outline | Separator L6/R6, stroke0.5; focus inset2, stroke2 | Separator0.5; focus inset3 / width2 | Separator edges follow S03; keep focus visible |
| S22 | Resize hit strip → glass | T −12 pinned / R −4 / B −12 pinned / L sidebar width | T 44 / R -4 / B 16 / L 300 | Keep exposed4pt strip |

Definition notes and source:

- **S01** — The native viewport fills the area under the36pt header. HTML catalog has content height with a max-height; its empty pinned remainder differs. [window.rs:2276–2288](../crates/combe/src/window.rs#L2276).
- **S02** — This is the catalog top padding, not an extra header-to-catalog gap. [window.rs:1856,1873–1885](../crates/combe/src/window.rs#L1856).
- **S03** — The12pt bottom is minimum natural content padding. Additional pinned empty space is flexible. [window.rs:1856,1873–1885,1958](../crates/combe/src/window.rs#L1856).
- **S04** — Browser measurement confirms16.5 after margin collapse. The same group separator pattern repeats. [window.rs:1875–1876,2028–2030](../crates/combe/src/window.rs#L1875).
- **S05** — Shares S03; do not add both values when computing the same edge. [window.rs:1883–1889](../crates/combe/src/window.rs#L1883).
- **S06** — Native30pt heading with18pt label frame. Prototype text span is intrinsic-width. [window.rs:1888–1889](../crates/combe/src/window.rs#L1888), [chrome_view.rs:194–202](../crates/combe/src/chrome_view.rs#L194).
- **S07** — 14 ×14 icon box; native and prototype agree. [window.rs:1897–1902](../crates/combe/src/window.rs#L1897).
- **S08** — This8pt gap is distinct from the12pt leading inset. [window.rs:1888–1902](../crates/combe/src/window.rs#L1888).
- **S09** — Native10 ×10 arrow sits4pt farther left than the prototype. [window.rs:1911](../crates/combe/src/window.rs#L1911).
- **S10** — Native text-frame gap2; prototype gap includes elastic short-text space. [window.rs:1888–1889,1911](../crates/combe/src/window.rs#L1888).
- **S11** — Heading height30 is a size; this2pt is an actual gap. [window.rs:1933–1935](../crates/combe/src/window.rs#L1933).
- **S12** — Visible row34, stride36. The same relation applies to all adjacent rows. [window.rs:1933–1935,1958](../crates/combe/src/window.rs#L1933).
- **S13** — Shares S03. Selected background and row hit target use this same outer box. [window.rs:1933–1935](../crates/combe/src/window.rs#L1933).
- **S14** — 13pt font inside18pt native line frame. Short prototype text leaves flexible trailing space. [window.rs:1940–1943](../crates/combe/src/window.rs#L1940), [chrome_view.rs:194–202](../crates/combe/src/chrome_view.rs#L194).
- **S15** — 6 ×6 dot; 14pt vertical inset is derived from34pt row height. [chrome_view.rs:70–78](../crates/combe/src/chrome_view.rs#L70).
- **S16** — Reducing the gap changes text indentation. Dot position remains32. [chrome_view.rs:70–78](../crates/combe/src/chrome_view.rs#L70), [window.rs:1940–1943](../crates/combe/src/window.rs#L1940).
- **S17** — Native check/shortcut share18 ×18. Prototype check13 ×13 with right12 differs. [chrome_view.rs:88–99](../crates/combe/src/chrome_view.rs#L88).
- **S18** — Prototype shortcut18 ×18 has right10. Inspect its reserved box even when the hint is not displayed. [chrome_view.rs:88–99](../crates/combe/src/chrome_view.rs#L88).
- **S19** — Fixed native full-label-frame gap; prototype short-label free space varies. [chrome_view.rs:88–99](../crates/combe/src/chrome_view.rs#L88), [window.rs:1940–1943](../crates/combe/src/window.rs#L1940).
- **S20** — A pinned sidebar fills the window; its catalog does not have to fill the height. Assumes unscrolled content without overflow; scrolling can make this distance negative. [window.rs:2284–2288,1958](../crates/combe/src/window.rs#L2284).
- **S21** — Hairlines and focus rings are drawing geometry, not content padding. [window.rs:2028–2030](../crates/combe/src/window.rs#L2028), [chrome_view.rs:58–64](../crates/combe/src/chrome_view.rs#L58).
- **S22** — Native hit strip spans the whole root split height, subject to covering views. Negative insets extend beyond glass. Prototype excludes44px at top and16px at bottom. [window.rs:2121–2134](../crates/combe/src/window.rs#L2121).

### Terminal and splits

| ID | Relationship | Native · pt | Prototype snapshot · px | Proposed · pt |
| --- | --- | --- | --- | --- |
| E01 | Each terminal pane: content padding | T 8 + grid remainder / R 8 + grid remainder / B 8 + grid remainder / L 8 + grid remainder | T 8 / R 8 / B 8 / L 8 | Keep8 on all four sides |
| E02 | Tab bottom → terminal surface top | 12 | 12 | 12 |
| E03 | Tab bottom → first text line box | 20 + grid remainder | 20 | 20 + grid remainder |
| E04 | Pinned glass right → text area left | 20 + grid remainder | 20 | 20 + grid remainder |
| E05 | Last pane text area → window right | 20 + grid remainder | 20 | 20 + grid remainder |
| E06 | Text area bottom → window bottom | 48 with quota; 20 without; plus grid remainder | 48 | 48 with quota; 20 without, plus remainder |
| E07 | Ghostty grid balance | Enabled; full cells plus distributed pixel remainder | Not simulated | Keep enabled |
| E08 | Terminal font and line height | 13pt compiled default; cell metrics owned by Ghostty/CoreText | 13 / line-height16 | Keep13pt; label prototype line-height as illustrative |
| E09 | Terminal area size | X W − sidebar − 36 pinned; W − 24 otherwise / Y H − 100 with quota; H − 72 without | X 864 / Y 660 | Keep surface size; recover space inside |
| P01 | Tab root / split container → surface | 0 extra padding on every side | Grid gap0 | 0 |
| P02 | Pane → adjacent pane | 0 extra gap; AppKit Thin divider | 0 | Keep0; retain native Thin |
| P03 | Text area → text area across split | 16 + Thin divider + grid remainders | 16.5 | 16 + Thin + remainders |
| P04 | No quota: surface → window bottom | 12 | 0 | Keep12; align prototype |

Definition notes and source:

- **E01** — Applies to every pane, including split and zoomed surfaces. Ghostty balance remains enabled. [habits.rs:35–37](../crates/combe/src/habits.rs#L35).
- **E02** — Ends at the surface edge. E03 additionally includes the terminal inner padding. [window.rs:1748–1753,2344–2352](../crates/combe/src/window.rs#L1748).
- **E03** — Includes the12pt gap below the tab plus8pt surface padding. Glyph ink sits inside a line box. [window.rs:1748–1753](../crates/combe/src/window.rs#L1748), [habits.rs:35–37](../crates/combe/src/habits.rs#L35).
- **E04** — 12pt outer gap plus8pt inner padding. In transient mode the glass overlays the terminal. [window.rs:2344–2352](../crates/combe/src/window.rs#L2344), [habits.rs:35–37](../crates/combe/src/habits.rs#L35).
- **E05** — 12pt outer gap plus the final pane's8pt inner padding. [window.rs:2344–2352](../crates/combe/src/window.rs#L2344), [habits.rs:35–37](../crates/combe/src/habits.rs#L35).
- **E06** — 40pt status allocation plus8pt padding. Without quota, native still reserves12pt outside the surface. Measures the bottommost visible pane in either split direction. [window.rs:2344–2352](../crates/combe/src/window.rs#L2344), [habits.rs:35–37](../crates/combe/src/habits.rs#L35).
- **E07** — 8pt is the explicit base, not a promise that each visible edge is exactly8pt. Prototype does not simulate grid balancing. [vendor/ghostty/src/renderer/size.zig:49–83,279–301](../vendor/ghostty/src/renderer/size.zig#L49).
- **E08** — Prototype13px/16px differs from the native grid. Padding recommendations do not change font or line height. [habits.rs:6–9](../crates/combe/src/habits.rs#L6), [vendor/ghostty/src/font/metrics.zig:265–283](../vendor/ghostty/src/font/metrics.zig#L265).
- **E09** — At1200 ×780, sidebar300 and quota:864 ×680. The prototype is20px shorter because its window is760px. W/H refer to the root content view, excluding system decoration. [window.rs:2344–2355](../crates/combe/src/window.rs#L2344).
- **P01** — Surface padding E01 remains separate. Split, zoom and survivor promotion do not add another outer wrapper. [split.rs:32,56,74,114](../crates/combe/src/split.rs#L32).
- **P02** — Prototype divider is a0.5px inside border. Native divider thickness is system-owned; no fixed thickness is declared. [split.rs:77](../crates/combe/src/split.rs#L77).
- **P03** — Each pane contributes its own8pt padding. The equivalent vertical split follows the same rule. [habits.rs:35–37](../crates/combe/src/habits.rs#L35), [split.rs:77](../crates/combe/src/split.rs#L77).
- **P04** — In the current No quota snapshot scene, the prototype leaves0 outside the surface. This is a recorded mismatch, not an applied change. [window.rs:2347](../crates/combe/src/window.rs#L2347).

### Quota

| ID | Relationship | Native · pt | Prototype snapshot · px | Proposed · pt |
| --- | --- | --- | --- | --- |
| Q01 | Quota chip → window bottom | B 12 | B 12 | 12 |
| Q02 | Terminal surface left → quota left | 0 | 0 | Keep 0 |
| Q03 | Terminal surface bottom → closed chip top | 0 | 0 | 0 |
| Q04 | Quota chip size | X measured summary text + 16 / Y 28 | X 176 / Y 28 | Keep X measured summary text + 16 / Y 28 |
| Q05 | Summary horizontal padding | R 8 / L 8 | R 8 / L 8 | Keep R 8 / L 8 |
| Q06 | Summary text line → chip top / bottom | T 5 / B 5 | T 5.6 / B 5.6 | Keep18pt line → 5pt top/bottom |
| Q07 | Provider name → percentage | 2 font spaces | 6 | Keep 2 font spaces |
| Q08 | Provider name → percentage gap | 2 font spaces | 6 | Keep 2 font spaces |
| Q09 | Provider block → next provider block | 3 font spaces | 18 | Keep 3 font spaces |
| Q10 | Expanded summary → glass right | 304 − content-derived chip width | 128 | Keep flexible |
| Q11 | Details → summary seam | 0 | 0 | 0 |
| Q12 | Details padding | T 16 / R 16 / B 12 / L 16 | T 16 / R 16 / B 12 / L 16 | Keep T 16 / R 16 / B 12 / L 16 |
| Q13 | Window column → remaining column | 8 | 8 | Keep 8 |
| Q14 | Remaining column → reset column | 8 | 8 | Keep 8 |
| Q15 | Quota data row size | X 272 inner width / Y 30 | X 272 / Y 30 | Keep X 272 inner width / Y 30 |
| Q16 | Quota row text → row top / bottom | T 6 / B 6 | T 6.6 / B 6.6 | Keep18pt line → 6pt top/bottom |
| Q17 | Quota row → next row | 0 | 0 | 0 |
| Q18 | Provider heading → first row box | 8 | 8 | Keep 8 |
| Q19 | Provider group → next heading | 16 | 16 | 16 |
| Q20 | Quota corners / expansion sizing | 14 radius closed and expanded; detail width304 | See note; different control structure | Keep |

Definition notes and source:

- **Q01** — The chip shares the window bottom inset. [quota_panel.rs:100–122](../crates/combe/src/quota_panel.rs#L100).
- **Q02** — Summary text starts8pt from the terminal area edge, matching its base text inset. Ghostty grid balance can add remainder inside each pane. [quota_panel.rs:109](../crates/combe/src/quota_panel.rs#L109).
- **Q03** — 28pt chip +12pt bottom inset uses40pt. Expansion covers the terminal without increasing the reserve. [quota_panel.rs:21–24,499–511](../crates/combe/src/quota_panel.rs#L21).
- **Q04** — Prototype fixes width176 for two providers or92 for one. Native width is content-derived. Horizontal text space is preserved; the summary hit target is12pt narrower. [quota_panel.rs:381–397](../crates/combe/src/quota_panel.rs#L381).
- **Q05** — CSS top/bottom padding0 is not the vertical text gap. Q06 measures the line box separately. [quota_panel.rs:381–397](../crates/combe/src/quota_panel.rs#L381), [chrome_view.rs:194–202](../crates/combe/src/chrome_view.rs#L194).
- **Q06** — Prototype line box is16.8px, giving5.6px above/below. Native line frame is18pt. [chrome_view.rs:194–202](../crates/combe/src/chrome_view.rs#L194).
- **Q07** — Rendered prototype text-box gap; Q08 shows the declared CSS gap. Native uses2 spaces inside a single string. [quota.rs:145](../crates/combe/src/quota.rs#L145).
- **Q08** — Native does not define6pt. HTML explicitly defines6px. Measure a name/number pair, not the whole string. [quota.rs:145](../crates/combe/src/quota.rs#L145).
- **Q09** — Only applicable with two providers. HTML explicitly uses18px. Native uses3 spaces in a single string. [quota_panel.rs:385](../crates/combe/src/quota_panel.rs#L385).
- **Q10** — Prototype176px summary leaves128px when expanded. This is free panel space, not summary padding. [quota_panel.rs:499–507](../crates/combe/src/quota_panel.rs#L499).
- **Q11** — Same glass surface; no separate gap or divider. [quota_panel.rs:547–549](../crates/combe/src/quota_panel.rs#L547).
- **Q12** — Details height is28 + sum(26 + row count ×30) +16 between providers. This includes all rows and the bottom inset. [quota_panel.rs:540–563](../crates/combe/src/quota_panel.rs#L540).
- **Q13** — Three native columns are50 /132 /74 wide, with8pt gaps inside272pt. [quota_panel.rs:573–596](../crates/combe/src/quota_panel.rs#L573).
- **Q14** — Same horizontal gap definition as Q13. [quota_panel.rs:573–596](../crates/combe/src/quota_panel.rs#L573).
- **Q15** — Row size, not its text padding. Neighbouring row boxes have no gap. [quota_panel.rs:573–602](../crates/combe/src/quota_panel.rs#L573).
- **Q16** — For the percentage the prototype has a slightly different line height. Native all columns use18pt frames. [quota_panel.rs:573–596](../crates/combe/src/quota_panel.rs#L573).
- **Q17** — Text-to-text distance adds both row insets:6 +0 +6 =12pt. [quota_panel.rs:602](../crates/combe/src/quota_panel.rs#L602).
- **Q18** — Native heading18pt; first row text starts6pt farther inside that row. [quota_panel.rs:563–573](../crates/combe/src/quota_panel.rs#L563).
- **Q19** — Last row text-to-heading distance is22pt, including6pt row bottom inset. [quota_panel.rs:604](../crates/combe/src/quota_panel.rs#L604).
- **Q20** — Same background and corner radius in both states. These are geometry values, not spacing. [quota_panel.rs:23–24,69,499–507](../crates/combe/src/quota_panel.rs#L23).

### Find bar

| ID | Relationship | Native · pt | Prototype snapshot · px | Proposed · pt |
| --- | --- | --- | --- | --- |
| F01 | Find bar → pane edges | T 6 / R 6 / B Flexible / L Flexible | T 8 / R 12 / B 610.8 / L 165.84 | Keep T 6 / R 6 / B Flexible / L Flexible |
| F02 | Find bar size | X max(0, min(340, initial pane width − 12)) / Y 38 | X 254.16 / Y 41.2 | Keep X max(0, min(340, initial pane width − 12)) / Y 38 |
| F03 | Search field → bar edges | T 7 / B 7 / L 6 | T 6 / B 6 / L 8 | Keep T 7 / B 7 / L 6 |
| F04 | Search field → count frame | 0 | 8 | Keep 0 |
| F05 | Count frame → bar top / bottom | T 10 / B 10 | T 12.9 / B 12.9 | Keep T 10 / B 10 |
| F06 | Count frame → Previous target | 0 | 8 | Keep 0 |
| F07 | Previous target → Next target | 0 | 8 | Keep 0 |
| F08 | Next target → Close target | 0 | 8 | Keep 0 |
| F09 | Previous target → bar top / bottom | T 0 / B 0 | T 10.5 / B 10.5 | Keep T 0 / B 0 |
| F10 | Close target → bar right edge | R 12 | R 8 | Keep R 12 |
| F11 | Find action glyph line frame | T10 R0 B10 L6, line18, font14 | See note; different control structure | Keep T10 R0 B10 L6, line18, font14 |
| F12 | Search icon / editable text / clear button | System-owned; no fixed internal pt values | See note; different control structure | Keep System-owned; no fixed internal pt values |

Definition notes and source:

- **F01** — The prototype currently uses top8/right12. The native bar is overlaid in the focused pane. [find_bar.rs:104–110](../crates/combe/src/find_bar.rs#L104).
- **F02** — Prototype is content-sized. Control height is not an external gap. Native width is computed at creation and not recomputed when that pane is later resized. [find_bar.rs:19–20,104–110](../crates/combe/src/find_bar.rs#L19).
- **F03** — Native nominal field192 ×24. Browser120px input has different intrinsic padding. [find_bar.rs:123–132](../crates/combe/src/find_bar.rs#L123).
- **F04** — A true0 frame gap in native AppKit. HTML flex gap8 is not native reality. [find_bar.rs:131–147](../crates/combe/src/find_bar.rs#L131).
- **F05** — Native64 ×18 frame. Actual number ink leaves content-dependent side space. [find_bar.rs:141–147](../crates/combe/src/find_bar.rs#L141).
- **F06** — Prototype has8px flex gap. [find_bar.rs:155](../crates/combe/src/find_bar.rs#L155).
- **F07** — Native targets each22 ×38 with no inter-button gaps. Preserve the current field and target widths. [find_bar.rs:163–177](../crates/combe/src/find_bar.rs#L163).
- **F08** — Same relationship as F07. [find_bar.rs:163–177](../crates/combe/src/find_bar.rs#L163).
- **F09** — Native target spans38pt bar height. Prototype button uses browser intrinsic sizing. [find_bar.rs:163–177](../crates/combe/src/find_bar.rs#L163).
- **F10** — The current native left/right outer insets are6/12. [find_bar.rs:124,155–177](../crates/combe/src/find_bar.rs#L124).
- **F11** — Text-frame insets are separate from the22 ×38 target. Prototype browser defaults differ. [find_bar.rs:165–175](../crates/combe/src/find_bar.rs#L165), [chrome_view.rs:194–202](../crates/combe/src/chrome_view.rs#L194).
- **F12** — NSSearchField and browser type=search have different internal geometry. Do not invent numeric parity. [find_bar.rs:128–140](../crates/combe/src/find_bar.rs#L128).

### System-owned controls

| ID | Relationship | Native · pt | Prototype snapshot · px | Proposed · pt |
| --- | --- | --- | --- | --- |
| A01 | Close / quit confirmation internals | AppKit NSAlert owns padding and button layout | Dialog padding24; actions gap8 / top20 | Keep system layout |
| A02 | Directory picker internals | AppKit NSOpenPanel owns layout | Toast placeholder | Keep system layout |
| A03 | Menu items / menu margins / traffic-light ink | AppKit owns internal geometry | 12px traffic circles | Keep system layout |

Definition notes and source:

- **A01** — Prototype dialog padding24, action gap8, action top margin20 are illustrative browser values. [window.rs:1017–1028](../crates/combe/src/window.rs#L1017).
- **A02** — The prototype toast is only a placeholder; it is not the native picker layout. [window.rs:dispatch(Click::AddRepo)](../crates/combe/src/window.rs).
- **A03** — Native menus, tooltips and system control glyphs do not have Combe spacing constants. [window.rs:install_menu,icon_button](../crates/combe/src/window.rs).

## Verification

The browser check `checkSpacing()` in [design.check.js](design.check.js) exercises the toggle, unchanged app geometry, non-intercepting overlay, zero gaps, unscaled measurements, sidebar states, horizontal/vertical and single-pane applicability, quota visibility and Find visibility. Run it with the other prototype checks documented in [DESIGN.md](DESIGN.md#gui-acceptance). Visually inspect both appearances. The current native changes pass `make check`; all 113 prototype assertions pass, including summary fit and details containment with one or two providers. The owner verified native sidebar and long-name behavior, quota hover/click/keyboard interaction and final-row visibility, light/dark appearance, splits, zoom and search.
