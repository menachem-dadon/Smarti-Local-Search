# Performance

Run `scripts/benchmark.ps1` for the native synthetic 100k-vector/100k-file ANN/FTS benchmark. It records construction, resident memory, query p50/p95, snapshot save/load, metadata insertion and FTS timing. This deliberately uses synthetic vectors to isolate ANN costs; product search continues to use real model embeddings exclusively.

See `artifacts/inference-smoke/health.json`, the persisted benchmark table and `docs/VALIDATION.md` for measured evidence. Timings describe this host and workload; they are not promised on other PCs.

The benchmark measures real batch sizes 1/4/8/16/32 and worker counts 1/2/4, and stores model checksum, app/runtime version, hardware fingerprint and measurement time. The current dispatcher remains a single prioritized actor, with small batches capped at four items (one for quiet/media), avoiding unbounded model instances. Measured results are reported; they do not automatically enable unsafe parallel engine access.

Metadata appears first, FTS runs immediately, and semantic requests wait 125ms to absorb typing. Small indexing batches allow a pending search to overtake subsequent indexing calls. An in-flight native call cannot be interrupted; its result may be discarded. Results and activity lists are virtualized.

Discovery skips known dependency/cache folders and the app's own resources/data. Content hashes stream through a 64KB buffer. File jobs are persistent. Media decodes per segment, with bounded resolution and CPU decode threads. Battery/saver policy defers heavy media jobs while allowing text/code and search. Inference/decoding processes use below-normal Windows priority.

Discovery walks each selected location once; it no longer performs a separate
counting walk. Existing indexed file counts provide a provisional discovery ETA.
A brand-new location displays "Estimating" while its total is unknown. The final
total is corrected to the number actually discovered. Discovery timing excludes
pause time; indexing ETA uses completion times by file type. Unseen types use the
observed average with a provisional label. Media duration and large files can
change the estimate substantially.

Root scan requests use a set and coalesce both queued and active requests. Normal
watcher events never queue a recursive root scan: file metadata is reconciled
individually, directory metadata checks direct children, and a directory creation
or move walks only that subtree. Native events debounce for 500 ms; excluded and
owned paths are filtered before buffering. The buffer is bounded to 8,192 paths,
and batches are bounded to 256 paths. An overflow schedules one metadata recovery
pass while the file queue is idle. Healthy watchers do not trigger periodic root
rescans. Offline locations and degraded watchers are checked again every five
minutes while idle. Startup still reconciles changes made while the app was shut
down; unchanged metadata/content does not generate new embeddings.

Identity lookups use `(root_id, stable_id)`; unchanged known paths update only
their reconciliation generation. A persisted job priority and covering
`(online, state, priority, file_id)` index select eligible work without sorting or
joining large file rows. Queue kind/online summaries are maintained by triggers.
Partial/covering indices serve status counts and location counts without reading
content or vector BLOBs. SQLite's page cache is bounded to 32 MiB. The 250 ms delay
applies only while idle or suspended. Interrupted active jobs return to queued
state; interrupted subtree walks are retained for resume. Extraction checks file
metadata again before committing and retries a file edited during processing.

Exclusions accept folder components, relative patterns, full local paths and UNC
paths. Absolute paths use case-insensitive component boundaries; local short path
aliases are resolved and UNC matching does not contact a server. Known caches,
generated build folders and narrowly timestamped automatic backups are defaults;
ordinary `backup`, `bin`, `vendor` and `artifacts` folders remain searchable. New
defaults are added once on upgrade without replacing existing settings, and users
can remove them. Excluded old records are removed from SQLite/ANN during discovery
even if unrelated directories are unreadable. Original files are never removed.

HNSW uses cosine float16 storage, M=16, efConstruction=128, efSearch=80. SQLite retains float32 recovery data. Both general multimodal and code namespaces are used. The current ANN snapshots load in memory; memory mapping, quantized SQLite storage and direct per-record USN replay are future optimizations.

Vite/Rollup property-path analysis caused runaway memory during React 19 builds on this host. Disabling Rollup tree shaking and directly importing only the used official Tabler modules resolved it: 94 modules, about 291KB JS and about 91KB compressed. The latest production bundle took 1.57 seconds. No source or third-party binary was patched.
