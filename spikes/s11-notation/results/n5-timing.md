### N5 (a) layout computation, ms (release)

| Case | Median | p95 | p99 | Runs |
|---|---|---|---|---|
| TYPICAL, notation + TAB, one system | 6.828 | 16.619 | 19.467 | 200 |
| TYPICAL, notation + TAB, in view (4 systems) | 6.648 | 9.519 | 9.879 | 200 |
| TYPICAL, notation + TAB, whole score | 14.277 | 19.898 | 24.031 | 200 |
| TYPICAL, notation + TAB, in view from cached plans | 0.303 | 0.524 | 0.605 | 200 |
| TYPICAL, notation + TAB, edit: replan 2 bars + in view from cache | 0.370 | 0.624 | 0.700 | 200 |
| TYPICAL, building all plans (cache fill, once per open) | 5.467 | 7.046 | 8.117 | 20 |
| TYPICAL, spike TAB only, in view | 5.755 | 8.948 | 12.320 | 200 |
| TYPICAL, today's mc-notation TAB, in view (outline + 4 systems) | 0.790 | 1.208 | 1.365 | 200 |
| LARGE, notation + TAB, one system | 124.688 | 160.701 | 192.654 | 200 |
| LARGE, notation + TAB, in view (4 systems) | 129.570 | 186.361 | 211.649 | 200 |
| LARGE, notation + TAB, whole score | 322.864 | 393.549 | 480.478 | 200 |
| LARGE, notation + TAB, in view from cached plans | 2.966 | 3.602 | 5.279 | 200 |
| LARGE, notation + TAB, edit: replan 2 bars + in view from cache | 3.784 | 4.285 | 4.905 | 200 |
| LARGE, building all plans (cache fill, once per open) | 118.075 | 133.649 | 167.369 | 20 |
| LARGE, spike TAB only, in view | 118.303 | 153.364 | 164.406 | 200 |
| LARGE, today's mc-notation TAB, in view (outline + 4 systems) | 12.180 | 13.801 | 15.064 | 200 |

### N5 (b) rendering (layout model to SVG), ms

| Case | Median | p95 | p99 | Runs |
|---|---|---|---|---|
| TYPICAL in view (4 systems) | 0.580 | 0.598 | 0.701 | 200 |
| TYPICAL whole score | 19.564 | 22.061 | 25.347 | 200 |

### N5 (c) PDF generation, ms

| Case | Median | p95 | p99 | Runs |
|---|---|---|---|---|
| feature_rich, vector | 0.140 | 0.161 | 0.207 | 200 |
| feature_rich, SVG-to-PDF (SVG given) | 13.634 | 16.194 | 18.499 | 200 |
| TYPICAL whole, vector | 59.714 | 66.229 | 82.856 | 50 |
| TYPICAL whole, SVG-to-PDF (SVG given) | 4179.830 | 5214.742 | 5349.476 | 50 |
| of which: system font scan (usvg fontdb) | 12.932 | 15.064 | 17.122 | 20 |

### N5 payload per system (TYPICAL, system 0)

| Layout | Bars | Lines | Glyphs and texts | Bytes (estimate) |
|---|---|---|---|---|
| Spike, notation + TAB | 1 | 106 | 164 | 4563 |
| Spike, TAB only | 1 | 28 | 68 | 1381 |
| Today's mc-notation TAB (default metrics) | 4 | 236 | 296 | 8748 (+ 864 hit regions, 32 bytes each) |
