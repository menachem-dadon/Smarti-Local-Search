# Smarti Local Search

תוכנת חיפוש מקומית ל־Windows, בעברית ובאנגלית. חיפוש שמות ותוכן מתחיל מיד; חיפוש סמנטי משתמש במודל EmbeddingGemma 2 740M מקומי. אפשר לחפש גם לפי תמונה או קובץ, להציג תוכן ודפי PDF, ולמצוא מקטעי אודיו ווידאו.

Smarti Local Search indexes your selected files locally. Search queries, file contents and embeddings are not sent to a remote service.

## הפעלה

התקן באמצעות קובץ ה־EXE תחת `target/release/bundle/nsis/`. המתקין מיועד למשתמש הנוכחי, וכולל את המודל, שירות ההסקה, FFmpeg, PDFium ומתקין WebView2 שאינו זקוק להורדה בזמן התקנה. בחר תיקיות באשף ההתחלה; אין חשבון או הורדת מודל בהפעלה הראשונה.

- Ctrl+K — שדה החיפוש.
- Ctrl+Alt+Space — חיפוש מהיר מכל תוכנה.
- Enter — פתיחת קובץ; Shift+Enter — הצגה בסייר.
- רווח — תצוגה מקדימה; Esc — סגירת תצוגה/חיפוש מהיר.
- חיפוש לדוגמה: `type:pdf`, `ext:rs`, `path:"C:\\Projects"`, `after:2026-01-01`, `size:>10MB`.

שגיאת מודל אינה חוסמת חיפוש שמות וטקסט. כונן מנותק נשמר באינדקס; אפשר לכלול תוצאות לא מקוונות בהגדרות. קבצים רגישים ותיקיות מטמון מוחרגים כברירת מחדל.

## Development and packaging

Requires Windows x64, Node 22+, Rust stable/MSVC with Windows SDK, and Python 3.13 **on the build machine**. Installed users need none of these development tools.

```powershell
.\scripts\dev.ps1
.\scripts\test.ps1
.\scripts\package.ps1
```

`package.ps1` restores pinned dependencies, downloads verified model/FFmpeg during the build, runs tests, creates the standalone inference host, collects licenses, and builds an NSIS installer. `-SkipTests` is for an already validated source tree. Large binaries and private indexes are ignored by Git.

Release app: `target/release/smarti-local-search.exe` (requires its adjacent `resources` folder and application-local MSVC DLLs).
Installer: `target/release/bundle/nsis/*-setup.exe`.
Checksums and build verification: `artifacts/release/`.
The installer is the distribution artifact; the standalone app EXE alone is insufficient.

Default private data: `%LOCALAPPDATA%/Smarti Local Search`. Uninstall preserves this index. Storage migration copies and validates the index, preserves the old copy, and restarts the app. `SMARTI_SEARCH_DATA_DIR` isolates development/testing.

## Validation and limits

See [validation evidence](docs/VALIDATION.md). Source checks, real inference tests, live UI checks, and installed-app checks are reported separately. GPU/NPU are enabled only after real local initialization and inference; an unsupported accelerator falls back to CPU. Vision/audio use CPU.

Final Hebrew delivery report, installer size and artifact paths: [מסירה](docs/DELIVERY.md).

The index stores extracted text and embeddings in ordinary local files. Store it privately; it is not an encrypted security boundary. Corrupt/encrypted/unsupported files are reported individually. Archive contents are not recursively unpacked. See [decisions](docs/DECISIONS.md) for implementation limits.

Architecture and formats: [architecture](docs/ARCHITECTURE.md), [index](docs/INDEX_FORMAT.md), [inference](docs/INFERENCE.md), [performance](docs/PERFORMANCE.md), [Windows](docs/WINDOWS_INTEGRATION.md), [design](docs/DESIGN_SYSTEM.md).

