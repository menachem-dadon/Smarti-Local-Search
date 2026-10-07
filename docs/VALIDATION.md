# Validation evidence

Validation date: 2026-10-07. Build, real inference, installed native operation and visual acceptance are separate evidence categories.

## Automated source checks

- Rust core: 12 unit/store tests passed. The native app geometry test also passed, covering 100/125/150/200% DPI calculations and negative monitor coordinates. Workspace formatting and Clippy with warnings treated as errors passed.
- Real end-to-end corpus test passed: filename/FTS, natural-language/code/image queries, PDF/Office/image/audio/video indexing, sensitive exclusion, stable rename identity, modified content, pause/resume and persisted restart. The extended test also passed native-model restoration after an injected database failure, and a confirmed change from 256 to 128 dimensions with preserved lexical search and successful semantic rebuilding. The real model was required; this test is explicitly enabled by `scripts/test.ps1`.
- Python framing, normalization, limits and supported document extraction: four tests passed.
- Frontend: ten tests passed, including language keys, accessible controls, stale-result cancellation, explicit Enter behavior, media-segment preview/jump and confirmation-error visibility/focus restoration. These are DOM/component tests; RTL attributes do not constitute physical visual acceptance.
- Frontend typecheck and production build passed: 94 modules, 291.62KB JavaScript (90.87KB compressed) and 31.75KB CSS.
- Resource verification passed: pinned model checksum, FFmpeg/PDFium/LiteRT native components, application-local MSVC runtime hashes, notices and a standalone inference executable.
- Final source formatting/diff, PowerShell syntax and release resource checks passed. The installer was built after capturing 90 source/configuration hashes; all still matched after compilation and installed-app testing.

## Real model and installed native app

The final NSIS installer was built and tested in a fresh private directory, `artifacts/installed-smoke-20261007-091652`, with `SMARTI_SEARCH_DATA_DIR` isolating the index. The installed app indexed 13 nonsensitive files (12 semantically), committed 15 vectors with zero errors, searched names/text/images, checked audio/video similarity with segment timestamps, rendered a PDF page and reported SQLite integrity OK. It also verified a visible native main-window object (1500 × 940), global shortcut registration, tray-menu creation, and opening/hiding the native Quick Search window. The native runtime test completed in 24.38 seconds on this shared host; this excludes installation and uninstall time. These native API checks do not prove rendered appearance, actual key presses or tray clicks.

The installed inference executable was separately launched from its installed resource directory: real CPU embeddings and PDF rendering passed, including an explicit NPU request that fell back to CPU. During automatic selection GPU initialization was attempted and failed; OpenVINO reported CPU/GPU devices and no NPU. This does not certify successful GPU or NPU operation on another PC.

Silent uninstall succeeded, removed the installed application executable and retained the private SQLite index. No Python/Node/Rust developer runtime is used by the installed application; Python only drives the external test harness.

Final installer: `artifacts/release/Smarti Local Search_0.1.0_x64-setup.exe`, 765,139,458 bytes (729.69 MiB). SHA-256: `5c08d6446723abed7ab32d71a8005f591fd7837bd652cd3a923b8f5718d50c67`. The Release application is 20,500,992 bytes; SHA-256: `66c7d3c66f1bf1a64bac134ca1aa154fe8924cbaf56cab038feae548cca4e2e4`.

Reports: `artifacts/release/verification.json` records resource verification and successful install/uninstall; `native-smoke.json` records installed native runtime evidence; `inference-fallback.json` records installed CPU fallback; `SHA256SUMS.txt` records the distribution hashes; `source-manifest.json` records the compiler input snapshot. The final installer and application were copied to `artifacts/release/`.

## 100k scale measurement

Native USearch/SQLite benchmark, synthetic 100,000 vectors at 128 dimensions and 100,000 metadata files, on a shared Intel i5-1235U / 16GB / Windows 11 machine:

| Measurement | Observed |
| --- | ---: |
| ANN query p50 / p95 (100 queries) | 0.810 / 1.921 ms |
| ANN resident working set after construction | 60.9 MiB |
| ANN snapshot load | 986.7 ms |
| FTS count of 100k matches | 3.43 ms |
| ANN construction | 604.8 s |
| Metadata insertion | 17.0 s |

These synthetic vectors isolate index costs. Product vectors are real model embeddings. These numbers are not product guarantees, a 1M-vector result or total application peak RAM. Full report: `artifacts/performance/benchmark.json`.

## Acceptance still requiring manual/external evidence

The Windows computer-use helper initially timed out. After work resumed it recovered, but the desktop was locked; UI input stopped and an unlock request was sent. No substitute screenshots or claimed visual approval were fabricated. Physical RTL/LTR, light/dark/system, 100/125/150/200% DPI, maximized/narrow windows, screen-reader behavior, actual global key presses, tray clicks and Explorer invocation still require manual acceptance. Native window visibility and shortcut registration are narrower automated checks.

A clean Windows VM, older CPU compatibility, successful accelerated drivers and code signing/SmartScreen reputation have not been certified. This installer is unsigned. Restart behavior, journal fallback and memory/scheduler tradeoffs are described in `DECISIONS.md`.
