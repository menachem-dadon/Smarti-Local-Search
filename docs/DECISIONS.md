# Decisions and limitations

- Rust owns index/search consistency. Python is packaged as a local stdio worker because the released Windows LiteRT-LM wheel exposes the official embedding API; installed users do not install Python.
- SQLite stores recovery vectors, and USearch stores the live ANN. This trades some disk size for reliable crash recovery.
- Windows CPU compatibility uses USearch's generic kernel; its optional NumKong feature failed to link with this MSVC toolchain. No upstream binary patches were used.
- LiteRT Unicode cache-directory initialization failed on this host. Passing no native cache directory fixes inference; model/data paths continue to support Unicode.
- A single priority actor limits model memory and allows search to overtake subsequent indexing batches. True concurrent engine pools and benchmark-driven automatic pool sizing are not enabled.
- USN identity/cursor checks can skip unchanged-volume scans. Journal changes currently trigger metadata reconciliation; direct USN record replay and an MFT bootstrap are not implemented. Other volumes receive background metadata reconciliation after restart.
- FTS is lexical; semantic passages are never presented as exact term highlights. Hybrid ranking fuses independent filename, FTS and semantic candidates and groups results by file.
- Audio/video are embedded directly. No generated transcript, caption, OCR service, chatbot or cloud API is invented.
- Archives are filename-only. Encrypted/corrupt unsupported documents are isolated errors. Sensitive content remains excluded unless enabled.
- Native GPU/NPU availability and performance depend on real local driver/runtime support. Successful CPU tests do not certify other accelerators.
- Package tests on this host do not certify a clean Windows VM, all DPI settings, older CPUs, accessibility screen readers or signing. Missing evidence is recorded explicitly in VALIDATION.md.
