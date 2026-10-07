# Third-party notices

No license is assigned to Smarti Local Search by this file. Third-party components retain their own licenses.

- EmbeddingGemma 2 740M: Google, Apache-2.0. Pinned official LiteRT model source and SHA-256 are in `resources/models/model-manifest.json`. Model card: https://huggingface.co/google/embeddinggemma-2 . Applicable model policy and limitations must be reviewed for any redistribution.
- LiteRT-LM: Google ODML authors, Apache-2.0; https://github.com/google-ai-edge/LiteRT-LM . Python wheel and its DLLs are bundled together.
- Tabler Icons: Copyright (c) 2020-2026 Paweł Kuna, MIT; https://github.com/tabler/tabler-icons . Artwork is inline SVG from the official `@tabler/icons-react` package, pinned to 3.48.0.
- FFmpeg: the FFmpeg developers, LGPL build from BtbN/FFmpeg-Builds. The exact release, archive checksum and source URL are in `resources/ffmpeg/manifest.json`. Dynamic libraries are distributed unmodified. Corresponding source and build scripts: https://github.com/BtbN/FFmpeg-Builds/tree/master and https://git.ffmpeg.org/ffmpeg.git . Include the matching source when redistributing as required by its licenses.
- PDFium and pypdfium2: BSD-3-Clause and Apache-2.0/BSD; license files accompanying the wheel are included in the generated package inventory.
- SQLite: public domain. USearch: Apache-2.0. Tree-sitter and language grammars: MIT.
- React, Tauri, Vite and the remaining Rust/npm/Python dependencies retain their respective licenses. `scripts/collect_notices.py` copies their supplied license/notice files and generates a versioned inventory under `resources/licenses/` before packaging.

The design foundations were supplied by the user in `tokens.zip`; their values and relevant primitives are used without importing the original Smarti application. Original application-specific documentation is retained as a design reference under `docs/DESIGN_REFERENCE.md`.
