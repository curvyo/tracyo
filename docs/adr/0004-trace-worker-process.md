# ADR 0004: The trace runs in a worker process; cancel, progress and preview

**Status:** Accepted (lead, slice 5, 2026-11-11; proposed by the architect 2026-11-11).

Deviation: the analysis runs on an app thread, not in the worker process, and cannot be stopped (technical debt 6); trace, preview and OCR follow this ADR.

## Context

The spec asks: the UI thread never runs vectrace (criterion 7); Stop ends a trace at once
(criterion 8); running out of memory shows "Failed", never a crash (criterion 24); closing the
window during a trace ends the app within 2 s (criterion 24). Facts of `vectrace` v0.1.0
(tag = 461d45c, read in its source):

- `job::run(&Job, on_percent: &mut dyn FnMut(u8)) -> Vec<f64>` is the only entry point that runs
  the text pass and shape recognition with the trace (`Job { raster, map, mode: Mono|Colours,
  lines: &[OcrLine], mask, shapes }`). It reports whole percents but **cannot be cancelled**: its
  inner poll always continues. vectrace's ADR says Curvyo stops it by terminating the web worker,
  and native callers "use the low-level calls and their poll" (`trace`, `trace_colours`), which
  have no text pass and no shapes.
- `wire::read(&[f64]) -> Option<Output>` turns the numbers back into outlines (mm, `kurbo::BezPath`),
  palette, layers, texts and shapes. The wire is vectrace's documented transport format.
- `ocr::read_text(&OcrModel, rgba, w, h, on_percent -> ControlFlow)` is cancellable and gives
  `Vec<OcrLine>` (plain public fields) in source pixels; `OcrLine::scaled` maps them to a reduced
  copy. `analyse(Raster)` gives the automatic proposal.
- Rust aborts the process when an allocation fails; a thread cannot catch it.

| Option | Verdict |
|---|---|
| **A. Worker process: the same binary started as `tracyo --trace-worker`, killed to cancel (chosen)** | Works with v0.1.0 unchanged; Stop and window close are a `Child::kill`; an abort from out of memory ends only the worker, and its exit is read as "out of memory at N Mpx". Mirrors Curvyo's terminated web worker, and the wire is already the transport format. Cost: a small framed protocol, a re-spawn (about 10 to 30 ms) after each cancel, the picture copied into the worker per job. |
| B. Worker thread + a vectrace PR that adds a poll to `job::run` (v0.2.0) | Cancel solved, out of memory not: an abort takes the whole app down, against criterion 24. Needs a vectrace release first. |
| C. Worker thread, a stopped job keeps running and its result is dropped | Stop would not stop; drags would pile up 12 Mpx jobs. |

## Decision

1. **One binary, two modes.** `main` checks for the single argument `--trace-worker` before any
   window code and then runs `tracyo_app::worker::serve(stdin, stdout)`; otherwise it starts the
   UI. The worker reads requests from stdin and writes replies to stdout, nothing else (no file,
   no network). On Windows it is spawned with `CREATE_NO_WINDOW`; on macOS it never touches
   winit, so no Dock icon appears. Integration tests pass `env!("CARGO_BIN_EXE_tracyo")` as the
   executable; the app passes `std::env::current_exe()`.
2. **Protocol** (`tracyo-core::protocol`, hand-written, little-endian, length-prefixed frames,
   no new dependency, round-trip tested): requests `Analyse { picture }`,
   `ReadText { picture }`, `Trace { picture, settings, lines, reduced, shapes }`; replies
   `Percent(u8)`, `Analysis(Proposal)`, `Lines(Vec<OcrLine>)`, `Wire(Vec<f64>)`, `Error(text)`.
   The picture is sent once per worker and kept there until a new one is sent.
3. **One job at a time, latest wins.** A changed value while a job runs kills the worker, spawns a
   new one and sends the new request (Curvyo's behaviour). Kill is also Stop, Esc and window
   close (the runner's `Drop` kills the child, so closing never waits for a trace). A worker that
   exits without a reply is "Failed": the reason is "out of memory at N Mpx" when its stderr says
   `memory allocation` (Rust's abort message) or it was killed by the kernel's OOM signal, else
   "internal error" (Curvyo's `traceText.ts` rule).
4. **The app side** (`tracyo-app::runner`): one reader thread per worker turns frames into
   messages on a channel and calls `ctx.request_repaint()`; the UI thread only reads the channel,
   runs `wire::read` and builds the preview. `tracyo-core::progress` holds the state machine with
   time passed in: the percentage shows after 400 ms of running and stays at least 300 ms once
   shown; a full trace before saving shows it at once; Stopped and Failed keep the old outline.
5. **Preview size.** While a value field is dragged, and always for a picture over 4 Mpx, the
   request is `reduced` (Curvyo's `reduced_copy`: longest side 1000 px, area average weighted by
   alpha, ported into `tracyo-core::picture` with its tests). On release the full picture is traced
   (pictures up to 4 Mpx). Save as… traces the full picture first when the preview is reduced.
   The pixel map is `scale(25.4 / 96 · s)` with `s` = full width / reduced width, so a reduced
   result is in the same millimetres.
6. **Text.** With Recognise text on, the first preview of a picture is `ReadText` (button "Reading
   text", stoppable) on the full picture; the app keeps the `OcrLine`s for that picture and sends
   them (scaled for a reduced copy) with every later `Trace`. With it off the model is never
   loaded. The analysis (`Analyse`) runs once per picture as its own job.
7. **Drawing the preview** (`tracyo-app::preview`): outlines are flattened and tessellated once per
   result and zoom level into one `egui::Mesh` (white 4.5 px casing, then 1.5 px `#2F6FEE` line,
   constant in screen pixels), cached in an `Arc`; pan translates the cached mesh, zoom
   re-tessellates (the old mesh is drawn scaled until the new one is ready in the same frame
   budget). Only outlines whose bounds meet the view are drawn. If a 12 Mpx result misses the
   50 ms frame budget (criterion 7, UI-thread CPU time), the fallback is a wgpu paint callback
   that keeps the mesh on the GPU and widens the line in the vertex shader; it is not built
   before a measurement asks for it.

## Consequences

- Tracyo needs no vectrace change for slice 1 to 4. Should vectrace later get a cancellable
  `job::run`, B becomes possible, but the out-of-memory reason for A stays.
- The picture is in memory twice while a job runs (48 MB for 12 Mpx), plus once per re-spawn.
- If the app itself is killed, a running worker finishes its job, finds stdin closed and exits.
- `rten`'s `rayon` threads run inside the worker process only.
