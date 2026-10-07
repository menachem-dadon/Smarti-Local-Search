# Index format

Version 1 uses `data/metadata.sqlite` (SQLite WAL) and `data/vectors/{general,code}.usearch`. The metadata schema records roots, exclusions, stable Windows volume/file IDs, file paths/types, byte size, display timestamps plus nanosecond change timestamps, streaming content hashes, state/errors, chunks, jobs, bounded activity, settings and measured benchmarks.

Chunks include sequence, modality, heading, PDF page/Office section, code lines, media start/end, content hash, namespace and an optional binary little-endian float32 vector. SQLite is authoritative. Float32 vectors aid recovery; ANN stores float16 cosine vectors. Every vector is truncated from the model's 768 dimensions to the configured 128/256/512/768 and then normalized.

External-content FTS5 tables index filenames/paths and extracted chunk text/headings using unicode61. Triggers maintain search consistency through inserts, updates and deletes. File replacement is transactional. Chunk IDs are never reused.

The fingerprint includes model identity, embedding dimensions, task prefixes, chunking/schema version, vision token budget, audio/video duration and sampling FPS. Semantic changes require explicit confirmation and rebuild semantic vectors while preserving lexical content. Backend and CPU thread changes do not invalidate mathematical embeddings.

A checkpoint combines the fingerprint with committed vector count and maximum chunk ID. Invalid/missing ANN snapshots rebuild from SQLite. Compact uses VACUUM and saves ANN. Migration uses SQLite VACUUM INTO, validates the destination, copies ANN, records a startup location pointer and retains the original data.

Semantic configuration changes atomically persist the new settings, invalidate previous vectors and queue files for rebuilding while retaining lexical chunks. A failure rolls back all three operations. The runtime restores the preceding native inference configuration before returning the error.

The activity table is capped at 2,000 rows. Diagnostic exports may contain paths and are explicitly requested by the user. They omit document contents. User data is never bundled or committed.
