(() => {
  const metrics = [
    {"id": "W01", "group": "window", "label": "Glass → window edges", "native": {"T": "12", "R": "window − sidebar − 12", "B": "12 pinned; flexible otherwise", "L": "12 open; 84 closed"}, "proposal": "Keep outer 12; keep closed left 84", "geometry": {"type": "inset", "outer": "#win", "inner": ".platter", "mask": "TRBL"}, "note": "Sidebar width is adjustable. Right and unpinned bottom are remaining space, not spacing tokens. Normal window values; full screen uses a 12pt leading inset.", "source": "window.rs:2235–2257"},
    {"id": "W02", "group": "window", "label": "Glass → first tab", "native": "12", "proposal": "12", "geometry": {"type": "gap", "a": ".platter", "ae": "R", "b": ".tab", "be": "L"}, "note": "The header and tabs stay in place in all sidebar states. Measured at the start of the tab scroll range.", "source": "window.rs:2338–2340"},
    {"id": "W03", "group": "window", "label": "Glass → terminal area, pinned", "native": "12", "proposal": "12", "geometry": {"type": "gap", "a": ".platter", "ae": "R", "b": ".term", "be": "L"}, "note": "In transient mode the panel overlaps the terminal; a negative measurement means overlap.", "source": "window.rs:2344–2352"},
    {"id": "W04", "group": "window", "label": "Terminal area → window edges", "native": {"T": "60", "R": "12", "B": "40 with quota; 12 without", "L": "sidebar + 24 pinned; 12 otherwise"}, "proposal": "Keep outer layout", "geometry": {"type": "inset", "outer": "#win", "inner": ".term", "mask": "TRBL"}, "note": "This locates the complete terminal area, which can contain multiple surfaces. Each surface has separate E01 padding.", "source": "window.rs:2338–2355"},
    {"id": "W05", "group": "window", "label": "Tab row allocation → terminal area", "native": "0", "proposal": "0", "geometry": {"type": "gap", "a": ".tabs", "ae": "B", "b": ".term", "be": "T"}, "note": "60pt is the top row allocation; there is no extra gap after it.", "source": "window.rs:39,2344–2352"},
    {"id": "W06", "group": "window", "label": "Window content size", "native": {"X": "1200 default", "Y": "780 default"}, "proposal": "Use 1200 × 780 for parity", "geometry": {"type": "size", "selector": "#win"}, "note": "Size, not spacing. The current prototype intentionally remains 1200 × 760 during this audit.", "source": "habits.rs:47–48"},
    {"id": "W07", "group": "window", "label": "Window and sidebar corner radii", "native": "34 window / 18 sidebar; fullscreen 0 window", "proposal": "Keep", "geometry": {"type": "info", "selector": ".platter"}, "note": "Radii are shape geometry, not empty-space tokens.", "source": "window.rs:47–48,2220–2229"},
    {"id": "H01", "group": "header", "label": "First traffic light → window", "native": {"T": "24 − system button height / 2", "L": "20"}, "proposal": "Keep center 24; align prototype", "geometry": {"type": "inset", "outer": "#win", "inner": ".traffic i:first-child", "mask": "TL"}, "note": "The prototype uses a 12px circle at y24, center30. Native centers the system button at y24.", "source": "window.rs:2365–2380"},
    {"id": "H02", "group": "header", "label": "Traffic light → next traffic light", "native": "20 − system button width", "proposal": "Keep 20pt origin pitch", "geometry": {"type": "gap", "a": ".traffic i:first-child", "ae": "R", "b": ".traffic i:nth-child(2)", "be": "L"}, "note": "The 8px prototype gap assumes 12px circles. Native button size belongs to AppKit.", "source": "window.rs:2370–2377"},
    {"id": "H03", "group": "header", "label": "Last traffic light → workspace trigger", "native": "24 − system button width", "proposal": "Keep current origin positions", "geometry": {"type": "gap", "a": ".traffic i:last-child", "ae": "R", "b": "#trigger", "be": "L"}, "note": "At a 12pt button width this is 12pt. Do not treat that assumed width as a native constant.", "source": "window.rs:2291–2295,2370–2377"},
    {"id": "H04", "group": "header", "label": "Workspace trigger → header", "native": {"T": "0", "R": "66", "B": "0 relative to header", "L": "0 closed; 72 open"}, "proposal": "Keep T 0 / R 66 / B 0 relative to header / L 0 closed; 72 open", "geometry": {"type": "inset", "outer": ".menu-header", "inner": "#trigger", "mask": "TRBL"}, "note": "The trigger is a hit target, not just the visible repo / branch text. Native width is sidebar − 138 (162 by default). Normal window reserves 72pt for traffic lights; full screen reserves 0.", "source": "window.rs:2291–2295"},
    {"id": "H05", "group": "header", "label": "Repo / branch text frame → trigger", "native": {"T": "9", "R": "28", "B": "9", "L": "12"}, "proposal": "Keep T 9 / R 28 / B 9 / L 12", "geometry": {"type": "inset", "outer": "#trigger", "inner": "#current", "mask": "TRBL"}, "note": "Native has a full 18pt label frame. The prototype name span follows short text, so its right remainder is elastic. Native frame values assume the default 300pt sidebar; the text frame clamps at narrow widths.", "source": "chrome_view.rs:194–202; window.rs:719–724"},
    {"id": "H06", "group": "header", "label": "Repo / branch text frame → header", "native": {"T": "9", "R": "94", "B": "9 relative to header", "L": "12 closed; 84 open"}, "proposal": "Keep T 9 / R 94 / B 9 relative to header / L 12 closed; 84 open", "geometry": {"type": "inset", "outer": ".menu-header", "inner": "#current", "mask": "TRBL"}, "note": "These sums include the traffic-light reservation and button group. They are not four independent settings. Native frame values assume the default 300pt sidebar; the text frame clamps at narrow widths.", "source": "window.rs:2291–2295; chrome_view.rs:194–202"},
    {"id": "H07", "group": "header", "label": "Disclosure arrow → trigger", "native": {"T": "13", "R": "12", "B": "13"}, "proposal": "Keep native; prototype right 10 → 12", "geometry": {"type": "inset", "outer": "#trigger", "inner": ".trigger .chev", "mask": "TRB"}, "note": "10 × 10. Hidden when pinned but its reserved space remains.", "source": "window.rs:2300–2308"},
    {"id": "H08", "group": "header", "label": "Repo / branch text frame → arrow", "native": "6", "proposal": "Keep native 6", "geometry": {"type": "gap", "a": "#current", "ae": "R", "b": ".trigger .chev", "be": "L"}, "note": "Native label-frame gap is fixed. Prototype short-label gap includes flexible free space. Native frame values assume the default 300pt sidebar; the text frame clamps at narrow widths.", "source": "window.rs:2303–2305; chrome_view.rs:194–202"},
    {"id": "H09", "group": "header", "label": "Workspace trigger → Add button", "native": "2", "proposal": "Keep 2", "geometry": {"type": "gap", "a": "#trigger", "ae": "R", "b": "#add", "be": "L"}, "note": "Prototype is currently 0 because its trigger is 2px wider.", "source": "window.rs:2294,2334"},
    {"id": "H10", "group": "header", "label": "Repo / branch text frame → Add button", "native": "30 = 28 + 2", "proposal": "Keep 30 = 28 + 2", "geometry": {"type": "gap", "a": "#current", "ae": "R", "b": "#add", "be": "L"}, "note": "This includes the arrow reservation. Actual short text leaves more visual free space. Native frame values assume the default 300pt sidebar; the text frame clamps at narrow widths.", "source": "window.rs:2294,2334; chrome_view.rs:194–202"},
    {"id": "H11", "group": "header", "label": "Add button → Pin button", "native": "4", "proposal": "Keep4", "geometry": {"type": "gap", "a": "#add", "ae": "R", "b": "#pin", "be": "L"}, "note": "Both boxes are28 ×28. The native trigger-to-Add gap remains2; the prototype remains0.", "source": "window.rs:2316–2335"},
    {"id": "H12", "group": "header", "label": "Add button → header top / bottom", "native": {"T": "4", "B": "4"}, "proposal": "4", "geometry": {"type": "inset", "outer": ".menu-header", "inner": "#add", "mask": "TB"}, "note": "Vertical centering is (36 − 28) / 2.", "source": "window.rs:2334–2335"},
    {"id": "H13", "group": "header", "label": "Pin button → header top / right / bottom", "native": {"T": "4", "R": "4", "B": "4"}, "proposal": "Keep T4 R4 B4", "geometry": {"type": "inset", "outer": ".menu-header", "inner": "#pin", "mask": "TRB"}, "note": "The right inset shares H18. Top and bottom are relative to the36pt header.", "source": "window.rs:2318–2319"},
    {"id": "H14", "group": "header", "label": "Add symbol → its button", "native": {"T": "System", "R": "System", "B": "System", "L": "System"}, "proposal": "Keep AppKit symbol layout", "geometry": {"type": "inset", "outer": "#add", "inner": "#add svg", "mask": "TRBL"}, "note": "Only the prototype fixes SVG size at15px, centered in28px. Native SF Symbol ink/padding are system-owned.", "source": "window.rs:820–837"},
    {"id": "H15", "group": "header", "label": "Pin symbol → its button", "native": {"T": "System", "R": "System", "B": "System", "L": "System"}, "proposal": "Keep AppKit symbol layout", "geometry": {"type": "inset", "outer": "#pin", "inner": "#pin svg", "mask": "TRBL"}, "note": "The prototype has 6.5px box insets. This is not a measured native glyph inset.", "source": "window.rs:820–837"},
    {"id": "H16", "group": "header", "label": "Header and tool hit-target sizes", "native": {"X": "28 tool", "Y": "36 header / 28 tool"}, "proposal": "Keep sizes", "geometry": {"type": "size", "selector": "#pin"}, "note": "Geometry, not spacing. Text, icon, hit target, and glass have different boundaries.", "source": "window.rs:40,44–45"},
    {"id": "H17", "group": "header", "label": "Add button → whole glass", "native": {"T": "4", "R": "36", "B": "G − 32", "L": "sidebar − 64 open; sidebar − 136 closed"}, "proposal": "T4 R36 B(G−32); left derived", "geometry": {"type": "inset", "outer": ".platter", "inner": "#add", "mask": "TRBL"}, "note": "G is actual glass height:36 closed,36 + capped catalog height transient, H−24 pinned. Normal window, default300pt sidebar; text uses native full frame, not short glyph ink. Bottom is free panel space, not header padding.", "source": "window.rs:2235–2257,2303–2334"},
    {"id": "H18", "group": "header", "label": "Pin button → whole glass", "native": {"T": "4", "R": "4", "B": "G − 32", "L": "sidebar − 32 open; sidebar − 104 closed"}, "proposal": "Keep T4 R4 B(G−32); left derived", "geometry": {"type": "inset", "outer": ".platter", "inner": "#pin", "mask": "TRBL"}, "note": "G is actual glass height:36 closed,36 + capped catalog height transient, H−24 pinned. Normal window, default300pt sidebar; text uses native full frame, not short glyph ink. Bottom is free panel space, not header padding.", "source": "window.rs:2235–2257,2303–2334"},
    {"id": "H19", "group": "header", "label": "Repo / branch text frame → whole glass", "native": {"T": "9", "R": "94", "B": "G − 27", "L": "84 open; 12 closed"}, "proposal": "Keep T 9 / R 94 / B G − 27 / L 84 open; 12 closed", "geometry": {"type": "inset", "outer": ".platter", "inner": "#current", "mask": "TRBL"}, "note": "G is actual glass height:36 closed,36 + capped catalog height transient, H−24 pinned. Normal window, default300pt sidebar; text uses native full frame, not short glyph ink. Bottom is free panel space, not header padding.", "source": "window.rs:2235–2257,2303–2334"},
    {"id": "H20", "group": "header", "label": "Repo / branch text frame → Pin button", "native": "62 = 28 + 2 + 28 + 4", "proposal": "Keep 62 = 28 + 2 + 28 + 4", "geometry": {"type": "gap", "a": "#current", "ae": "R", "b": "#pin", "be": "L"}, "note": "Includes the arrow reservation, the Add target and both inter-control gaps. Default300pt sidebar; short prototype text leaves flexible space.", "source": "window.rs:2303–2334"},
    {"id": "T01", "group": "tabs", "label": "Tab → top row top / bottom", "native": {"T": "12", "B": "12"}, "proposal": "12", "geometry": {"type": "inset", "outer": ".tabs", "inner": ".tab", "mask": "TB"}, "note": "Top row60, tab36. This already accounts for the 12px space below the tab.", "source": "window.rs:1748–1753"},
    {"id": "T02", "group": "tabs", "label": "Tab size", "native": {"X": "180", "Y": "36"}, "proposal": "Keep", "geometry": {"type": "size", "selector": ".tab"}, "note": "Nominal size, not padding.", "source": "window.rs:40–41"},
    {"id": "T03", "group": "tabs", "label": "Tab → next tab", "native": "12", "proposal": "12", "geometry": {"type": "gap", "a": ".tab:first-child", "ae": "R", "b": ".tab:nth-child(2)", "be": "L"}, "note": "Repeats for every adjacent tab; shown when there are at least two tabs.", "source": "window.rs:1780"},
    {"id": "T04", "group": "tabs", "label": "Last tab → New tab button", "native": "12", "proposal": "12", "geometry": {"type": "gap", "a": ".tab:last-of-type", "ae": "R", "b": ".newtab", "be": "L"}, "note": "Same spacing definition as the tab-to-tab gap.", "source": "window.rs:1780–1785"},
    {"id": "T05", "group": "tabs", "label": "Title text frame → tab", "native": {"T": "9", "R": "28", "B": "9", "L": "12"}, "proposal": "Native left12; keep 28 right reservation", "geometry": {"type": "inset", "outer": ".tab", "inner": ".tab .label", "mask": "TRBL"}, "note": "Native maximum label frame140 ×18. Prototype uses left14/right34 and a flex label. Glyph ink is not measured.", "source": "window.rs:1753; chrome_view.rs:194–202"},
    {"id": "T06", "group": "tabs", "label": "Close hit target → tab", "native": {"T": "0", "R": "4", "B": "0", "L": "156"}, "proposal": "Keep full-height native target", "geometry": {"type": "inset", "outer": ".tab", "inner": ".tab .x", "mask": "TRBL"}, "note": "Native20 ×36; prototype22 ×22, right8/top7. These are distinct current implementations.", "source": "window.rs:1766–1774"},
    {"id": "T07", "group": "tabs", "label": "Title text frame → Close hit target", "native": "4", "proposal": "4", "geometry": {"type": "gap", "a": ".tab .label", "ae": "R", "b": ".tab .x", "be": "L"}, "note": "Short prototype labels leave extra elastic space. Native full label frame has a4pt gap.", "source": "window.rs:1753,1766–1774"},
    {"id": "T08", "group": "tabs", "label": "Close glyph line frame → Close hit target", "native": "T9 R0 B9 L6; line18", "proposal": "Keep native frame; center glyph only after review", "geometry": {"type": "info", "selector": ".tab .x"}, "note": "Prototype × is a text node at15px. Native uses a12pt font in a separate full-height20pt target.", "source": "window.rs:1766–1774; chrome_view.rs:194–204"},
    {"id": "T09", "group": "tabs", "label": "New tab hit target", "native": {"X": "36", "Y": "36"}, "proposal": "Keep", "geometry": {"type": "size", "selector": ".newtab"}, "note": "Native plus text frame has T9 R0 B9 L10; prototype centers a20px plus.", "source": "window.rs:1783–1789"},
    {"id": "T10", "group": "tabs", "label": "New tab button → window right edge", "native": "Flexible; right viewport inset12", "proposal": "Keep flexible", "geometry": {"type": "gap", "a": ".newtab", "ae": "R", "b": "#win", "be": "R"}, "note": "This is unused tab capacity, not a fixed margin. Overflow scrolls.", "source": "window.rs:1794–1796,2338–2340"},
    {"id": "S01", "group": "sidebar", "label": "Catalog viewport → glass", "native": {"T": "36", "R": "0", "B": "0 pinned", "L": "0"}, "proposal": "Keep", "geometry": {"type": "inset", "outer": ".platter", "inner": "#catalog", "mask": "TRBL"}, "note": "The native viewport fills the area under the36pt header. HTML catalog has content height with a max-height; its empty pinned remainder differs.", "source": "window.rs:2276–2288"},
    {"id": "S02", "group": "sidebar", "label": "Header → first repo heading", "native": "8", "proposal": "8", "geometry": {"type": "gap", "a": ".menu-header", "ae": "B", "b": ".group-head", "be": "T"}, "note": "This is the catalog top padding, not an extra header-to-catalog gap.", "source": "window.rs:1856,1873–1885"},
    {"id": "S03", "group": "sidebar", "label": "Catalog content padding", "native": {"T": "8", "R": "6", "B": "12", "L": "6"}, "proposal": "Keep T 8 / R 6 / B 12 / L 6", "geometry": {"type": "padding", "selector": "#catalog"}, "note": "The12pt bottom is minimum natural content padding. Additional pinned empty space is flexible.", "source": "window.rs:1856,1873–1885,1958"},
    {"id": "S04", "group": "sidebar", "label": "Last workspace row → next repo heading", "native": "16.5 = 8 + 0.5 + 8", "proposal": "Keep 8 / hairline / 8", "geometry": {"type": "gap", "a": ".group:first-child .row:last-child", "ae": "B", "b": ".group:nth-child(2) .group-head", "be": "T"}, "note": "Browser measurement confirms16.5 after margin collapse. The same group separator pattern repeats.", "source": "window.rs:1875–1876,2028–2030"},
    {"id": "S05", "group": "sidebar", "label": "Repo heading → catalog sides", "native": {"R": "6", "L": "6"}, "proposal": "Keep R 6 / L 6", "geometry": {"type": "inset", "outer": "#catalog", "inner": ".group-head", "mask": "RL"}, "note": "Shares S03; do not add both values when computing the same edge.", "source": "window.rs:1883–1889"},
    {"id": "S06", "group": "sidebar", "label": "Repo heading text frame → heading", "native": {"T": "6", "R": "28", "B": "6", "L": "34"}, "proposal": "Keep current text and arrow frames", "geometry": {"type": "inset", "outer": ".group-head", "inner": ".group-head > span", "mask": "TRBL"}, "note": "Native30pt heading with18pt label frame. Prototype text span is intrinsic-width.", "source": "window.rs:1888–1889; chrome_view.rs:194–202"},
    {"id": "S07", "group": "sidebar", "label": "Folder icon → heading", "native": {"T": "8", "B": "8", "L": "12"}, "proposal": "Keep", "geometry": {"type": "inset", "outer": ".group-head", "inner": ".group-head > svg:first-child", "mask": "TBL"}, "note": "14 ×14 icon box; native and prototype agree.", "source": "window.rs:1897–1902"},
    {"id": "S08", "group": "sidebar", "label": "Folder icon → repo title", "native": "8", "proposal": "8", "geometry": {"type": "gap", "a": ".group-head > svg:first-child", "ae": "R", "b": ".group-head > span", "be": "L"}, "note": "This8pt gap is distinct from the12pt leading inset.", "source": "window.rs:1888–1902"},
    {"id": "S09", "group": "sidebar", "label": "Repo disclosure arrow → heading", "native": {"T": "10", "R": "16", "B": "10"}, "proposal": "Keep T 10 / R 16 / B 10", "geometry": {"type": "inset", "outer": ".group-head", "inner": ".group-head .arrow", "mask": "TRB"}, "note": "Native10 ×10 arrow sits4pt farther left than the prototype.", "source": "window.rs:1911"},
    {"id": "S10", "group": "sidebar", "label": "Repo text frame → disclosure arrow", "native": "2", "proposal": "Keep 2", "geometry": {"type": "gap", "a": ".group-head > span", "ae": "R", "b": ".group-head .arrow", "be": "L"}, "note": "Native text-frame gap2; prototype gap includes elastic short-text space.", "source": "window.rs:1888–1889,1911"},
    {"id": "S11", "group": "sidebar", "label": "Repo heading → first workspace row", "native": "2", "proposal": "Keep2 as dense-list exception", "geometry": {"type": "gap", "a": ".group-head", "ae": "B", "b": ".group .row", "be": "T"}, "note": "Heading height30 is a size; this2pt is an actual gap.", "source": "window.rs:1933–1935"},
    {"id": "S12", "group": "sidebar", "label": "Workspace row → next workspace row", "native": "2", "proposal": "Keep2 as dense-list exception", "geometry": {"type": "gap", "a": ".group:nth-child(2) .row:first-child", "ae": "B", "b": ".group:nth-child(2) .row:nth-child(2)", "be": "T"}, "note": "Visible row34, stride36. The same relation applies to all adjacent rows.", "source": "window.rs:1933–1935,1958"},
    {"id": "S13", "group": "sidebar", "label": "Workspace row → catalog sides", "native": {"R": "6", "L": "6"}, "proposal": "Keep R 6 / L 6", "geometry": {"type": "inset", "outer": "#catalog", "inner": ".row", "mask": "RL"}, "note": "Shares S03. Selected background and row hit target use this same outer box.", "source": "window.rs:1933–1935"},
    {"id": "S14", "group": "sidebar", "label": "Workspace text frame → row", "native": {"T": "8", "R": "34", "B": "8", "L": "48"}, "proposal": "Keep T 8 / R 34 / B 8 / L 48", "geometry": {"type": "inset", "outer": ".row", "inner": ".row .label", "mask": "TRBL"}, "note": "13pt font inside18pt native line frame. Short prototype text leaves flexible trailing space.", "source": "window.rs:1940–1943; chrome_view.rs:194–202"},
    {"id": "S15", "group": "sidebar", "label": "Session dot → workspace row", "native": {"T": "14", "B": "14", "L": "32"}, "proposal": "Keep", "geometry": {"type": "inset", "outer": ".row", "inner": ".row .dot", "mask": "TBL"}, "note": "6 ×6 dot; 14pt vertical inset is derived from34pt row height.", "source": "chrome_view.rs:70–78"},
    {"id": "S16", "group": "sidebar", "label": "Session dot → workspace text", "native": "10", "proposal": "Keep 10", "geometry": {"type": "gap", "a": ".row .dot", "ae": "R", "b": ".row .label", "be": "L"}, "note": "Reducing the gap changes text indentation. Dot position remains32.", "source": "chrome_view.rs:70–78; window.rs:1940–1943"},
    {"id": "S17", "group": "sidebar", "label": "Selected marker frame → row", "native": {"T": "8", "R": "9", "B": "8"}, "proposal": "Keep T 8 / R 9 / B 8", "geometry": {"type": "inset", "outer": ".row.on", "inner": ".row.on .check", "mask": "TRB"}, "note": "Native check/shortcut share18 ×18. Prototype check13 ×13 with right12 differs.", "source": "chrome_view.rs:88–99"},
    {"id": "S18", "group": "sidebar", "label": "Shortcut hint → row", "native": {"T": "8", "R": "9", "B": "8"}, "proposal": "Keep T 8 / R 9 / B 8", "geometry": {"type": "inset", "outer": ".row", "inner": ".row .shortcut", "mask": "TRB"}, "note": "Prototype shortcut18 ×18 has right10. Inspect its reserved box even when the hint is not displayed.", "source": "chrome_view.rs:88–99"},
    {"id": "S19", "group": "sidebar", "label": "Workspace text frame → marker", "native": "7", "proposal": "Keep 7", "geometry": {"type": "gap", "a": ".row .label", "ae": "R", "b": ".row .shortcut", "be": "L"}, "note": "Fixed native full-label-frame gap; prototype short-label free space varies.", "source": "chrome_view.rs:88–99; window.rs:1940–1943"},
    {"id": "S20", "group": "sidebar", "label": "Last row → glass bottom", "native": "12 + flexible remainder, pinned", "proposal": "Keep flexible; do not force uniform12", "geometry": {"type": "gap", "a": ".group:last-child .row:last-child", "ae": "B", "b": ".platter", "be": "B"}, "note": "A pinned sidebar fills the window; its catalog does not have to fill the height. Assumes unscrolled content without overflow; scrolling can make this distance negative.", "source": "window.rs:2284–2288,1958"},
    {"id": "S21", "group": "sidebar", "label": "Separator edges and focus outline", "native": "Separator L6/R6, stroke0.5; focus inset2, stroke2", "proposal": "Separator edges follow S03; keep focus visible", "geometry": {"type": "info", "selector": ".group:nth-child(2)"}, "note": "Hairlines and focus rings are drawing geometry, not content padding.", "source": "window.rs:2028–2030; chrome_view.rs:58–64"},
    {"id": "S22", "group": "sidebar", "label": "Resize hit strip → glass", "native": {"T": "−12 pinned", "R": "−4", "B": "−12 pinned", "L": "sidebar width"}, "proposal": "Keep exposed4pt strip", "geometry": {"type": "inset", "outer": ".platter", "inner": ".sidebar-resize", "mask": "TRBL"}, "note": "Native hit strip spans the whole root split height, subject to covering views. Negative insets extend beyond glass. Prototype excludes44px at top and16px at bottom.", "source": "window.rs:2121–2134"},
    {"id": "E01", "group": "terminal", "label": "Each terminal pane: content padding", "native": {"T": "8 + grid remainder", "R": "8 + grid remainder", "B": "8 + grid remainder", "L": "8 + grid remainder"}, "proposal": "Keep8 on all four sides", "geometry": {"type": "padding", "selector": ".pane"}, "note": "Applies to every pane, including split and zoomed surfaces. Ghostty balance remains enabled.", "source": "habits.rs:35–37"},
    {"id": "E02", "group": "terminal", "label": "Tab bottom → terminal surface top", "native": "12", "proposal": "12", "geometry": {"type": "gap", "a": ".tab", "ae": "B", "b": ".pane", "be": "T"}, "note": "Ends at the surface edge. E03 additionally includes the terminal inner padding.", "source": "window.rs:1748–1753,2344–2352"},
    {"id": "E03", "group": "terminal", "label": "Tab bottom → first text line box", "native": "20 + grid remainder", "proposal": "20 + grid remainder", "geometry": {"type": "contentGap", "a": ".tab", "ae": "B", "b": ".pane", "be": "T"}, "note": "Includes the12pt gap below the tab plus8pt surface padding. Glyph ink sits inside a line box.", "source": "window.rs:1748–1753; habits.rs:35–37"},
    {"id": "E04", "group": "terminal", "label": "Pinned glass right → text area left", "native": "20 + grid remainder", "proposal": "20 + grid remainder", "geometry": {"type": "contentGap", "a": ".platter", "ae": "R", "b": ".pane", "be": "L"}, "note": "12pt outer gap plus8pt inner padding. In transient mode the glass overlays the terminal.", "source": "window.rs:2344–2352; habits.rs:35–37"},
    {"id": "E05", "group": "terminal", "label": "Last pane text area → window right", "native": "20 + grid remainder", "proposal": "20 + grid remainder", "geometry": {"type": "contentGap", "a": ".pane:last-child", "ae": "R", "b": "#win", "be": "R", "contentA": true}, "note": "12pt outer gap plus the final pane's8pt inner padding.", "source": "window.rs:2344–2352; habits.rs:35–37"},
    {"id": "E06", "group": "terminal", "label": "Text area bottom → window bottom", "native": "48 with quota; 20 without; plus grid remainder", "proposal": "48 with quota; 20 without, plus remainder", "geometry": {"type": "contentGap", "a": ".pane", "ae": "B", "b": "#win", "be": "B", "contentA": true}, "note": "40pt status allocation plus8pt padding. Without quota, native still reserves12pt outside the surface. Measures the bottommost visible pane in either split direction.", "source": "window.rs:2344–2352; habits.rs:35–37"},
    {"id": "E07", "group": "terminal", "label": "Ghostty grid balance", "native": "Enabled; full cells plus distributed pixel remainder", "proposal": "Keep enabled", "geometry": {"type": "info", "selector": ".pane"}, "note": "8pt is the explicit base, not a promise that each visible edge is exactly8pt. Prototype does not simulate grid balancing.", "source": "vendor/ghostty/src/renderer/size.zig:49–83,279–301"},
    {"id": "E08", "group": "terminal", "label": "Terminal font and line height", "native": "13pt compiled default; cell metrics owned by Ghostty/CoreText", "proposal": "Keep13pt; label prototype line-height as illustrative", "geometry": {"type": "info", "selector": ".pane"}, "note": "Prototype13px/16px differs from the native grid. Padding recommendations do not change font or line height.", "source": "habits.rs:6–9; vendor/ghostty/src/font/metrics.zig:265–283"},
    {"id": "E09", "group": "terminal", "label": "Terminal area size", "native": {"X": "W − sidebar − 36 pinned; W − 24 otherwise", "Y": "H − 100 with quota; H − 72 without"}, "proposal": "Keep surface size; recover space inside", "geometry": {"type": "size", "selector": ".term"}, "note": "At1200 ×780, sidebar300 and quota:864 ×680. The prototype is20px shorter because its window is760px. W/H refer to the root content view, excluding system decoration.", "source": "window.rs:2344–2355"},
    {"id": "P01", "group": "terminal", "label": "Tab root / split container → surface", "native": "0 extra padding on every side", "proposal": "0", "geometry": {"type": "info", "selector": ".term"}, "note": "Surface padding E01 remains separate. Split, zoom and survivor promotion do not add another outer wrapper.", "source": "split.rs:32,56,74,114"},
    {"id": "P02", "group": "terminal", "label": "Pane → adjacent pane", "native": "0 extra gap; AppKit Thin divider", "proposal": "Keep0; retain native Thin", "geometry": {"type": "gap", "a": ".pane:first-child", "ae": "R", "b": ".pane:last-child", "be": "L"}, "note": "Prototype divider is a0.5px inside border. Native divider thickness is system-owned; no fixed thickness is declared.", "source": "split.rs:77"},
    {"id": "P03", "group": "terminal", "label": "Text area → text area across split", "native": "16 + Thin divider + grid remainders", "proposal": "16 + Thin + remainders", "geometry": {"type": "splitContent"}, "note": "Each pane contributes its own8pt padding. The equivalent vertical split follows the same rule.", "source": "habits.rs:35–37; split.rs:77"},
    {"id": "P04", "group": "terminal", "label": "No quota: surface → window bottom", "native": "12", "proposal": "Keep12; align prototype", "geometry": {"type": "info", "selector": ".term"}, "note": "In the current No quota snapshot scene, the prototype leaves0 outside the surface. This is a recorded mismatch, not an applied change.", "source": "window.rs:2347"},
    {"id": "Q01", "group": "quota", "label": "Quota chip → window bottom", "native": {"B": "12"}, "proposal": "12", "geometry": {"type": "inset", "outer": "#win", "inner": "#quota", "mask": "B"}, "note": "The chip shares the window bottom inset.", "source": "quota_panel.rs:100–122"},
    {"id": "Q02", "group": "quota", "label": "Terminal surface left → quota left", "native": "0", "proposal": "Keep 0", "geometry": {"type": "gap", "a": ".term", "ae": "L", "b": "#quota", "be": "L"}, "note": "Summary text starts8pt from the terminal area edge, matching its base text inset. Ghostty grid balance can add remainder inside each pane.", "source": "quota_panel.rs:109"},
    {"id": "Q03", "group": "quota", "label": "Terminal surface bottom → closed chip top", "native": "0", "proposal": "0", "geometry": {"type": "gap", "a": ".term", "ae": "B", "b": "#quota-trigger", "be": "T"}, "note": "28pt chip +12pt bottom inset uses40pt. Expansion covers the terminal without increasing the reserve.", "source": "quota_panel.rs:21–24,499–511"},
    {"id": "Q04", "group": "quota", "label": "Quota chip size", "native": {"X": "measured summary text + 16", "Y": "28"}, "proposal": "Keep X measured summary text + 16 / Y 28", "geometry": {"type": "size", "selector": "#quota-trigger"}, "note": "Prototype fixes width176 for two providers or92 for one. Native width is content-derived. Horizontal text space is preserved; the summary hit target is12pt narrower.", "source": "quota_panel.rs:381–397"},
    {"id": "Q05", "group": "quota", "label": "Summary horizontal padding", "native": {"R": "8", "L": "8"}, "proposal": "Keep R 8 / L 8", "geometry": {"type": "padding", "selector": "#quota-trigger", "mask": "RL"}, "note": "CSS top/bottom padding0 is not the vertical text gap. Q06 measures the line box separately.", "source": "quota_panel.rs:381–397; chrome_view.rs:194–202"},
    {"id": "Q06", "group": "quota", "label": "Summary text line → chip top / bottom", "native": {"T": "5", "B": "5"}, "proposal": "Keep18pt line → 5pt top/bottom", "geometry": {"type": "inset", "outer": "#quota-trigger", "inner": ".quota-trigger > span", "mask": "TB"}, "note": "Prototype line box is16.8px, giving5.6px above/below. Native line frame is18pt.", "source": "chrome_view.rs:194–202"},
    {"id": "Q07", "group": "quota", "label": "Provider name → percentage", "native": "2 font spaces", "proposal": "Keep 2 font spaces", "geometry": {"type": "textGap", "selector": ".quota-trigger > span"}, "note": "Rendered prototype text-box gap; Q08 shows the declared CSS gap. Native uses2 spaces inside a single string.", "source": "quota.rs:145"},
    {"id": "Q08", "group": "quota", "label": "Provider name → percentage gap", "native": "2 font spaces", "proposal": "Keep 2 font spaces", "geometry": {"type": "css", "selector": ".quota-trigger > span", "property": "columnGap"}, "note": "Native does not define6pt. HTML explicitly defines6px. Measure a name/number pair, not the whole string.", "source": "quota.rs:145"},
    {"id": "Q09", "group": "quota", "label": "Provider block → next provider block", "native": "3 font spaces", "proposal": "Keep 3 font spaces", "geometry": {"type": "gap", "a": ".quota-trigger > span:first-child", "ae": "R", "b": ".quota-trigger > span:last-child", "be": "L"}, "note": "Only applicable with two providers. HTML explicitly uses18px. Native uses3 spaces in a single string.", "source": "quota_panel.rs:385"},
    {"id": "Q10", "group": "quota", "label": "Expanded summary → glass right", "native": "304 − content-derived chip width", "proposal": "Keep flexible", "geometry": {"type": "gap", "a": "#quota-trigger", "ae": "R", "b": "#quota", "be": "R"}, "note": "Prototype176px summary leaves128px when expanded. This is free panel space, not summary padding.", "source": "quota_panel.rs:499–507"},
    {"id": "Q11", "group": "quota", "label": "Details → summary seam", "native": "0", "proposal": "0", "geometry": {"type": "gap", "a": "#quota-details", "ae": "B", "b": "#quota-trigger", "be": "T"}, "note": "Same glass surface; no separate gap or divider.", "source": "quota_panel.rs:547–549"},
    {"id": "Q12", "group": "quota", "label": "Details padding", "native": {"T": "16", "R": "16", "B": "12", "L": "16"}, "proposal": "Keep T 16 / R 16 / B 12 / L 16", "geometry": {"type": "padding", "selector": "#quota-details"}, "note": "Details height is28 + sum(26 + row count ×30) +16 between providers. This includes all rows and the bottom inset.", "source": "quota_panel.rs:540–563"},
    {"id": "Q13", "group": "quota", "label": "Window column → remaining column", "native": "8", "proposal": "Keep 8", "geometry": {"type": "gap", "a": ".quota-row > span:first-child", "ae": "R", "b": ".quota-row > strong", "be": "L"}, "note": "Three native columns are50 /132 /74 wide, with8pt gaps inside272pt.", "source": "quota_panel.rs:573–596"},
    {"id": "Q14", "group": "quota", "label": "Remaining column → reset column", "native": "8", "proposal": "Keep 8", "geometry": {"type": "gap", "a": ".quota-row > strong", "ae": "R", "b": ".quota-row > .quota-reset", "be": "L"}, "note": "Same horizontal gap definition as Q13.", "source": "quota_panel.rs:573–596"},
    {"id": "Q15", "group": "quota", "label": "Quota data row size", "native": {"X": "272 inner width", "Y": "30"}, "proposal": "Keep X 272 inner width / Y 30", "geometry": {"type": "size", "selector": ".quota-row"}, "note": "Row size, not its text padding. Neighbouring row boxes have no gap.", "source": "quota_panel.rs:573–602"},
    {"id": "Q16", "group": "quota", "label": "Quota row text → row top / bottom", "native": {"T": "6", "B": "6"}, "proposal": "Keep18pt line → 6pt top/bottom", "geometry": {"type": "inset", "outer": ".quota-row", "inner": ".quota-row > span:first-child", "mask": "TB"}, "note": "For the percentage the prototype has a slightly different line height. Native all columns use18pt frames.", "source": "quota_panel.rs:573–596"},
    {"id": "Q17", "group": "quota", "label": "Quota row → next row", "native": "0", "proposal": "0", "geometry": {"type": "gap", "a": ".quota-provider:first-child .quota-row:first-of-type", "ae": "B", "b": ".quota-provider:first-child .quota-row:last-child", "be": "T"}, "note": "Text-to-text distance adds both row insets:6 +0 +6 =12pt.", "source": "quota_panel.rs:602"},
    {"id": "Q18", "group": "quota", "label": "Provider heading → first row box", "native": "8", "proposal": "Keep 8", "geometry": {"type": "gap", "a": ".quota-provider h2", "ae": "B", "b": ".quota-row", "be": "T"}, "note": "Native heading18pt; first row text starts6pt farther inside that row.", "source": "quota_panel.rs:563–573"},
    {"id": "Q19", "group": "quota", "label": "Provider group → next heading", "native": "16", "proposal": "16", "geometry": {"type": "gap", "a": ".quota-provider:first-child .quota-row:last-child", "ae": "B", "b": ".quota-provider:last-child h2", "be": "T"}, "note": "Last row text-to-heading distance is22pt, including6pt row bottom inset.", "source": "quota_panel.rs:604"},
    {"id": "Q20", "group": "quota", "label": "Quota corners / expansion sizing", "native": "14 radius closed and expanded; detail width304", "proposal": "Keep", "geometry": {"type": "info", "selector": "#quota"}, "note": "Same background and corner radius in both states. These are geometry values, not spacing.", "source": "quota_panel.rs:23–24,69,499–507"},
    {"id": "F01", "group": "find", "label": "Find bar → pane edges", "native": {"T": "6", "R": "6", "B": "Flexible", "L": "Flexible"}, "proposal": "Keep T 6 / R 6 / B Flexible / L Flexible", "geometry": {"type": "inset", "outer": ".pane", "inner": ".find-bar", "mask": "TRBL"}, "note": "The prototype currently uses top8/right12. The native bar is overlaid in the focused pane.", "source": "find_bar.rs:104–110"},
    {"id": "F02", "group": "find", "label": "Find bar size", "native": {"X": "max(0, min(340, initial pane width − 12))", "Y": "38"}, "proposal": "Keep X max(0, min(340, initial pane width − 12)) / Y 38", "geometry": {"type": "size", "selector": ".find-bar"}, "note": "Prototype is content-sized. Control height is not an external gap. Native width is computed at creation and not recomputed when that pane is later resized.", "source": "find_bar.rs:19–20,104–110"},
    {"id": "F03", "group": "find", "label": "Search field → bar edges", "native": {"T": "7", "B": "7", "L": "6"}, "proposal": "Keep T 7 / B 7 / L 6", "geometry": {"type": "inset", "outer": ".find-bar", "inner": ".find-bar input", "mask": "TBL"}, "note": "Native nominal field192 ×24. Browser120px input has different intrinsic padding.", "source": "find_bar.rs:123–132"},
    {"id": "F04", "group": "find", "label": "Search field → count frame", "native": "0", "proposal": "Keep 0", "geometry": {"type": "gap", "a": ".find-bar input", "ae": "R", "b": ".find-bar [data-count]", "be": "L"}, "note": "A true0 frame gap in native AppKit. HTML flex gap8 is not native reality.", "source": "find_bar.rs:131–147"},
    {"id": "F05", "group": "find", "label": "Count frame → bar top / bottom", "native": {"T": "10", "B": "10"}, "proposal": "Keep T 10 / B 10", "geometry": {"type": "inset", "outer": ".find-bar", "inner": ".find-bar [data-count]", "mask": "TB"}, "note": "Native64 ×18 frame. Actual number ink leaves content-dependent side space.", "source": "find_bar.rs:141–147"},
    {"id": "F06", "group": "find", "label": "Count frame → Previous target", "native": "0", "proposal": "Keep 0", "geometry": {"type": "gap", "a": ".find-bar [data-count]", "ae": "R", "b": ".find-bar button:first-of-type", "be": "L"}, "note": "Prototype has8px flex gap.", "source": "find_bar.rs:155"},
    {"id": "F07", "group": "find", "label": "Previous target → Next target", "native": "0", "proposal": "Keep 0", "geometry": {"type": "gap", "a": ".find-bar button:first-of-type", "ae": "R", "b": ".find-bar button:nth-of-type(2)", "be": "L"}, "note": "Native targets each22 ×38 with no inter-button gaps. Preserve the current field and target widths.", "source": "find_bar.rs:163–177"},
    {"id": "F08", "group": "find", "label": "Next target → Close target", "native": "0", "proposal": "Keep 0", "geometry": {"type": "gap", "a": ".find-bar button:nth-of-type(2)", "ae": "R", "b": ".find-bar button:last-of-type", "be": "L"}, "note": "Same relationship as F07.", "source": "find_bar.rs:163–177"},
    {"id": "F09", "group": "find", "label": "Previous target → bar top / bottom", "native": {"T": "0", "B": "0"}, "proposal": "Keep T 0 / B 0", "geometry": {"type": "inset", "outer": ".find-bar", "inner": ".find-bar button:first-of-type", "mask": "TB"}, "note": "Native target spans38pt bar height. Prototype button uses browser intrinsic sizing.", "source": "find_bar.rs:163–177"},
    {"id": "F10", "group": "find", "label": "Close target → bar right edge", "native": {"R": "12"}, "proposal": "Keep R 12", "geometry": {"type": "inset", "outer": ".find-bar", "inner": ".find-bar button:last-of-type", "mask": "R"}, "note": "The current native left/right outer insets are6/12.", "source": "find_bar.rs:124,155–177"},
    {"id": "F11", "group": "find", "label": "Find action glyph line frame", "native": "T10 R0 B10 L6, line18, font14", "proposal": "Keep T10 R0 B10 L6, line18, font14", "geometry": {"type": "info", "selector": ".find-bar button"}, "note": "Text-frame insets are separate from the22 ×38 target. Prototype browser defaults differ.", "source": "find_bar.rs:165–175; chrome_view.rs:194–202"},
    {"id": "F12", "group": "find", "label": "Search icon / editable text / clear button", "native": "System-owned; no fixed internal pt values", "proposal": "Keep System-owned; no fixed internal pt values", "geometry": {"type": "info", "selector": ".find-bar input"}, "note": "NSSearchField and browser type=search have different internal geometry. Do not invent numeric parity.", "source": "find_bar.rs:128–140"},
    {"id": "A01", "group": "system", "label": "Close / quit confirmation internals", "native": "AppKit NSAlert owns padding and button layout", "proposal": "Keep system layout", "geometry": {"type": "info", "selector": "#close-confirm"}, "note": "Prototype dialog padding24, action gap8, action top margin20 are illustrative browser values.", "source": "window.rs:1017–1028"},
    {"id": "A02", "group": "system", "label": "Directory picker internals", "native": "AppKit NSOpenPanel owns layout", "proposal": "Keep system layout", "geometry": {"type": "info", "selector": "#add"}, "note": "The prototype toast is only a placeholder; it is not the native picker layout.", "source": "window.rs:dispatch(Click::AddRepo)"},
    {"id": "A03", "group": "system", "label": "Menu items / menu margins / traffic-light ink", "native": "AppKit owns internal geometry", "proposal": "Keep system layout", "geometry": {"type": "info", "selector": ".traffic"}, "note": "Native menus, tooltips and system control glyphs do not have Combe spacing constants.", "source": "window.rs:install_menu,icon_button"},
    {"id": "R01", "group": "radii", "label": "Window content clip", "native": "34; 0 in fullscreen", "proposal": "Keep34 / fullscreen0", "geometry": {"type": "radius", "selector": "#win"}, "note": "Native custom content clipping, not a replacement for system window controls. The prototype right-side wrapper inherits34 but paints no separate rounded surface.", "source": "window.rs:47,699–705,2221–2233"},
    {"id": "R02", "group": "radii", "label": "Sidebar glass: chip / transient / pinned", "native": "18", "proposal": "Keep18 in all three states", "geometry": {"type": "radius", "selector": ".platter"}, "note": "The outer sidebar layer and material use the same radius. A36pt chip is a capsule; expansion keeps its corner radius.", "source": "window.rs:48,707–715"},
    {"id": "R03", "group": "radii", "label": "Workspace trigger background", "native": "No independent fill", "proposal": "Keep no independent fill", "geometry": {"type": "radius", "selector": "#trigger"}, "note": "Native hover highlight is disabled. CSS18 is a transparent button shape, not a second visible glass surface. Focus is listed under R17.", "source": "window.rs:719–739"},
    {"id": "R04", "group": "radii", "label": "Repo-heading highlight", "native": "Requested16; rx16 / ry15 at height30", "proposal": "Keep current shape", "geometry": {"type": "radius", "selector": ".group-head", "catalog": true}, "note": "AppKit limits the two arc axes independently; the uniform CSS16 corners scale to15 at height30. This is a current renderer difference, not two design tokens.", "source": "chrome_view.rs:13,43–53; window.rs:1883–1889"},
    {"id": "R05", "group": "radii", "label": "Workspace-row selected / hover fill", "native": "16", "proposal": "Keep16", "geometry": {"type": "radius", "selector": ".row", "catalog": true}, "note": "Rows are34pt high. Selection and hover share this path. HTML uses the radius-row16 token for both list rows and repo headings.", "source": "chrome_view.rs:13,43–53; window.rs:1933–1945"},
    {"id": "R06", "group": "radii", "label": "Active-tab glass", "native": "18", "proposal": "Keep18", "geometry": {"type": "radius", "selector": ".tab.on"}, "note": "Only the active native tab has this glass sibling. It is not the same layer as the hover highlight.", "source": "window.rs:1753–1763"},
    {"id": "R07", "group": "radii", "label": "Tab hover highlight", "native": "18", "proposal": "Keep18 with active-tab glass", "geometry": {"type": "radius", "selector": ".tab"}, "note": "Tab titles set their ClickView radius to18, matching the glass. Other ClickViews retain16.", "source": "chrome_view.rs:43–53; window.rs:1753–1765"},
    {"id": "R08", "group": "radii", "label": "Tab Close target background", "native": "No independent fill", "proposal": "Keep no independent fill", "geometry": {"type": "radius", "selector": ".tab .x"}, "note": "Native Close is20×36 and disables hover fill. The transparent HTML22×22 button declares50%;11 is not a native radius. Focus is R17.", "source": "window.rs:1766–1779"},
    {"id": "R09", "group": "radii", "label": "New-tab target background", "native": "No independent fill", "proposal": "Keep no independent fill", "geometry": {"type": "radius", "selector": ".newtab"}, "note": "Native36×36 target disables hover fill. HTML declares18 on a transparent button. Focus is R17.", "source": "window.rs:1783–1793"},
    {"id": "R10", "group": "radii", "label": "Add / Pin button shape", "native": "AppKit; no explicit Combe radius", "proposal": "Keep native button ownership", "geometry": {"type": "radius", "selector": "#pin"}, "note": "Both are28×28 borderless NSButtons. That does not establish a native14pt circle. The prototype explicitly uses50%.", "source": "window.rs:820–837"},
    {"id": "R11", "group": "radii", "label": "Session dot", "native": "Circle, diameter6 / radius3", "proposal": "Keep circle, radius3", "geometry": {"type": "radius", "selector": ".dot", "catalog": true}, "note": "Drawn with an oval in a square frame. A size-derived circle is not a general rounded-rectangle token.", "source": "chrome_view.rs:14,66–80"},
    {"id": "R12", "group": "radii", "label": "Command shortcut badge", "native": "Circle, diameter18 / radius9", "proposal": "Keep circle, radius9", "geometry": {"type": "radius", "selector": ".shortcut", "catalog": true}, "note": "The badge background exists only while shortcut hints are shown. Its geometry is reserved while hidden; selecting this ID measures that reserved shape.", "source": "chrome_view.rs:85–94"},
    {"id": "R13", "group": "radii", "label": "Quota glass: closed / expanded", "native": "14", "proposal": "Keep14 in both states", "geometry": {"type": "radius", "selector": "#quota", "scene": "workspace", "quota": true}, "note": "The same glass panel grows upward. A28pt chip is a capsule; changing only the expanded radius would introduce a separate shape transition.", "source": "quota_panel.rs:69"},
    {"id": "R14", "group": "radii", "label": "Quota summary button background", "native": "No independent fill", "proposal": "Keep no independent fill", "geometry": {"type": "radius", "selector": "#quota-trigger", "scene": "workspace"}, "note": "The native summary disables hover fill; the14pt visible surface belongs to R13. HTML14 on the transparent trigger is not another painted layer.", "source": "quota_panel.rs:392–408"},
    {"id": "R15", "group": "radii", "label": "Find bar glass", "native": "12", "proposal": "Keep12", "geometry": {"type": "radius", "selector": ".find-bar", "scene": "find"}, "note": "The material has radius12, but it is a sibling of the field and action buttons. Its clipping does not clip those sibling controls. Native height38; prototype height is content-derived.", "source": "find_bar.rs:117–122"},
    {"id": "R16", "group": "radii", "label": "Find Previous / Next / Close hover", "native": "Requested16; rx11 / ry16 at22×38", "proposal": "Keep for now; any new shape needs a focused visual decision", "geometry": {"type": "radius", "selector": ".find-bar button", "scene": "find"}, "note": "All three native actions use the shared ClickView fill. HTML buttons have no rounded hover surface. This is not the12pt exterior glass.", "source": "find_bar.rs:155–177; chrome_view.rs:43–53"},
    {"id": "R17", "group": "radii", "label": "ClickView keyboard focus ring", "native": "Tab16; other controls request14; inset2; stroke2", "proposal": "Keep per-control focus shapes", "geometry": {"type": "info", "selector": "#win", "text": "Tab outline2, offset−3 on18pt host; other CSS outlines follow their host"}, "note": "The focus path uses the instance corner radius minus2. Tabs use16; all other ClickViews retain14 before per-axis bounds clipping. Frames, hit tests and focus actions are unchanged.", "source": "chrome_view.rs:54–61; window.rs:719–739,1753–1793; quota_panel.rs:392–408; find_bar.rs:163–177"},
    {"id": "R18", "group": "radii", "label": "Terminal area / pane / split surfaces", "native": "0 independent corner radius", "proposal": "Keep0", "geometry": {"type": "radius", "selector": ".pane", "scene": "workspace"}, "note": "No rounded terminal cards. Panes and split containers rely on the outer window clip; split dividers have no rounded surface.", "source": "surface.rs; split.rs; docs/DESIGN.md:66"},
    {"id": "R19", "group": "radii", "label": "Native search-field bezel", "native": "AppKit owns the shape", "proposal": "Keep NSSearchField", "geometry": {"type": "radius", "selector": ".find-bar input", "scene": "find"}, "note": "Combe creates NSSearchField without setting its corner radius. HTML input6 is an illustrative browser control.", "source": "find_bar.rs:128–140"},
    {"id": "R20", "group": "radii", "label": "Traffic lights", "native": "AppKit owns the shape", "proposal": "Keep system controls", "geometry": {"type": "radius", "selector": ".traffic i"}, "note": "The HTML12×12 circle resolves to radius6. It is not a fixed Combe system-button radius.", "source": "window.rs:2365–2380"},
    {"id": "R21", "group": "radii", "label": "Close / quit confirmation surface", "native": "AppKit NSAlert", "proposal": "Keep system dialog", "geometry": {"type": "info", "selector": "#win", "text": "Illustrative CSS18; inspect the Close confirmation scene separately"}, "note": "The HTML confirmation radius18 is a visual placeholder, not a native NSAlert constant.", "source": "window.rs:1017–1028"},
    {"id": "R22", "group": "radii", "label": "Confirmation action buttons", "native": "AppKit NSAlert buttons", "proposal": "Keep system buttons", "geometry": {"type": "info", "selector": "#win", "text": "Illustrative CSS7; inspect the Close confirmation scene separately"}, "note": "HTML7 is illustrative. Combe does not assign a radius to native confirmation buttons.", "source": "window.rs:1017–1028"},
    {"id": "R23", "group": "radii", "label": "Picker / menus / scrollbars / symbol curves", "native": "AppKit owns internal geometry", "proposal": "Keep system ownership", "geometry": {"type": "info", "selector": "#win", "text": "Not numerically simulated"}, "note": "NSOpenPanel, menu items, native scrollbars and SF Symbol ink have no Combe radius token. Text glyph curves, including the selected checkmark, are not rounded component backgrounds.", "source": "window.rs:install_menu,dispatch(Click::AddRepo),icon_button"},
    {"id": "R24", "group": "radii", "label": "Prototype toast", "native": "No native counterpart", "proposal": "Exclude from native radius scale", "geometry": {"type": "info", "selector": "#win", "text": "CSS10"}, "note": "The repo-picker placeholder toast exists only in the interactive reference. It does not define a native notification component.", "source": "docs/design.html:.toast"},
    {"id": "R25", "group": "radii", "label": "Prototype selectors / inspection controls", "native": "Outside the native app", "proposal": "Exclude from native radius scale", "geometry": {"type": "info", "selector": "#win", "text": "CSS6"}, "note": "Appearance, Component and measurement controls belong to the reference page, not the Combe window.", "source": "docs/design.html:select,input,.spacing-controls button"},
    {"id": "R26", "group": "radii", "label": "Measurement labels", "native": "Outside the native app", "proposal": "Exclude from native radius scale", "geometry": {"type": "info", "selector": "#win", "text": "SVG rx3"}, "note": "Annotation labels exist only in the inspector. Shadow blur and stroke width are separate quantities, not corner radii.", "source": "docs/design.spacing.js:draw"}
  ];
  const stage = document.querySelector('.stage');
  const toggle = document.getElementById('spacing-toggle');
  const inspector = document.getElementById('spacing-inspector');
  const area = document.getElementById('spacing-area');
  const sidebar = document.getElementById('spacing-sidebar');
  const picker = document.getElementById('spacing-metric');
  const toolbar = document.getElementById('spacing-toolbar');
  const table = document.getElementById('spacing-rows');
  const selection = document.getElementById('spacing-selection');
  const svgNS = 'http://www.w3.org/2000/svg';
  const overlay = document.createElementNS(svgNS, 'svg');
  overlay.classList.add('spacing-overlay');
  overlay.setAttribute('viewBox', '0 0 1200 760');
  overlay.setAttribute('aria-hidden', 'true');
  overlay.setAttribute('hidden', '');
  stage.append(overlay);
  let selected = 'E01';
  let frame = 0;
  let animateUntil = 0;

  function resolve(selector) {
    const candidates = [...document.querySelectorAll(selector)];
    if (selector === '.pane:last-child' && !candidates.some(visible)) return [...document.querySelectorAll('.pane')].filter(visible).at(-1);
    return candidates.find(visible);
  }

  function visible(element) {
    if (!element.getClientRects().length || element.closest('[hidden]')) return false;
    if (element.closest('#catalog') && mode === 'closed') return false;
    if (element.closest('#quota-details') && !quota.classList.contains('open')) return false;
    return true;
  }

  function rect(element, content = false) {
    const origin = win.getBoundingClientRect();
    const scale = origin.width / 1200;
    const box = element.getBoundingClientRect();
    const result = {L: (box.left - origin.left) / scale, R: (box.right - origin.left) / scale, T: (box.top - origin.top) / scale, B: (box.bottom - origin.top) / scale};
    if (content) {
      const style = getComputedStyle(element);
      result.L += parseFloat(style.paddingLeft) + parseFloat(style.borderLeftWidth);
      result.R -= parseFloat(style.paddingRight) + parseFloat(style.borderRightWidth);
      result.T += parseFloat(style.paddingTop) + parseFloat(style.borderTopWidth);
      result.B -= parseFloat(style.paddingBottom) + parseFloat(style.borderBottomWidth);
    }
    return result;
  }

  function segment(key, a, b, first, last) {
    const horizontal = first === 'L' || first === 'R';
    const cross = horizontal ? (Math.max(a.T, b.T) + Math.min(a.B, b.B)) / 2 : (Math.max(a.L, b.L) + Math.min(a.R, b.R)) / 2;
    return {key, value: b[last] - a[first], x1: horizontal ? a[first] : cross, y1: horizontal ? cross : a[first], x2: horizontal ? b[last] : cross, y2: horizontal ? cross : b[last], horizontal};
  }

  function measure(metric) {
    const g = metric.geometry;
    if (g.type === 'radius') {
      const element = resolve(g.selector);
      if (!element) return null;
      const box = rect(element), style = getComputedStyle(element);
      const declared = style.borderTopLeftRadius;
      const radius = Math.min(parseFloat(declared) * (declared.endsWith('%') ? (box.R - box.L) / 100 : 1), (box.R - box.L) / 2, (box.B - box.T) / 2);
      return {text: 'CSS ' + declared + '; rendered circular radius ' + number(radius), boxes: [{...box,radius}], segments: [
        {key:'R',value:radius,x1:box.L,y1:box.T+radius,x2:box.L+radius,y2:box.T+radius,horizontal:true}
      ]};
    }
    if (['W03','E04','S22'].includes(metric.id) && mode !== 'pinned') return null;
    if (metric.id === 'P04') {
      if (!document.querySelector('.status-line').hidden) return null;
      const a = rect(document.querySelector('.term')), b = rect(win);
      return {segments:[segment('',a,b,'B','B')],boxes:[]};
    }
    if (g.type === 'info') {
      const element = resolve(g.selector);
      const values = {W07: 'Window34 / sidebar18', S21: 'Separator0.5; focus inset3 / width2', E07: 'Not simulated', E08: '13 / line-height16', P01: 'Grid gap0', A01: 'Dialog padding24; actions gap8 / top20', A02: 'Toast placeholder', A03: '12px traffic circles'};
      return {text: g.text || values[metric.id] || 'See note; different control structure', segments: [], boxes: g.text ? [] : element ? [rect(element)] : []};
    }
    if (g.type === 'inset') {
      const outer = resolve(g.outer), inner = resolve(g.inner);
      if (!outer || !inner) return null;
      const a = rect(outer), b = rect(inner);
      const segments = [...g.mask].map(edge => {
        const forward = edge === 'T' || edge === 'L';
        return segment(edge, forward ? a : b, forward ? b : a, edge, edge);
      });
      return {segments, boxes: [b]};
    }
    if (g.type === 'padding') {
      const element = resolve(g.selector);
      if (!element) return null;
      const a = rect(element), b = rect(element, true);
      return {segments: [...(g.mask || 'TRBL')].map(edge => {
        const forward = edge === 'T' || edge === 'L';
        return segment(edge, forward ? a : b, forward ? b : a, edge, edge);
      }), boxes: [b]};
    }
    if (g.type === 'size') {
      const element = resolve(g.selector);
      if (!element) return null;
      const a = rect(element);
      return {segments: [segment('X', a, a, 'L', 'R'), segment('Y', a, a, 'T', 'B')], boxes: [a]};
    }
    if (g.type === 'splitContent' || metric.id === 'P02') {
      const panes = [...document.querySelectorAll('.pane')].filter(visible);
      if (panes.length < 2) return null;
      const a = rect(panes[0], g.type === 'splitContent'), b = rect(panes[1], g.type === 'splitContent');
      const vertical = document.querySelector('.term').classList.contains('down');
      return {segments: [segment('', a, b, vertical ? 'B' : 'R', vertical ? 'T' : 'L')], boxes: [a, b]};
    }
    if (g.type === 'textGap' || g.type === 'css') {
      const parent = resolve(g.selector || '.quota-trigger > span');
      if (!parent) return null;
      const text = [...parent.childNodes].find(node => node.nodeType === Node.TEXT_NODE);
      const value = parent.querySelector('b');
      if (!text || !value) return null;
      const range = document.createRange();
      range.selectNodeContents(text);
      const origin = win.getBoundingClientRect(), scale = origin.width / 1200;
      const bounds = range.getBoundingClientRect();
      const a = {L: (bounds.left - origin.left) / scale, R: (bounds.right - origin.left) / scale, T: (bounds.top - origin.top) / scale, B: (bounds.bottom - origin.top) / scale};
      const result = segment('', a, rect(value), 'R', 'L');
      if (g.type === 'css') result.value = parseFloat(getComputedStyle(parent)[g.property]);
      return {segments: [result], boxes: []};
    }
    const first = metric.id === 'E06' ? [...document.querySelectorAll('.pane')].filter(visible).sort((a,b) => rect(b).B - rect(a).B)[0] : resolve(g.a), last = resolve(g.b);
    if (!first || !last || (metric.id === 'Q09' && first === last)) return null;
    const a = rect(first, g.type === 'contentGap' && g.contentA);
    const b = rect(last, g.type === 'contentGap' && !g.contentA);
    return {segments: [segment('', a, b, g.ae, g.be)], boxes: []};
  }

  function valueText(value) {
    if (typeof value !== 'object') return String(value);
    return Object.entries(value).map(([key, item]) => key + ' ' + item).join(' · ');
  }

  function number(value) {
    return String(Math.round(value * 10) / 10);
  }

  function measurementText(result) {
    if (!result) return 'Hidden or not applicable';
    return result.text || result.segments.map(s => (s.key ? s.key + ' ' : '') + number(s.value)).join(' · ');
  }

  function nativeValue(metric, segment) {
    return typeof metric.native === 'object' ? metric.native[segment.key] : metric.native;
  }

  function svg(name, attributes, text) {
    const node = document.createElementNS(svgNS, name);
    for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, value);
    if (text !== undefined) node.textContent = text;
    overlay.append(node);
    return node;
  }

  function draw(metric, result, occupied) {
    if (!result) return;
    for (const box of result.boxes) svg('rect', {x: box.L, y: box.T, width: Math.max(0, box.R - box.L), height: Math.max(0, box.B - box.T), rx:box.radius || 0, class: 'measure-box'});
    for (const s of result.segments) {
      if ([s.x1,s.x2,s.y1,s.y2].some(n => !Number.isFinite(n))) continue;
      const line = s.horizontal ? 'M' + s.x1 + ',' + (s.y1 - 3) + 'v6 M' + s.x1 + ',' + s.y1 + 'H' + s.x2 + ' M' + s.x2 + ',' + (s.y2 - 3) + 'v6' : 'M' + (s.x1 - 3) + ',' + s.y1 + 'h6 M' + s.x1 + ',' + s.y1 + 'V' + s.y2 + ' M' + (s.x2 - 3) + ',' + s.y2 + 'h6';
      svg('path', {d: line, class: 'measure-line'});
      const native = nativeValue(metric, s);
      const numeric = typeof native === 'string' && /^\d+(\.\d+)?$/.test(native) ? Number(native) : null;
      const suffix = native !== undefined && (numeric === null || Math.abs(numeric - s.value) > .1) ? ' · N ' + (String(native).length > 16 ? 'see table' : native) : '';
      const text = metric.id + (s.key ? '.' + s.key : '') + ' ' + number(s.value) + suffix;
      const width = Math.min(260, text.length * 5.9 + 12);
      const midX = (s.x1 + s.x2) / 2, midY = (s.y1 + s.y2) / 2;
      let x = Math.max(3, Math.min(1197 - width, midX - width / 2));
      let y = Math.max(3, Math.min(737, midY - 21));
      for (let step = 0; step < 80; step++) {
        if (!occupied.some(b => x < b.x + b.width + 3 && x + width + 3 > b.x && y < b.y + 22 && y + 22 > b.y)) break;
        y += 23;
        if (y > 737) { y = 3; x = (x + width + 10) % Math.max(1, 1197 - width); }
      }
      occupied.push({x,y,width});
      svg('line', {x1: midX, y1: midY, x2: x + width / 2, y2: y + 10, class: 'measure-leader'});
      svg('rect', {x,y,width,height:20,rx:3,class:'measure-label'});
      svg('text', {x:x+6,y:y+14}, text);
    }
  }

  function reveal(metric) {
    if (metric.geometry.scene) {
      const component = document.getElementById('component');
      component.value = metric.geometry.scene;
      component.dispatchEvent(new Event('change'));
    }
    if (metric.geometry.catalog && mode === 'closed') setMode('pinned');
    if (metric.geometry.quota) setQuotaOpen(true);
    if (metric.group === 'sidebar' && mode === 'closed') setMode('pinned');
    if (metric.group === 'quota' && metric.id >= 'Q10') setQuotaOpen(true);
    if (metric.id === 'P04') {
      const component = document.getElementById('component');
      component.value = 'none';
      renderTabs();
    }
    if (metric.group === 'find') {
      const component = document.getElementById('component');
      if (component.value !== 'find') { component.value = 'find'; renderTabs(); }
    }
    if (metric.group === 'system' && metric.id === 'A01') {
      selection.textContent = metric.id + ': Native system dialog internals are not fixed by Combe. Browser values are illustrative.';
    }
  }

  function renderTable() {
    const filtered = metrics.filter(m => area.value === 'all' || m.group === area.value);
    table.replaceChildren();
    picker.replaceChildren();
    picker.add(new Option('Area overview', ''));
    for (const metric of filtered) {
      picker.add(new Option(metric.id + ' · ' + metric.label, metric.id));
      const tr = document.createElement('tr');
      tr.dataset.metric = metric.id;
      tr.setAttribute('aria-selected', String(selected === metric.id));
      const id = document.createElement('td'), button = document.createElement('button');
      button.type = 'button';
      button.textContent = metric.id;
      button.setAttribute('aria-label', 'Measure ' + metric.id + ': ' + metric.label);
      button.addEventListener('click', () => {
        selected = metric.id;
        reveal(metric);
        selection.textContent = metric.id + ' · ' + metric.label + '. Request format: “' + metric.id + ' [edge] → [value] pt; [state or all states].” ' + metric.note;
        renderTable();
        schedule();
      });
      id.append(button);
      const label = document.createElement('td');
      label.textContent = metric.label;
      const source = document.createElement('small');
      source.textContent = metric.source;
      label.append(source);
      const native = document.createElement('td'); native.textContent = valueText(metric.native);
      const prototype = document.createElement('td'); prototype.className = 'spacing-value';
      const proposal = document.createElement('td'); proposal.textContent = metric.proposal;
      tr.append(id,label,native,prototype,proposal);
      table.append(tr);
    }
    picker.value = selected;
  }

  function render() {
    frame = 0;
    if (!toggle.checked) return;
    sidebar.value = mode;
    overlay.replaceChildren();
    const measured = new Map(metrics.map(m => [m.id, measure(m)]));
    const active = metrics.find(m => m.id === selected);
    document.getElementById('spacing-active').textContent = active ? active.id + ' · Native: ' + valueText(active.native) + ' pt | Prototype: ' + measurementText(measured.get(active.id)) + ' px | Proposed: ' + active.proposal : 'Overview. Select a measurement for its exact native and prototype values.';
    for (const row of table.rows) row.querySelector('.spacing-value').textContent = measurementText(measured.get(row.dataset.metric));
    const overview = new Set(['W01','W02','H11','T03','S03','E01','Q12','F01']);
    const plotted = metrics.filter(m => selected ? m.id === selected : area.value === 'all' ? overview.has(m.id) : m.group === area.value);
    const occupied = [];
    for (const metric of plotted) draw(metric, measured.get(metric.id), occupied);
    if (performance.now() < animateUntil) frame = requestAnimationFrame(render);
  }

  function schedule() {
    if (toggle.checked && !frame) frame = requestAnimationFrame(render);
  }

  toggle.addEventListener('change', () => {
    inspector.hidden = !toggle.checked;
    toolbar.hidden = !toggle.checked;
    overlay.toggleAttribute('hidden', !toggle.checked);
    if (toggle.checked) {
      selection.textContent = 'T / R / B / L = top / right / bottom / left. X / Y = width / height. Negative values indicate overlap. All areas shows an overview; select an ID for detail.';
      renderTable();
      schedule();
    } else if (frame) { cancelAnimationFrame(frame); frame = 0; }
  });
  area.addEventListener('change', () => {
    const defaults = {window:'W04',header:'H05',tabs:'T05',sidebar:'S03',terminal:'E01',quota:'Q12',find:'F01',system:'A01',radii:'R02',all:''};
    selected = defaults[area.value];
    if (area.value === 'sidebar') setMode('pinned');
    if (area.value === 'quota') setQuotaOpen(true);
    if (area.value === 'find') reveal(metrics.find(m => m.id === 'F01'));
    selection.textContent = 'Select an ID to isolate its edges. Shared component definitions apply to every repeated instance.';
    renderTable();
    schedule();
  });
  picker.addEventListener('change', () => {
    selected = picker.value;
    const metric = metrics.find(m => m.id === selected);
    if (metric) { reveal(metric); selection.textContent = metric.id + ' · ' + metric.label + '. ' + metric.note; }
    renderTable();
    schedule();
  });
  sidebar.addEventListener('change', () => { setMode(sidebar.value); schedule(); });
  document.getElementById('spacing-all').addEventListener('click', () => { selected = ''; renderTable(); schedule(); });
  new MutationObserver(() => { animateUntil = performance.now() + 450; schedule(); }).observe(win, {attributes:true,childList:true,subtree:true,characterData:true});
  new ResizeObserver(schedule).observe(stage);
  win.addEventListener('transitionrun', () => { animateUntil = performance.now() + 450; schedule(); });
  win.addEventListener('transitionend', schedule);
  window.combeSpacing = {metrics, measure, refresh:schedule};
})();
