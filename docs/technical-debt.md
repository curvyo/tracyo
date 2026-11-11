# Technical debt

Known gaps, each with where it comes from and what would close it. Owned by the architect.

| # | Debt | Source | Closes when |
|---|---|---|---|
| 1 | Screen-reader announcements of Curvyo ("7 paths, 412 nodes", "3 text lines set") are not made | spec, Out of scope (Could) | a story adds the AccessKit live-region texts |
| 2 | Pipette cursor is a crosshair, not Curvyo's eyedropper image | ADR 0003 §10 | eframe exposes custom cursor images |
| 3 | No file drop on a pure Wayland session (no XWayland) | ADR 0003 §3, winit 0.30 | winit delivers drops on Wayland; then the X11 preference is dropped |
| 4 | Without an xdg-desktop-portal the Linux file dialogs do not open and Tracyo cannot tell this from a cancel | ADR 0003 §4 | rfd reports the missing portal, or a GTK fallback is decided |
| 5 | A worker whose app was killed finishes its current job before it exits | ADR 0004 | a parent-death signal without `unsafe` is available, or jobs poll for a closed pipe |
| 6 | Release builds are unsigned (Gatekeeper and SmartScreen warn) | ADR 0007 §9, Frage 6 | the customer decides on signing |
| 7 | Curvyo draws its file-drop cue in `#F5F5F5` instead of `#2F6FEE` | ux-notes §11 | reported to and fixed in Curvyo; then the dash pattern is checked against it |
