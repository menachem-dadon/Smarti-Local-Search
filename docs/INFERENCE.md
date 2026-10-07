# Local inference

The pinned official model is [EmbeddingGemma 2 740M LiteRT-LM](https://huggingface.co/litert-community/embeddinggemma-2-740m-litert-lm), revision `24d962e906c7d332c6428e71c9676855024569e2`. Manifest size: 484,622,336 bytes. SHA-256: `e7a8a2204b91e0f96e92960e84a09a89212e1633dcb7575a9bf3378b4df77f4c`.

LiteRT-LM 0.18.0 is bundled in a PyInstaller onedir executable with CPython and native DLLs. This uses the released `EmbeddingEngine` API, including `compute_embedding_batch` when available. There are no random vectors, lexical-derived replacement embeddings, cloud fallbacks or installed-user Python requirements.

Query text: `task: search result | query: ...`. Query code: `task: code retrieval | query: ...`. Corpus text/code: `title: filename | text: ...`. Images/audio do not receive invented task prefixes. Code vectors occupy a separate namespace.

Automatic selection initializes GPU, NPU and CPU candidates and measures warmed real queries; only candidates with valid finite normalized 768-dimensional output qualify. OpenVINO detects real NPU support. Backend errors or worker exits retry once on CPU. Explicit GPU/NPU preferences also include CPU fallback. Vision/audio backends remain CPU. Backend probing is runtime-dependent and does not promise compatibility merely from a device name.

Text/image context uses 32–512 tokens; audio/video use a lazy CPU context up to 8,192 tokens. On low available RAM the text context is released before allocating media. A single model file is distributed. Native LiteRT cache paths containing non-ASCII characters are omitted because the released Windows runtime failed to create its cache on this host; the app's model and data paths still support Unicode.

The IPC frame is a little-endian uint32 length plus MessagePack, capped at 64MB. Vectors are binary float32, never JSON matrices. Native stdout diagnostics are redirected to stderr. Search jobs outrank indexing jobs between native calls. A process deadline kills a hung worker after 120 seconds (290 for benchmark) and allows one restart. Idle unloading is controlled by Keep search ready.

PDFium extracts text and page images; DOCX/PPTX/XLSX/ODS use native document libraries. FFmpeg/FFprobe determine media segments; each segment is decoded only when embedded, keeping long videos from producing a full decoded-file cache up front. Segment temporary files are removed after inference. No runtime model download occurs.
