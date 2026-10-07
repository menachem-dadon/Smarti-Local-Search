# Performance

Run `scripts/benchmark.ps1` for the native synthetic 100k-vector/100k-file ANN/FTS benchmark. It records construction, resident memory, query p50/p95, snapshot save/load, metadata insertion and FTS timing. This deliberately uses synthetic vectors to isolate ANN costs; product search continues to use real model embeddings exclusively.

See `artifacts/inference-smoke/health.json`, the persisted benchmark table and `docs/VALIDATION.md` for measured evidence. Timings describe this host and workload; they are not promised on other PCs.

The benchmark measures real batch sizes 1/4/8/16/32 and worker counts 1/2/4, and stores model checksum, app/runtime version, hardware fingerprint and measurement time. The current dispatcher remains a single prioritized actor, with small batches capped at four items (one for quiet/media), avoiding unbounded model instances. Measured results are reported; they do not automatically enable unsafe parallel engine access.

Metadata appears first, FTS runs immediately, and semantic requests wait 125ms to absorb typing. Small indexing batches allow a pending search to overtake subsequent indexing calls. An in-flight native call cannot be interrupted; its result may be discarded. Results and activity lists are virtualized.

Discovery skips known dependency/cache folders and the app's own resources/data. Content hashes stream through a 64KB buffer. File jobs are persistent. Media decodes per segment, with bounded resolution and CPU decode threads. Battery/saver policy defers heavy media jobs while allowing text/code and search. Inference/decoding processes use below-normal Windows priority.

HNSW uses cosine float16 storage, M=16, efConstruction=128, efSearch=80. SQLite retains float32 recovery data. Both general multimodal and code namespaces are used. The current ANN snapshots load in memory; memory mapping, quantized SQLite storage and direct per-record USN replay are future optimizations.

Vite/Rollup property-path analysis caused runaway memory during React 19 builds on this host. Disabling Rollup tree shaking and directly importing only the used official Tabler modules resolved it: 94 modules, about 291KB JS and about 91KB compressed. The latest production bundle took 1.57 seconds. No source or third-party binary was patched.
