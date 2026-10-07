# Smarti Local Search
## Full Product & Implementation Specification for Codex

### 0. הוראת־על לקודקס

בנה את **Smarti Local Search** כתוכנת Windows מלאה, עובדת וניתנת להתקנה, ולא כאב־טיפוס חזותי בלבד.

המטרה היא לבצע את הפרויקט **בבת אחת** ככל האפשר. אין לעצור לאחר scaffolding, מסך דמה, מנוע טקסט בלבד או "שלב ראשון". יש לממש את כל היכולות הנדרשות במסמך זה, להריץ בדיקות, לתקן כשלים, לבצע build של Release וליצור מתקין Windows סופי.

אם קיימת בחירת implementation קטנה שאינה מוגדרת במפורש, בחר ברירת מחדל מקצועית והמשך. אל תעצור כדי לשאול שאלות שאפשר לפתור באופן סביר לבד. תעד החלטות משמעותיות ב־`docs/DECISIONS.md`.

אין למחוק או להסתיר כשל אמיתי כדי לגרום לבדיקות לעבור.

אין להשאיר כפתורים שאינם מחוברים לפעולה, flows מדומים, נתוני demo במוצר הסופי או TODO עבור יכולת המוגדרת כאן כחובה.

התוצר העיקרי הוא **קוד המקור המלא**. כל EXE/Installer הוא תוצר שנבנה מן המקור ולא תחליף לו. בכל שינוי עתידי יש לבצע את השינוי במקור ולבנות ממנו מחדש; אין לבצע patch ידני לקובץ בינארי.

---

# 1. זהות המוצר

שם המוצר:

`Smarti Local Search`

Application ID מומלץ:

`com.smarti.localsearch`

המוצר הוא מנוע חיפוש מקומי וסמנטי לכל קובצי המשתמש ב־Windows.

הוא צריך לשלב את מהירות החיפוש לפי שם/נתיב/טקסט של כלי חיפוש רגילים עם חיפוש סמנטי מבוסס EmbeddingGemma 2.

דוגמאות:

- "המסמך שבו כתבתי על ארכיטקטורת SparkMoE"
- "התמונה של ילד משחק בכדור ליד הים"
- "הקוד שמטפל בחיבור ל־OAuth"
- "הסרטון שבו רואים גרף של ביצועי Qwen"
- "ההקלטה שבה דיברו על ביטול הפגישה"
- חיפוש לפי תמונה אחרת.
- חיפוש לפי קטע אודיו.
- חיפוש לפי קובץ לדוגמה.
- חיפוש מדויק בשם קובץ, נתיב, ביטוי או extension.

הכול Local-first ו־Offline-first.

אין צורך בחשבון.

אין API ענן.

אין שליחת קבצים, embeddings, metadata או שאילתות לשום שרת.

אין telemetry כברירת מחדל, ועדיף שלא יהיה telemetry כלל בגרסה זו.

---

# 2. מקורות העיצוב המצורפים

לפרויקט נמסרו:

`tokens.zip`

ו־

`ui_ux_redesign_plan.md`

יש ללמוד את שניהם לפני בניית ה־UI.

`tokens.zip` כולל לפחות:

- `tokens.ts`
- `system.css`
- `primitives.tsx`
- `README.md`

הקבצים האלה הם מקור ההתייחסות הראשי לשפת העיצוב.

יש להשתמש מהם ב:

- צבעים.
- טיפוגרפיה.
- spacing.
- radii.
- semantic color roles.
- מצבי controls.
- focus.
- menus.
- dialogs.
- cards.
- fields.
- switches.
- tabs.
- tooltips.
- scrollbars.
- motion.
- light/dark.
- RTL behavior.
- reduced motion.
- accessibility conventions.

אבל אין לייבא מהמסמכים דרישות פונקציונליות השייכות ל־Smarti המקורי.

במיוחד:

- אין צורך ב־Smarti Core.
- אין Agent.
- אין Chat.
- אין ספקי LLM.
- אין Model Picker של סמארטי.
- אין Workbench של סמארטי.
- אין Browser.
- אין Tasks/Memory/Tools של Smarti.
- אין תהליכי UX-0…UX-6 כחובת פיתוח.
- אין תלות בארכיטקטורת ה־Python Core הישנה של Smarti.

אלה הם הקשר של הפרויקט המקורי בלבד.

יש לקחת ממנו את **שפת העיצוב**, לא את מבנה המוצר.

---

# 3. Design System מחייב

העבר את החלקים הרלוונטיים של מערכת העיצוב לתיקייה:

`src/design-system/`

אין ליצור CSS נוסף שמתחרה בשפת העיצוב.

אין hardcoded colors ברכיבי המוצר מלבד מקרים חריגים ומתועדים.

## 3.1 צבעים

השתמש בערכים המצורפים ב־`tokens.ts`.

Light:

- background: `#f3f4f6`
- surface: `#ffffff`
- surfaceMuted: `#f0f2f5`
- text: `#252b36`
- textMuted: `#606b7e`
- border: `#dde1e8`
- controlBorder: `#7c8594`
- accent: `#365ccd`
- accentHover: `#2e50b8`
- accentPressed: `#244397`
- accentSoft: `#edf1ff`
- action: `#397bfa`
- success: `#28714b`
- warning: `#865b17`
- danger: `#b53645`

Dark:

- background: `#101319`
- surface: `#191d25`
- surfaceMuted: `#232935`
- text: `#edf0f7`
- textMuted: `#a5afc0`
- border: `#343c4b`
- controlBorder: `#8893a6`
- accent: `#a2baff`
- accentSoft: `#293654`
- action: `#397bfa`
- success: `#91d3ac`
- warning: `#e5bd79`
- danger: `#ffb0b8`

השתמש בתפקיד סמנטי ולא בצבע מקרי.

`accent` = בחירה/כפתור ראשי רגיל.

`action` = פעולה מיידית ובולטת במיוחד.

Danger רק לפעולות הרסניות אמיתיות.

Glow/gradient אינם קישוט שגרתי.

## 3.2 טיפוגרפיה

Default:

`"Segoe UI", Arial, sans-serif`

קוד/נתיבים:

`Consolas, "Courier New", monospace`

גדלים:

- caption: 13px
- control: 14px
- body: 16px
- heading: 20px
- display: 32px

אין לארוז קובצי font חיצוניים.

## 3.3 Spacing

השתמש בסולם בלבד:

`4 / 8 / 12 / 16 / 24 / 32 / 48`

## 3.4 Corners

- controls: 10px
- cards: 16px
- dialogs: 16px
- large search/composer-like surfaces: 24px
- pills: fully rounded

## 3.5 targets

יעד לחיצה רגיל:

40px minimum.

פעולות חשובות צריכות להישאר נוחות גם ב־125%/150%/200% DPI.

## 3.6 Motion

- fast: 120ms
- menu: 160ms
- dialog: 200ms
- panel: 260ms
- entry: 180ms

Easing:

`cubic-bezier(.2,.8,.2,1)`

Reduced Motion חייב לכבות תנועות שאינן חיוניות.

אין animations מתמשכות סתם.

אין shimmer כשאין טעינה אמיתית.

---

# 4. אייקונים

השתמש **רק בסט החינמי והרשמי של Tabler Icons**.

אין להשתמש ב־Font Awesome, Material Icons, emoji או אייקונים אקראיים.

יש להשתמש ב־SVG.

אפשר להשתמש ב־:

`@tabler/icons-react`

כאשר הוא מייצר inline SVG.

Pin dependency לגרסה קבועה; אם המקורות המצורפים כבר משתמשים ב־3.48.0, ניתן לשמור אותה כדי לשמור עקביות.

SVG:

- 24×24 viewBox.
- stroke 2.
- `currentColor`.
- aria-hidden על artwork.
- accessible name שייך לכפתור.

יש להשתמש באייקונים ככל האפשר במקום שורות של כפתורים טקסטואליים.

לדוגמה:

- Search → `IconSearch`
- Settings → `IconSettings`
- Index → `IconDatabase`
- Folder → `IconFolder`
- Add folder → `IconFolderPlus`
- Text → `IconFileText`
- Code → `IconCode`
- Image → `IconPhoto`
- Audio → `IconMusic`
- Video → `IconVideo`
- PDF → `IconFileTypePdf`
- Filter → `IconAdjustmentsHorizontal`
- Pause → `IconPlayerPause`
- Resume → `IconPlayerPlay`
- Cancel/stop → `IconPlayerStopFilled`
- Rescan → `IconRefresh`
- Open → `IconExternalLink`
- Reveal in Explorer → `IconFolderOpen`
- Copy path → `IconCopy`
- More → `IconDots`
- Performance → `IconBolt`
- CPU → `IconCpu`
- Storage → `IconDeviceFloppy` או `IconDatabase`
- Privacy → `IconShieldLock`
- About → `IconInfoCircle`
- Theme → sun/moon/device icons.

Primary flows שבהם טקסט נחוץ יכולים להשתמש ב־icon + label.

Toolbar actions ברורות צריכות להיות icon-only עם accessible label.

אל תוסיף tooltip לפעולה מובנת מאליה כמו Copy אם היא מופיעה בהקשר ברור; כן הוסף tooltip כשמשמעות האייקון אינה ברורה.

אין CDN.

כל האייקונים נארזים מקומית.

ל־Windows `.ico` של ה־EXE מותר לרנדר build-time אייקון מתאים מתוך Tabler SVG, למשל `file-search`, עד שיהיה logo ייעודי.

---

# 5. Technology Stack

השתמש ב:

## Desktop shell

- Tauri 2.
- React.
- TypeScript.
- Vite.

## Native/backend

Rust.

Rust אחראי על:

- filesystem.
- index lifecycle.
- metadata.
- SQLite.
- FTS.
- vector index.
- Windows integration.
- process lifecycle.
- scheduling.
- file extraction orchestration.
- incremental indexing.
- watcher.
- query fusion.
- preview metadata.
- app settings.

## Inference

EmbeddingGemma 2 בלבד כמנוע embedding ראשי.

מודל:

`EmbeddingGemma 2 740M`

בפורמט LiteRT-LM הרשמי והמותאם ל־on-device.

Canonical model source:

`litert-community/embeddinggemma-2-740m-litert-lm`

יש להשתמש בקובץ `.litertlm` הרשמי.

אין להוריד אותו בהפעלה הראשונה.

הוא חייב להיות חלק מהמתקין.

יש לקבע revision/hash בעת packaging ולשמור SHA-256 manifest.

---

# 6. למה LiteRT-LM

LiteRT-LM הוא backend ברירת המחדל מכיוון שהוא מאפשר:

- CPU.
- GPU.
- NPU.
- Windows.
- model quantized קטן יחסית.
- multimodal input.
- on-demand modality loading.
- offline operation.
- low latency.

אין לבנות את המוצר כך שיהיה קשור ישירות ל־LiteRT internals.

צור interface ברור:

`EmbeddingBackend`

למשל:

- `initialize`
- `shutdown`
- `capabilities`
- `embedText`
- `embedCodeQuery`
- `embedImage`
- `embedAudio`
- `embedVideo`
- `embedComposite`
- `benchmark`
- `healthCheck`

כך אפשר יהיה בעתיד להחליף backend בלי לשנות UI/indexer/search.

---

# 7. Inference Host

מומלץ לא להכניס את כל LiteRT bindings ישירות ל־React או ל־Rust אם הדבר הופך את הפרויקט לשביר.

בנה process מקומי מבודד:

`smarti-local-search-inference.exe`

מומלץ Python + LiteRT-LM, נארז כ־standalone sidecar.

ה־sidecar צריך להיטען אוטומטית על ידי Tauri/Rust.

אין לפתוח HTTP port אם אין צורך.

העדף IPC מקומי.

מומלץ:

framed MessagePack על stdin/stdout או named pipe.

אין להשתמש ב־JSON עבור matrices גדולים של embeddings.

Protocol:

request ID.

operation.

arguments.

response ID.

status.

timings.

embedding matrix כ־binary contiguous floats.

ה־sidecar חייב להיות restartable אם הוא קורס.

Rust צריך לזהות crash, לבצע restart פעם אחת וליפול ל־CPU אם backend מואץ נכשל.

אין firewall prompt.

אין listen על network interfaces.

---

# 8. Packaging של inference runtime

המשתמש לא צריך להתקין:

- Python.
- pip.
- Hugging Face.
- CUDA Toolkit.
- Git.
- LiteRT.
- FFmpeg.

הכול מגיע עם המוצר.

נסה קודם ליצור inference sidecar עם PyInstaller/Nuitka במצב onedir.

אם LiteRT native dependencies אינם נארזים בצורה אמינה, השתמש ב־self-contained Python distribution שנארז במתקין.

אין להפוך זאת לדרישה ידנית מהמשתמש.

---

# 9. מודל EmbeddingGemma 2

EmbeddingGemma 2 ממפה:

- text.
- code.
- image.
- audio.
- video.
- composite/interleaved content.

לאותו vector space.

Native output:

768 dimensions.

Supported MRL:

- 768
- 512
- 256
- 128

ברירת מחדל של Smarti Local Search:

**256 dimensions**

לאחר truncation יש לבצע L2 normalization.

שמור corpus/query באותו dimension.

Advanced setting:

- Compact — 128
- Balanced — 256
- Quality — 512
- Maximum — 768

ברירת מחדל: Balanced 256.

החלפת dimension דורשת full semantic re-index.

הצג זאת למשתמש לפני שינוי.

---

# 10. Task prompts

טקסט רגיל:

Corpus document:

`title: {filename/title} | text: {content}`

Query:

SearchQuery / equivalent official LiteRT prefix.

Code:

השתמש ב־CodeRetrieval formatting/prompt המומלץ על ידי EmbeddingGemma 2.

Filename צריך לשמש title לקוד.

אל תשתמש באותו task prefix בצורה עיוורת לכל modality.

Image/audio/video עוברים ללא text task prefix אלא אם נבנה composite input מכוון.

---

# 11. Precision

כאשר LiteRT official quantized model משמש ל־inference, השתמש בייצוג הנתמך בו.

Vector שנשמר באינדקס יכול להיות converted ל־float16 לאחר L2 normalization כדי לחסוך מקום, אם vector backend תומך בכך.

Similarity:

cosine.

אין לשמור normalized vector ואז לנרמל בצורה שגויה שוב.

---

# 12. Hardware Detection

בפעם הראשונה:

זהה:

- CPU.
- core/thread count.
- total RAM.
- GPU adapters.
- NPU availability.
- available LiteRT backends.
- battery/power status.

אל תניח ש־GPU/NPU נתמך רק לפי שם החומרה.

יש לבדוק בפועל:

1. engine initialization.
2. small inference.
3. output finite.
4. vector dimension correct.
5. latency.
6. stability.

Backend שנכשל מסומן unavailable.

Fallback תמיד ל־CPU.

---

# 13. Hardware Auto Benchmark

בעת first run הרץ benchmark קצר.

אל תציג benchmark ארוך שחוסם onboarding.

בדוק לכל backend אפשרי:

- warm-up.
- query latency.
- text throughput.
- image latency אם Vision יופעל.
- audio latency אם Audio יופעל.

בדוק concurrency:

- 1
- 2
- 4
- ואם הגיוני 8.

אם runtime exposes native batch input, בדוק:

- batch 1
- 4
- 8
- 16
- 32

ואל תבחר batch גדול יותר אם latency/RAM מתדרדרים.

שמור תוצאות לפי:

- model hash.
- app version.
- hardware fingerprint.
- driver/runtime version.

אין להריץ benchmark מחדש בכל startup.

הוסף Settings → Performance → "הרץ בדיקת ביצועים מחדש".

---

# 14. Batching / concurrency

Embedding איננו autoregressive LLM.

בנה:

`EmbeddingScheduler`

הוא צריך:

- לאסוף jobs.
- לקבץ לפי modality.
- לקבץ לפי approximate input length.
- לתת priority לשאילתות חיפוש.
- לבצע bulk indexing ב־batches או controlled concurrency.
- לא לטעון עותקים רבים של המודל ללא סיבה.

אם LiteRT version הנוכחי מספק native batched embeddings:

השתמש בו.

אם לא:

אל תזייף native batching.

השתמש ב־auto-tuned concurrency.

ניתן להריץ כמה requests במקביל רק אם benchmark מראה רווח.

---

# 15. Query priority

חיפוש משתמש תמיד קודם ל־background indexing.

אם המשתמש מתחיל חיפוש בזמן אינדוקס:

- אל תעצור את כל האינדקס.
- סיים את היחידה הקטנה הפעילה.
- תן query priority.
- החזר לתור האינדוקס.

הממשק צריך להרגיש מיידי גם בזמן initial scan.

---

# 16. Dual engine optimization

אם יש מספיק RAM וה־runtime מאפשר:

אפשר לטעון:

### Query engine

Optimized for very short text input.

מוכן מראש בזמן שהאפליקציה/Tray פעילים.

### Bulk indexing engine

Optimized for chunks ארוכים יותר/media.

אם זיכרון נמוך או initialization שני נכשל:

השתמש ב־single engine.

הבחירה Auto.

אין לכפות שתי טעינות מודל.

---

# 17. Index Architecture

הפרד בין:

### Metadata/lexical database

SQLite.

### Semantic vectors

USearch/HNSW local index.

מומלץ:

`usearch`

או equivalent robust local HNSW implementation.

אין שרת Vector DB.

אין Docker.

אין Qdrant service.

---

# 18. SQLite

שמור לפחות:

## files

- internal ID
- volume ID
- stable file ID אם NTFS מאפשר
- canonical path
- display path
- filename
- extension
- detected MIME/type
- size
- created
- modified
- last indexed
- content hash
- current state
- semantic coverage
- error state
- root ID
- removable/offline status.

## chunks

- chunk ID
- file ID
- sequence
- modality
- semantic task
- text excerpt/content
- heading/symbol
- page
- line start/end
- timestamp start/end
- content hash
- embedding ID.

## roots

Indexed locations.

## jobs

Persistent indexing queue/state.

## settings

Non-secret app settings.

## accelerator benchmark

Measured capabilities.

## index schema/model info

- model identifier.
- model checksum.
- dimensions.
- schema version.

---

# 19. FTS

השתמש ב־SQLite FTS5 עבור:

- filename.
- path.
- extracted text.
- code.
- headings.
- symbols.

Metadata search צריך לעבוד גם אם semantic model עדיין לא נטען.

חיפוש filename/path צריך להיות מהיר מאוד.

---

# 20. Vector indexes

מומלץ להפריד:

`general.usearch`

ו־

`code.usearch`

General:

- documents.
- notes.
- PDF.
- Office text.
- images.
- audio.
- video.
- visual pages.

Code:

code chunks עם CodeRetrieval embedding.

אפשר להוסיף namespaces נוספים בעתיד, אבל אין צורך לפצל לפי כל file type.

HNSW defaults יכולים להיות בערך:

- M: 16
- efConstruction: 128
- efSearch: 64–100

אבל יש benchmark ולבחור ערכים הגיוניים.

האינדקס חייב לתמוך ב־add/delete/update ללא full rebuild על כל שינוי.

---

# 21. Hybrid Search

Smart Search ברירת מחדל משלב:

1. exact filename/path matching.
2. FTS/BM25.
3. semantic ANN.
4. lightweight metadata ranking.

אל תחזיר vector similarity בלבד.

בצע fusion, למשל Reciprocal Rank Fusion או normalized weighted fusion.

שם קובץ מדויק צריך לקבל boost משמעותי.

Quoted exact phrase צריך לקבל boost.

Path match צריך לקבל boost.

Semantic relevance נשאר המרכיב המרכזי כשאין match מילולי.

אל תיתן ל־"חדש יותר" לנצח relevance משמעותית רק בגלל date.

---

# 22. Grouping

Corpus מחולק ל־chunks, אך תוצאות ברירת מחדל מוצגות לפי קובץ.

קובץ אחד לא צריך למלא את כל עשרת המקומות הראשונים.

לכל file result:

- השתמש ב־best matching chunk.
- הצג אפשרות "עוד התאמות בקובץ".
- שמור 1–3 matches פנימיים.

---

# 23. Search pipeline

בזמן הקלדה:

### Immediate pass

הפעל filename/path/FTS ללא AI.

### Semantic pass

Debounce קצר, בערך 100–150ms.

בטל queries שהתיישנו.

Embed query.

ANN.

Fusion.

עדכן results.

אל תיתן לתוצאות לקפוץ בצורה פראית.

נסה לשמור selected result אם הוא עדיין קיים.

---

# 24. Query syntax

Smarti Local Search צריך לעבוד גם ללא syntax.

אבל תמוך ב:

`type:pdf`

`type:image`

`type:video`

`type:audio`

`type:code`

`ext:rs`

`ext:docx`

`path:"C:\Projects"`

`name:report`

`after:2026-01-01`

`before:2026-10-01`

`size:>10MB`

Quoted exact phrases.

Parsed filters יכולים להפוך ל־chips בחיפוש המתקדם.

הסר את חלקי filter מהטקסט שנשלח ל־semantic model.

---

# 25. All files — משמעות

"כל הקבצים" פירושו:

כל קובץ נגיש למשתמש מקבל metadata entry אם הוא נמצא ב־scope.

קובץ עם format נתמך מקבל בנוסף content extraction + semantic indexing.

קובץ binary לא נתמך עדיין ניתן למציאה לפי:

- filename.
- path.
- extension.
- metadata.

אין לנסות להפוך DLL/EXE/random binary ל־UTF-8.

---

# 26. Index Scope

Onboarding מציע:

### Recommended

"הקבצים שלי"

כולל:

- Desktop.
- Documents.
- Downloads.
- Pictures.
- Videos.
- Music.
- common development folders אם התגלו.

### Entire computer

כל הכוננים המקומיים הנגישים.

### Custom

בחירת folders/drives.

אפשר לשנות אחר כך.

---

# 27. Default exclusions

כדי לא לבזבז שעות על junk, exclusions ברירת מחדל צריכים לכלול caches/system content בעלי ערך סמנטי נמוך.

למשל:

- Windows system caches.
- temp.
- browser caches.
- package caches.
- recycle bin.
- thumbnails cache.
- huge dependency directories כמו `node_modules` כברירת מחדל.
- `.git/objects`.
- build outputs.
- IDE cache.
- virtual environments לפי בחירה.

אבל:

אפשר לבטל exclusions.

Source code project עצמו כן נסרק.

אין לדלג אוטומטית על כל hidden file.

Sensitive files כגון credentials/.env/private keys יכולים להיות excluded כברירת מחדל עם אפשרות opt-in מפורשת.

---

# 28. Symlinks/Junctions

מנע loops.

אל תסרוק את אותו physical directory שוב ושוב בגלל junction.

כבד root boundaries.

---

# 29. Fast filesystem discovery

Windows/NTFS fast path:

השתמש ב־USN Journal / NTFS APIs כאשר הדבר אפשרי.

מומלץ:

`FSCTL_ENUM_USN_DATA`

ל־initial enumeration.

שמור:

- volume.
- journal ID.
- last USN.

אם fast path לא זמין:

fallback ל־parallel recursive enumeration באמצעות Rust.

אין לדרוש Administrator רק בשביל החיפוש.

AccessDenied אינו crash.

---

# 30. Incremental indexing

לא לבצע full scan בכל startup.

לאחר initial indexing:

- Windows file notifications.
- USN journal catch-up.
- modified file detection.
- create.
- rename.
- delete.

עבור non-NTFS/network:

watcher + periodic reconciliation.

אם האפליקציה הייתה סגורה:

USN checkpoint צריך לאפשר catch-up.

---

# 31. Change detection

לפני re-embedding:

בדוק:

- stable file ID.
- size.
- mtime.

בעת extraction חשב BLAKE3 content hash.

אם timestamp השתנה אבל content hash זהה:

אל תחשב embedding מחדש.

חשב chunk hashes.

במסמך שעבר שינוי קטן:

נסה לעשות reuse ל־embeddings של chunks שלא השתנו.

---

# 32. Crash resilience

Indexing state נשמר.

אם התוכנה נסגרת:

בהפעלה הבאה המשך מן המקום הסביר האחרון.

אין להשאיר DB חצי מושחת.

השתמש ב־transactions.

Semantic updates צריכים להיות recoverable.

---

# 33. File extraction pipeline

Pipeline:

Discovery  
→ classification  
→ extraction  
→ chunking  
→ embedding queue  
→ vector store  
→ metadata/FTS commit

השלבים עובדים במקביל.

Extraction יכול להשתמש ב־Rayon/thread pool.

Inference worker נפרד.

DB writes נאספים ל־batches/transactions.

---

# 34. Text file support

כלול לפחות:

- `.txt`
- `.md`
- `.markdown`
- `.rst`
- `.log`
- `.ini`
- `.cfg`
- `.conf`
- `.toml`
- `.yaml`
- `.yml`
- `.json`
- `.jsonl`
- `.xml`
- `.csv`
- `.tsv`
- `.sql`
- `.html`
- `.htm`
- `.css`
- `.scss`
- `.less`
- `.rtf`, אם parser אמין.
- `.eml`, לפחות text subject/body.

זהה encoding.

תמוך UTF-8/UTF-16 ו־common Windows encodings.

אל תקרוס על invalid byte sequences.

---

# 35. Code support

לפחות:

- Python
- JavaScript
- TypeScript
- JSX
- TSX
- Rust
- Go
- Java
- Kotlin
- C
- C++
- C#
- PHP
- Ruby
- Swift
- Dart
- Shell
- PowerShell
- BAT/CMD
- HTML
- CSS
- SCSS
- Vue
- Svelte
- SQL
- YAML
- JSON
- TOML

השתמש ב־Tree-sitter לגרסאות/שפות מעשיות.

Chunk code לפי:

- class
- function
- method
- module
- type/interface
- meaningful top-level block.

אם parser לא קיים:

fallback line-aware chunker.

---

# 36. Text chunking

אל תטמיע מסמך שלם כווקטור יחיד.

Target:

כ־300–500 tokens.

Hard target בערך 512 tokens כשאפשר.

Overlap:

כ־50–80 tokens.

כבד:

- paragraphs.
- headings.
- sentence boundaries.
- lists.

אל תחתוך באמצע surrogate/UTF sequence.

Long section יכול להתחלק.

Short neighboring sections יכולים להתאחד.

---

# 37. Code chunking

Code chunk צריך לכלול context מועיל.

לדוגמה:

- filename.
- class name.
- function signature.
- implementation.

אם function גדולה מדי:

split לפי blocks/lines.

Filename משמש title.

אין overlap גדול שמכפיל אינדקס ללא צורך.

---

# 38. PDF

PDF text-based:

- extract text.
- preserve page number.
- chunk page-aware.

PDF scanned/image-heavy:

אם Vision indexing מופעל:

render pages כתמונות והטמע page image.

כך scanned PDF עדיין searchable semantically.

ל־PDF עם טקסט ותמונה אפשר ליצור:

- text chunks.
- visual page embedding.

אל תיצור כפילות UI מבלבלת; שתיהן מתמזגות לתוצאת file אחת.

השתמש ב־PDFium או פתרון מקומי אמין שנארז עם התוכנה.

---

# 39. DOCX

Extract:

- title.
- headings.
- paragraphs.
- tables.
- lists.

שמור section context.

אין צורך ב־Microsoft Word מותקן.

---

# 40. PPTX

Extract לפי slide.

לכל slide:

- slide number.
- text.
- speaker notes אם זמין.

אין צורך ב־PowerPoint.

---

# 41. XLSX/ODS

Extract sheet-aware blocks.

אל תהפוך workbook עצום למחרוזת אחת.

שמור:

- sheet name.
- row range.
- meaningful cell content.

הימנע מהטמעת מיליון תאים ריקים.

---

# 42. Images

Formats לפחות:

- JPG/JPEG.
- PNG.
- WEBP.
- BMP.
- TIFF.
- GIF.

כל תמונה מקבלת visual embedding.

Metadata FTS:

- filename.
- path.
- EXIF textual metadata רלוונטי אם קיים.

Animated GIF יכול להיחשב image בשלב הראשון; אופציונלית media pipeline יכול לטפל בו כסרטון.

אין צורך ליצור caption באמצעות מודל גנרטיבי.

---

# 43. Audio

Formats לפחות:

- WAV
- MP3
- M4A/AAC
- FLAC
- OGG/Opus
- WMA אם decoder תומך.

השתמש ב־FFmpeg local bundle או decoder אמין מקומי.

Normalize input כנדרש על ידי EmbeddingGemma 2, לרבות 16 kHz mono כאשר נדרש.

Segment:

ברירת מחדל 45–60 שניות.

Overlap:

5–10 שניות.

שמור timestamps.

חיפוש יכול להחזיר:

`meeting.mp3 — 00:23:40`

ולא רק את הקובץ.

---

# 44. Video

Formats לפחות:

- MP4.
- MKV.
- MOV.
- WEBM.
- AVI.
- M4V.

השתמש ב־FFmpeg local.

Segment ברירת מחדל:

30 שניות.

Overlap:

5 שניות.

Sample video בערך 1 FPS כברירת מחדל.

Balanced mode יכול להשתמש ב־vision token budget נמוך יותר הנתמך על ידי LiteRT.

כל video segment צריך לנסות לשלב:

- visual frames.
- audio.

ל־EmbeddingGemma 2 כ־composite/interleaved representation כאשר ה־API מאפשר.

אם LiteRT Python API exposes direct video content:

השתמש בו.

אם לא:

sample frames + audio והעבר כ־composite input.

שמור:

- start.
- end.
- frame range.
- audio presence.

Search result פותח video בזמן המתאים.

---

# 45. Media decode failure

קובץ codec בעייתי אינו מפיל scan.

שמור metadata.

סמן content indexing כ־failed.

אפשר Retry.

Error details במסך Activity/Diagnostics.

---

# 46. Query by media

Main search ו־Quick Search צריכים לאפשר:

- drag image.
- paste image from clipboard.
- choose image.
- choose audio.
- choose video.
- choose arbitrary indexed file as "search similar".

Image query:

embed image → search general vector index.

Audio/video ארוכים:

segment query ולבצע result fusion בין embeddings של segments במקום average עיוור אם ניתן.

---

# 47. Search UI — Main Window

המסך הראשי צריך להרגיש כמו כלי Windows מודרני, לא dashboard עסקי עמוס.

Layout RTL בעברית.

## Right navigation rail

סרגל צר בצד ימין.

אייקונים:

- Search.
- Index.
- Locations.
- Activity.
- Settings.

Settings בתחתית.

Logo/name בחלק העליון.

אפשר להרחיב labels כאשר יש מספיק מקום, אבל default compact.

## Main content

Search הוא המסך המרכזי.

---

# 48. Main Search screen

בחלק העליון:

Search field גדול ונקי.

גובה נוח.

Radius מה־design system.

מימין:

Search icon.

משמאל בתוך האזור:

- attach/example-file icon.
- clear כשהשדה אינו ריק.
- optional advanced filters icon.

Placeholder:

"חפש בכל הקבצים שלך…"

לא לכתוב הסברים ארוכים בתוך search bar.

---

# 49. Search mode

Default:

`חכם`

אפשר לבחור בתפריט קטן:

- חכם — hybrid.
- סמנטי.
- מדויק.

אל תציג שלושה כפתורים ענקיים תמיד.

---

# 50. Filters

מתחת לשורת החיפוש, רק כאשר יש filters או Advanced פתוח.

Filter chips:

- Type.
- Location.
- Date.
- Extension.
- Size.

כל chip removable.

יש "נקה מסננים" רק אם יש מסננים.

---

# 51. Results

Result list virtualized.

אל ת-render אלפי DOM nodes.

כל Result row:

- thumbnail או file-type icon מצד ימין.
- filename/title.
- matching excerpt.
- path secondary.
- page/line/timestamp כאשר רלוונטי.
- modified date/size רק אם שימושי.
- modality indicator קטן.
- actions משמאל/בריחוף.

פעולות:

- Open.
- Reveal in Explorer.
- Copy path.
- More.

אין "כפתור" טקסטואלי לכל אחת.

---

# 52. Result visual hierarchy

Filename:

הכי בולט.

Matched excerpt:

שני.

Path:

muted.

Metadata:

caption.

Semantic score אינו מוצג כברירת מחדל.

אפשר להציג Diagnostics/Advanced.

---

# 53. Exact highlights

כאשר FTS מחזיר exact terms:

highlight עדין.

Semantic-only result אינו צריך fake highlighting.

---

# 54. Preview panel

Click יחיד על תוצאה:

פותח Preview panel בצד שמאל בפריסת RTL רחבה.

Search results נשארים מימין.

Resizable divider.

Preview:

### Text

matching passage + nearby text.

### Code

syntax-friendly monospace.

matched lines.

### Image

image preview.

### PDF

page preview/extracted text.

### Audio

audio player שמתחיל סמוך ל־match.

### Video

video player שמתחיל ב־matched timestamp.

Toolbar:

- Open.
- Reveal.
- Copy path.
- Close preview.

בחלון צר:

Preview הופך למסך/panel מלא עם Back.

---

# 55. Open behavior

Double click result:

Open file עם default Windows application.

Enter על selected result:

Open.

אפשרות בהגדרות לשנות Enter ל־Preview, אבל ברירת מחדל Open.

---

# 56. Keyboard UX

Search field:

`Ctrl+K` → focus.

Results:

Arrow Up/Down.

Enter → open.

Space או Right/Left לפי RTL → preview כאשר הגיוני.

Escape:

- סוגר menu/dialog.
- אחר כך preview.
- אחר כך Quick Search overlay.

Tab order הגיוני.

אין keyboard traps.

---

# 57. Quick Search overlay

יכולת מרכזית.

Global hotkey ברירת מחדל:

`Ctrl+Alt+Space`

ניתן לשינוי.

Overlay:

- חלון קטן.
- centered horizontally.
- קרוב לחלק העליון, לא ממש במרכז המסך.
- כ־720–800px רוחב.
- rounded 16–24.
- shadow floating.
- מופיע מהר.
- focus search מיד.

מציג בערך 8 תוצאות ראשונות.

לא צריך navigation sidebar.

---

# 58. Quick Search behavior

בעת פתיחה:

search field ריק וממוקד.

המודל הטקסטואלי צריך להיות warm ככל האפשר.

בזמן הקלדה:

exact/filename results מגיעים מייד.

semantic results מצטרפים.

Arrow keys select.

Enter open.

Shift+Enter:

Reveal in Explorer.

Ctrl+Enter:

פתח Main Window עם אותה שאילתה.

Esc:

סגור overlay.

---

# 59. Quick Search image input

אפשר:

- paste screenshot.
- drag image.
- click image/file icon.

הקלט הופך ל־chip/preview קטן.

לא צריך text query יחד, אבל אפשר לתמוך בשילוב text + image אם backend מאפשר.

---

# 60. Search empty state

לפני שיש query:

אל תמלא dashboard בכרטיסים.

אפשר להציג:

- "חפש לפי משמעות, שם, תוכן או תמונה"
- כמה shortcuts עדינים.
- מצב האינדקס בשורה שקטה.

אם indexing עדיין מתבצע:

"42,813 קבצים כבר זמינים לחיפוש · האינדוקס ממשיך ברקע"

---

# 61. Search during first scan

ברגע שיש metadata ראשון:

filename search עובד.

ברגע שיש vectors ראשונים:

semantic search עובד על מה שכבר נסרק.

אין מסך blocking "חכה לסיום האינדוקס".

---

# 62. Index screen

Header:

"אינדקס"

Summary:

- discovered files.
- metadata indexed.
- semantically indexed.
- current stage.
- errors.
- index size.
- last update.

Real progress בלבד.

---

# 63. Index stages

שלבי מוצר:

1. File discovery.
2. Text & documents.
3. Code.
4. Images / visual documents.
5. Audio.
6. Video.

אפשר לעבוד במקביל במידה סבירה, אך UI מציג stage ברור.

Priority:

טקסט וקוד לפני media כברירת מחדל כדי לתת value מהר.

---

# 64. Index controls

Icon/compact actions:

- Pause.
- Resume.
- Stop.
- Rescan.
- Rebuild.

Rebuild full index דורש confirmation.

Pause אינו מוחק תור.

Stop initial scan שומר מה שכבר נבנה.

---

# 65. Index metrics

הצג בזמן index:

- files/sec.
- chunks/sec.
- embeddings/sec.
- active backend.
- accelerator.
- concurrency/batch.
- current modality.

ETA רק אם יש מספיק נתונים יציבים.

אל תציג ETA מזויף.

---

# 66. Details panel

Technical details מאחורי expand:

- backend.
- hardware.
- model.
- vector dimension.
- HNSW stats.
- active queue.
- extraction workers.
- inference workers.
- average latency.

לא במסך הראשי כברירת מחדל.

---

# 67. Locations screen

רשימה של roots.

לכל location:

- folder/drive icon.
- path.
- status.
- number of files.
- indexed percentage.
- estimated/current index size.
- menu.

Actions:

- Add folder.
- Add drive.
- Remove from index.
- Rescan.
- Edit exclusions.

---

# 68. Removable drives

תמוך בכוננים removable.

זהה drive identity.

כאשר אינו מחובר:

results יכולים להיות מוסתרים כברירת מחדל.

אפשר toggle:

"הצג קבצים מכוננים שאינם מחוברים".

אם מוצגים:

badge עדין "לא מחובר".

---

# 69. Network drives

אפשר support ל־UNC/network drives.

ברירת מחדל off.

הסבר:

"סריקה ברשת עלולה להיות איטית".

אין USN assumptions.

השתמש watcher/polling מתאים.

---

# 70. Activity screen

מכיל:

- current indexing jobs.
- recent completed scans.
- skipped files.
- extraction errors.
- model/backend fallbacks.

לא log dump כברירת מחדל.

Error row:

filename/path + reason + retry.

אפשר:

"ייצא דוח אבחון".

---

# 71. Settings IA

Settings categories:

### General

- Start with Windows.
- Minimize to tray.
- Close behavior.
- Language.
- global shortcut.

### Search

- Smart/Semantic/Exact default.
- results count.
- search-as-you-type.
- preview behavior.
- offline-drive results.

### Indexing

- modalities enabled.
- exclusions.
- follow removable drives.
- network drives.
- pause on battery.
- pause on battery saver.
- pause during gaming/fullscreen optional.
- media segment sizes under Advanced.

### Performance

- Auto.
- Quiet.
- Balanced.
- Fast.
- accelerator selection.
- benchmark.
- CPU thread limit.
- inference concurrency.
- advanced batch controls.

### Storage

- index path.
- current database size.
- cache size.
- vector quality/dimension.
- compact index.
- rebuild.
- clear all.

### Appearance

- System.
- Light.
- Dark.
- text size.
- reduced motion.

### Windows Integration

- global quick search.
- Explorer context menu.
- Start with Windows.
- system tray.
- notifications when initial indexing completes.

### Privacy

Explain clearly:

"כל החיפוש והאינדוקס מתבצעים במחשב שלך."

No cloud.

No upload.

No telemetry.

Show indexed roots and sensitive exclusions.

### Advanced

- model details.
- backend details.
- index schema.
- debug logging.
- reset app settings.

### About

- app version.
- EmbeddingGemma 2 model/version.
- licenses.
- third-party notices.

---

# 72. Performance Profiles

## Quiet

- low CPU.
- low concurrency.
- pause more aggressively on battery.
- media indexing lower priority.

## Balanced

Default.

Use moderate CPU.

Try accelerated backend.

No visible UI stutter.

## Fast

Use most available CPU/accelerator.

Higher concurrency.

Warn only if running on battery.

## Auto

Benchmark chooses parameters based on machine.

Auto is recommended and initial default.

---

# 73. Search latency goals

These are engineering goals, not hard guarantees.

Warm filename/FTS:

preferably <50ms.

Semantic text query:

aim <250ms even on ordinary CPU where possible.

Accelerated hardware:

aim <100ms.

UI should never wait on semantic results before showing exact results.

---

# 74. Indexing performance

Do not hardcode an advertised files/second number.

Measure current hardware.

Show actual throughput.

Optimization priorities:

1. avoid duplicate disk reads.
2. parallel extraction.
3. incremental chunking.
4. grouped embeddings.
5. batch/concurrency tuning.
6. batched DB writes.
7. incremental updates.
8. skip unchanged data.

---

# 75. Media quality profiles

Balanced:

- 256d.
- 70-ish vision soft-token budget where supported.
- 1 FPS video.
- 30s video windows.
- ~60s audio windows.

Quality:

- 512d.
- higher vision token budget.
- potentially higher frame rate where useful.

Compact:

- 128d.
- primarily recommended for text-heavy huge indexes.
- warn that multimodal quality is reduced.

Maximum:

768d.

---

# 76. Storage

Default data directory:

`%LOCALAPPDATA%\Smarti Local Search\`

Suggested:

`data\metadata.sqlite`

`data\vectors\general.usearch`

`data\vectors\code.usearch`

`cache\thumbnails\`

`cache\media\`

`logs\`

Index location should be configurable.

App binaries/model stay in installation directory.

---

# 77. Cache policy

Thumbnails/media previews:

bounded cache.

Default max e.g. 512MB–1GB.

LRU cleanup.

No duplicate permanent copies of user media.

Temp transcoding removed after use.

---

# 78. Index compacting

Deleted/updated vectors may create tombstones.

Track ratio.

When threshold becomes high, compact/rebuild vector index in background.

Search stays available where possible.

לא לבצע rebuild כל יום ללא צורך.

---

# 79. Windows integration

Implement:

- single app instance.
- system tray.
- global quick-search hotkey.
- Start with Windows optional.
- open file.
- reveal in Explorer.
- copy path.
- taskbar activation.
- Windows notifications sparingly.
- per-user install preferred.

---

# 80. Explorer context menu

Optional setting:

"הוסף 'חפש עם Smarti Local Search' לסייר הקבצים"

Register under HKCU, ללא admin.

For folder context:

פתח Main Search constrained to selected folder.

For file context:

אפשר:

"חפש קבצים דומים"

כאשר הקובץ הוא modality נתמך.

Uninstall חייב להסיר registration.

---

# 81. Custom protocol

אפשר ליצור:

`smartilocal://`

לשימוש פנימי עתידי.

לדוגמה:

`smartilocal://search?...`

אבל אין expose לא מאובטח.

Validate all paths/arguments.

---

# 82. Tray

Tray menu:

- חיפוש מהיר.
- פתח Smarti Local Search.
- Pause/Resume indexing אם פעיל.
- מצב האינדקס.
- יציאה.

Close button behavior:

ברירת מחדל close window, app יכול להמשיך ב־tray אם Start/Quick Search פעילים.

הגדרה ניתנת לשינוי.

---

# 83. Power awareness

Windows battery status.

Balanced:

כאשר Battery Saver פעיל:

pause heavy media indexing.

Text incremental updates יכולים להמשיך במינון נמוך.

Fast mode:

כבד user preference.

---

# 84. Responsiveness

Indexing לעולם לא רץ על UI thread.

File extraction לעולם לא חוסם React.

Search calls cancellable.

Preview generation cancellable.

Rust event stream updates UI at reasonable frequency, לא מאות events בשנייה.

Throttle progress UI updates ל־5–10/sec לכל היותר.

---

# 85. RTL

Hebrew UI default.

Physical RTL אמיתי.

נתיבים:

`dir=ltr` / `<bdi>` לפי הצורך.

Code:

LTR.

Numbers/percentages מבודדים.

English filenames לא שוברים layout.

Windows titlebar buttons נשארים בהתנהגות native הצפויה.

---

# 86. English support

בנה i18n architecture פשוטה כבר עכשיו.

Hebrew default.

English translation file.

אין hardcoded Hebrew scattered בכל component.

לדוגמה:

`src/i18n/he.ts`

`src/i18n/en.ts`

---

# 87. Accessibility

WCAG-oriented.

- visible keyboard focus.
- correct aria labels.
- minimum target sizes.
- dialogs trap focus.
- Escape closes.
- menu keyboard navigation.
- no hover-only critical action.
- reduced motion.
- forced colors sanity.
- 100/125/150/200% DPI.
- screen-reader labels for icon-only buttons.

---

# 88. Window sizes

Main window:

Recommended initial around 1200×780.

Remember position/size.

Reasonable minimum around 820×560.

Below desktop-friendly width:

collapse navigation and preview intelligently.

אין לחתוך search/actions.

Quick Search window has fixed-ish max width אבל מתאים למסכים קטנים.

---

# 89. Themes

- System.
- Light.
- Dark.

System follows Windows theme.

No restart.

Persist locally.

Use supplied design tokens.

---

# 90. Preview security

לעולם אל תציג raw HTML file ישירות עם scripts פעילים.

HTML preview sanitized/plain.

Markdown sanitized.

Code escaped.

PDF rendered safely.

Media uses local file protocol with allowlisted paths.

No arbitrary remote web content.

---

# 91. Privacy and security

No network required for normal operation.

No remote analytics.

No content logs.

Log only technical metadata necessary for debugging.

Do not log extracted file contents.

Do not log embeddings.

Diagnostic export may contain paths; warn user before export.

Database files accessible only to current Windows user according to normal filesystem ACL.

---

# 92. Sensitive files

Default exclusion suggestions:

- private key formats.
- credential stores.
- browser password databases.
- OS credential files.

Do not claim that local semantic indexing itself is a security boundary.

Allow advanced user to opt in.

---

# 93. FFmpeg

Bundle an appropriate legal redistributable build.

Prefer LGPL-compatible build where possible.

Include required notices.

No dependency on system-installed FFmpeg.

---

# 94. Third-party licenses

Include:

- Tabler MIT notice.
- LiteRT/LiteRT-LM notices.
- EmbeddingGemma 2 Apache 2.0 notice and applicable policy information.
- FFmpeg notices.
- PDFium if used.
- Rust/npm/Python dependency notices as required.

Do not arbitrarily choose a license for Smarti Local Search itself.

Only document third-party licenses unless repository already defines a project license.

---

# 95. Installer

Create a proper Windows installer `.exe`.

Recommended Tauri NSIS.

Per-user install by default.

No admin required for normal install.

Installer must include:

- app frontend.
- Rust backend.
- inference host.
- LiteRT runtime dependencies.
- EmbeddingGemma 2 `.litertlm`.
- FFmpeg.
- PDF runtime if needed.
- Tabler assets.
- license notices.
- required native DLLs.

No first-run model download.

---

# 96. Offline installer

עדיף שהמתקין יעבוד גם במחשב ללא אינטרנט.

Bundle WebView2 offline/fixed runtime if required by chosen Tauri configuration, או השתמש בדרך Tauri אמינה שמונעת תלות בהורדה בזמן ההתקנה.

המטרה:

Fresh Windows installation → installer → application works.

---

# 97. Model integrity

Packaging script:

download/fetch model only at build time.

Verify exact SHA-256.

Generate:

`resources/models/model-manifest.json`

עם:

- model id.
- source revision.
- file name.
- sha256.
- size.
- license.

At startup:

verify existence.

Full checksum can run first install/when version changes rather than every launch אם expensive.

אם model corrupted:

הצג repair/error ברור.

אל תנסה silently cloud download.

---

# 98. Model lifecycle

Load text capability early.

Vision/audio lazily לפי צורך אם runtime supports on-demand modality loading.

כאשר no indexing/search במשך זמן ממושך:

אפשר לשחרר heavy modality resources.

Text query engine יכול להישאר resident כאשר Quick Search enabled.

הוסף setting:

"Keep search ready in memory"

Default on כאשר RAM ≥8GB.

---

# 99. Memory pressure

אם Windows reports low memory:

- reduce concurrency.
- unload media encoder when idle.
- pause media indexing.
- avoid second engine.
- keep UI/search responsive.

אין OOM retry loop.

---

# 100. Search result persistence

Search state יכול להישמר בזמן מעבר Settings/Index ולחזור אליו.

אין צורך לשמור search history כברירת מחדל.

אפשר setting אופציונלי:

"שמור חיפושים אחרונים"

Default off או מינימלי מטעמי פרטיות.

---

# 101. Index health

Settings/Index screen:

- model version.
- index schema.
- dimension.
- file count.
- chunk count.
- vector count.
- DB size.
- vector size.
- last successful reconciliation.
- watcher state.

Action:

"בדוק תקינות אינדקס".

---

# 102. Re-index triggers

Full semantic re-index נדרש אם:

- model changes incompatibly.
- vector dimension changes.
- task formatting changes materially.
- index schema incompatible.

אין reindex בגלל UI update.

---

# 103. Migration

Store schema version.

SQLite migrations transactional.

Vector index version metadata.

אם migration impossible:

offer rebuild semantic index while retaining settings/root configuration.

---

# 104. Project structure

Recommended:

```text
smarti-local-search/
├─ desktop/
│  ├─ src/
│  │  ├─ design-system/
│  │  ├─ components/
│  │  ├─ features/
│  │  │  ├─ search/
│  │  │  ├─ quick-search/
│  │  │  ├─ indexing/
│  │  │  ├─ locations/
│  │  │  ├─ activity/
│  │  │  └─ settings/
│  │  ├─ i18n/
│  │  └─ app/
│  └─ src-tauri/
├─ crates/
│  ├─ search-core/
│  ├─ indexer/
│  ├─ extractors/
│  ├─ vector-store/
│  ├─ metadata-store/
│  └─ windows-integration/
├─ inference-host/
│  ├─ src/
│  ├─ requirements.lock
│  └─ packaging/
├─ resources/
│  ├─ models/
│  ├─ ffmpeg/
│  ├─ pdfium/
│  └─ licenses/
├─ scripts/
├─ tests/
├─ docs/
└─ README.md
```

אפשר לשנות names, אבל שמור separation דומה.

---

# 105. Frontend state

השתמש ב־state management קטן וברור.

Zustand מותר.

אל תכניס Redux אם אין צורך.

Server/native state מגיע מ־Tauri commands/events.

אין duplication של DB state ב־localStorage.

---

# 106. UI virtualization

השתמש ב־`@tanstack/react-virtual` או equivalent עבור:

- result lists.
- error lists.
- large location/file lists.

---

# 107. Backend commands

Expose typed Tauri commands/events.

דוגמאות:

- `search`
- `cancel_search`
- `get_preview`
- `get_index_status`
- `start_index`
- `pause_index`
- `resume_index`
- `stop_index`
- `rebuild_index`
- `add_root`
- `remove_root`
- `update_root`
- `get_activity`
- `retry_failed_item`
- `get_settings`
- `update_settings`
- `run_benchmark`
- `get_hardware_info`
- `open_path`
- `reveal_path`
- `copy_path`
- `set_global_shortcut`

Define TypeScript types from shared schemas where practical.

---

# 108. Events

Rust → frontend:

- index progress.
- index stage.
- index paused/resumed.
- file change batch.
- inference state.
- accelerator fallback.
- index complete.
- index error count changed.

Events should be coarse-grained enough for performance.

---

# 109. Search cancellation

כל search request מקבל ID.

New query cancels/invalidates old query.

Old inference response אסור שיחליף UI של query חדש.

אותו כלל ל־preview.

---

# 110. Explorer/open security

Canonicalize paths.

אל תאפשר custom protocol arbitrary command execution.

Use Windows shell APIs / Tauri opener only on validated file paths.

---

# 111. Initial onboarding

On first launch:

## Screen 1 — Welcome

Logo/name.

"כאן מחפשים לפי משמעות, לא רק לפי שם."

"כל הנתונים נשארים במחשב שלך."

Button:

"התחל"

## Screen 2 — What to index

Cards/choices:

- הקבצים שלי — recommended.
- כל המחשב.
- בחירה ידנית.

Advanced link:

exclusions.

## Screen 3 — Performance

Run short auto hardware detection/benchmark.

Show simple result:

"נמצא מאיץ GPU"

או

"נשתמש במעבד"

Do not expose raw technical dump.

Profile:

Auto — recommended.

## Screen 4 — Start

"Smarti Local Search יכול להתחיל לחפש עוד לפני שהסריקה מסתיימת."

Start indexing.

Then open main search immediately.

אין wizard של שמונה מסכים.

---

# 112. Onboarding model load

אל תציג "Downloading model".

המודל כבר מותקן.

אפשר להציג:

"מכין את מנוע החיפוש…"

רק אם initialization באמת מתבצע.

Progress צריך לשקף real steps, או spinner indeterminate.

---

# 113. Status indications

Global unobtrusive status באזור navigation/footer:

- Ready.
- Indexing.
- Paused.
- Model loading.
- Error.

אין badges צבעוניים בכל מקום.

---

# 114. Notifications

Windows toast only for meaningful events:

- initial indexing completed.
- critical indexing failure requiring user action.

No toast for every incremental update.

---

# 115. No fake progress

Never show "94%" because timer reached it.

Progress based on discovered items/work.

If denominator unknown:

indeterminate.

---

# 116. Error UX

Errors should explain:

מה קרה.

מה הושפע.

מה אפשר לעשות.

לדוגמה:

"לא ניתן היה לקרוא 14 קבצים בגלל הרשאות. שאר האינדקס תקין."

Actions:

View.

Retry.

לא modal interrupt לכל file.

---

# 117. Empty/error states

Use design-system `EmptyState`, `Alert`, etc.

אין giant illustrations.

אין paragraphs ארוכים.

---

# 118. Search-as-you-type fallback

אם semantic model עדיין initializing:

המשך exact search.

הצג indicator קטן:

"משלים חיפוש סמנטי…"

אל תנעול search field.

---

# 119. Model failure

אם GPU/NPU crashes:

1. retry engine once.
2. fallback CPU.
3. continue search/indexing.
4. show non-blocking notification.
5. record diagnostics.

User should not lose index.

---

# 120. Binary compatibility

Primary target:

Windows 10 22H2 x64 ומעלה.

Windows 11 x64.

Architecture should not make ARM64 impossible, but x64 installer is required deliverable.

---

# 121. CPU compatibility

CPU-only must work.

Do not require AVX-512.

Detect required instruction sets.

If optimized backend requires unsupported instruction:

fallback compatible CPU implementation.

---

# 122. GPU compatibility

Probe LiteRT GPU backend.

Do not hardcode NVIDIA only.

Support whatever Intel/AMD/NVIDIA combinations current LiteRT backend successfully initializes.

Failure → CPU.

---

# 123. NPU compatibility

Probe actual LiteRT NPU backend.

Likely Intel/Qualcomm depending supported hardware.

Do not show NPU option as available until inference test succeeds.

---

# 124. Separate modality backends

If LiteRT allows:

Text backend.

Vision backend.

Audio backend.

יכולים להיבחר בנפרד.

Auto benchmark יכול לבחור, לדוגמה:

- Text → NPU.
- Vision → GPU.
- Audio → NPU/CPU.

שמור profile.

---

# 125. FFmpeg workload

Decode/transcode workers צריכים להיות limited.

אל תפעיל 16 video decodes במקביל ואז תחנוק disk/CPU.

Auto tune.

Default 1–2 video workers.

Audio 2–4.

Image decoding more parallel.

---

# 126. I/O scheduling

Avoid random disk thrash.

Group scans by root/path where possible.

Don't read media files twice אם ניתן.

Extraction computes hash during same stream where practical.

---

# 127. File size limits

אין לקרוא huge file כולו ל־RAM.

Streaming parsers.

Large text file split incrementally.

Media decoded segment-by-segment.

Configurable safeguards.

Huge unsupported file still metadata-indexed.

---

# 128. Archive files

ZIP/RAR/7z internal indexing is **not required by default**.

אפשר future option.

Do not recursively explode archives in v1.

Archive itself searchable by filename/path.

---

# 129. Reparse/temporary files

Skip transient/temp files that disappear במהלך scan without noisy errors.

Retry once when appropriate.

---

# 130. Result sorting

Default:

Relevance.

Options:

- Relevance.
- Modified.
- Name.
- Size.

Semantic score remains independent.

---

# 131. Result filters

File type groups:

- Documents.
- Code.
- Images.
- Audio.
- Video.
- Other.

Quick icons.

No 40-extension checkbox wall.

Extension filter lives in Advanced.

---

# 132. "Similar files"

From result More menu:

"מצא קבצים דומים"

Use selected item's embedding.

For file with multiple chunks:

use selected matched chunk or representative embedding.

Open results as semantic similarity query.

---

# 133. Duplicate detection

Optional helpful capability:

כאשר embeddings/hashes מראים exact duplicate, אפשר badge קטן:

"כפול"

אך אין צורך לבנות duplicate manager מלא.

Content-hash cache should reuse work internally where safe.

---

# 134. File renames

On NTFS stable file identity:

rename/update path without recomputing embeddings אם content unchanged.

---

# 135. Deletions

Remove file from:

- FTS.
- metadata.
- active vector mapping.

Vector index compact later as needed.

---

# 136. Index consistency

Search should never return a file marked deleted unless result is explicitly an offline removable-drive result.

Validate before Open אם file disappeared.

---

# 137. Code search query

In Smart mode:

אם query/filter clearly code-oriented, run CodeRetrieval query embedding.

אפשר heuristics:

- code filter active.
- extension filter is code.
- query contains code-like symbols.
- terms such as function/class/method/API.
- current result scope is source folder.

אפשר להריץ general + code search ולבצע fusion אם latency סביר.

Do not force two model calls for every two-character query unless benchmark says it is cheap.

---

# 138. Multilingual

Hebrew and English searches should work transparently.

Do not translate queries through cloud services.

EmbeddingGemma 2 handles multilingual semantic representation.

FTS remains literal.

---

# 139. Thumbnails

Generate thumbnails lazily.

Do not pre-generate thumbnails for millions of files.

Use Windows thumbnail provider where practical or local decoder.

Cache bounded.

---

# 140. Search result media timeline

Audio/video result must show:

`00:14:22–00:14:52`

or representative timestamp.

Click preview starts there.

---

# 141. PDF result

Show:

`עמוד 17`

Code:

`שורות 84–121`

XLSX:

`Sheet1 · שורות 120–138`

PPTX:

`שקופית 8`

---

# 142. App icon/branding

Do not invent a completely different brand language.

Use Smarti design palette.

Name shown prominently only where needed.

Main UI should prioritize search, not logo.

---

# 143. Window title

`Smarti Local Search`

Quick Search overlay can omit standard titlebar chrome if Tauri custom window works reliably, but preserve:

- drag behavior.
- accessibility.
- close/escape.
- shadows.

Main window should retain familiar Windows window controls.

---

# 144. Updates

Auto-updater is not required for this PoC.

Architecture should not prevent adding it.

Do not add an untested updater merely for completeness.

---

# 145. Logging

Use rotating logs.

Levels:

- error.
- warn.
- info.
- debug.

Default info.

Never include raw document content.

Debug may contain paths.

Retention limited.

---

# 146. Diagnostics

Advanced diagnostics page:

- App version.
- OS.
- CPU.
- RAM.
- GPU.
- NPU/backend.
- LiteRT version.
- model checksum/version.
- query latency benchmark.
- vector DB stats.
- DB integrity result.
- FFmpeg version.
- PDF runtime version.

Buttons:

- Copy diagnostics.
- Export logs.
- Re-run benchmark.
- Check index health.

---

# 147. Testing corpus

צור `tests/fixtures/corpus/`.

כולל small synthetic:

- Hebrew txt.
- English txt.
- Markdown.
- source code.
- DOCX.
- PDF.
- image.
- audio.
- video.

No copyrighted test material.

---

# 148. Unit tests

Test:

- chunking.
- code chunking.
- path normalization.
- duplicate detection.
- dimension validation.
- L2 normalization.
- FTS queries.
- ranking fusion.
- filter parsing.
- file update.
- rename.
- delete.
- crash resume.
- settings migrations.

---

# 149. Integration tests

Test:

index fixture corpus.

Search filename.

Search exact text.

Semantic search.

Code search.

Image query.

Media timestamp indexing smoke test.

Index during search.

Pause/resume.

Delete/modify file.

Restart app/backend.

---

# 150. Frontend tests

Test:

- keyboard search.
- filters.
- quick search result selection.
- preview.
- Settings.
- dialogs.
- theme.
- RTL.
- reduced motion.
- focus restoration.
- icon-only accessible labels.

---

# 151. Build checks

At minimum:

Frontend typecheck.

Frontend production build.

Rust fmt/clippy/tests.

Inference host tests.

Integration tests.

Packaging smoke test.

`git diff --check`.

---

# 152. Visual checks

Check:

- light.
- dark.
- Hebrew RTL.
- English LTR.
- 100%.
- 125%.
- 150%.
- 200% DPI.
- narrow window.
- maximized.
- long Hebrew filenames.
- long English paths.
- mixed Hebrew/English.

---

# 153. Performance tests

Create benchmark command:

`SmartiLocalSearch.exe --benchmark`

או dev equivalent.

Measure:

- metadata enumeration.
- text extraction.
- chunking.
- embeddings/sec.
- image/sec.
- query embedding latency.
- ANN latency.
- FTS latency.
- fusion latency.
- peak RAM.

Do not bake benchmark results as product claims.

---

# 154. Search benchmark target

ANN over approximately 1M vectors should remain in millisecond-level search territory where hardware/index permits.

The model embedding call will generally dominate interactive semantic latency.

Optimize UX accordingly.

---

# 155. Large index simulation

Test at least synthetic:

- 100k vectors.

If reasonable during development:

- 1M vectors.

Measure:

memory.

search latency.

index load/startup.

Do not require loading all vectors as Python objects.

---

# 156. Index startup

Prefer memory-mapped vector index or equivalent.

No full deserialize into JS/Python.

Rust/native owns search index.

---

# 157. Background priority

Windows process/thread priorities:

indexing should run below interactive UI priority where appropriate.

Inference search query priority higher than bulk indexing.

Don't set dangerous realtime priority.

---

# 158. UI resource use

React should not poll backend every 50ms.

Use events.

Virtualize lists.

Memoize expensive view transforms.

---

# 159. Build scripts

Provide:

`dev.ps1`

`test.ps1`

`build.ps1`

`package.ps1`

`clean.ps1`

`fetch-model.ps1`

`verify-release.ps1`

One command should be able to go from prepared checkout + dependencies to release.

Document prerequisites for developer machine בלבד.

End user has no prerequisites.

---

# 160. Model fetch script

`fetch-model.ps1`:

- downloads exact pinned model revision.
- verifies checksum.
- places in resources.
- refuses wrong checksum.
- does not redownload if verified file exists.

Release packaging fails if model missing.

Do not silently create installer without model.

---

# 161. Clean build

README must document clean build from source.

No dependency on files that exist only on original developer PC.

No absolute local paths.

No Hebrew-path assumptions.

Paths with spaces/non-ASCII must work.

---

# 162. Installer verification

After packaging:

run scripted verification where possible:

- model present.
- sidecar present.
- ffmpeg present.
- licenses present.
- frontend assets present.
- executable versions correct.
- no development-only files required.
- no network model fetch URL invoked at runtime.

---

# 163. First install smoke test

From clean user data directory:

- launch.
- onboarding.
- select test folder.
- indexing.
- exact search.
- semantic search.
- Quick Search.
- restart.
- incremental modification.
- uninstall.

---

# 164. Uninstall

Uninstaller removes application binaries.

Ask/offer whether to remove local index data if installer framework supports safe choice.

Do not silently delete potentially large user-created index without warning.

Remove:

- Explorer context menu registration.
- autostart.
- protocol handler.
- global app registration.

---

# 165. Source deliverables

Final repository must include:

- complete source.
- frontend.
- Rust.
- inference host source.
- packaging scripts.
- tests.
- design system.
- third-party notices.
- model manifest.
- docs.
- installer configuration.

Do not commit secrets.

Do not commit arbitrary temp/index data.

---

# 166. Binary deliverables

Produce:

- Release EXE/app.
- Windows installer EXE.
- optional portable ZIP if straightforward.
- checksums.

Give exact output paths in final report.

---

# 167. Documentation

Create:

`README.md`

`docs/ARCHITECTURE.md`

`docs/INDEX_FORMAT.md`

`docs/INFERENCE.md`

`docs/PERFORMANCE.md`

`docs/WINDOWS_INTEGRATION.md`

`docs/DESIGN_SYSTEM.md`

`docs/DECISIONS.md`

`THIRD_PARTY_NOTICES.md`

README should be practical, not 100 pages.

---

# 168. Architecture documentation

Include Mermaid diagram:

UI  
→ Tauri/Rust  
→ Search Engine  
→ SQLite/FTS + Vector Index

and:

Indexer  
→ Extractors  
→ Embedding Scheduler  
→ Inference Host/LiteRT  
→ vector store.

---

# 169. Privacy documentation

README/About:

"Smarti Local Search indexes your selected files locally. Search queries, file contents and embeddings are not sent to a remote service."

Do not claim absolute security.

---

# 170. Index database privacy

No encryption requirement for PoC.

אבל document that the index itself can contain extracted text and therefore should be considered private user data.

Do not store it in shared/public folders by default.

---

# 171. No cloud dependency

Application must still work כאשר internet disabled.

Only development scripts may access internet to fetch dependencies/model.

Runtime should not.

---

# 172. No generative AI

Do not add chat.

Do not summarize files using an LLM.

Do not answer questions from files.

This product is search.

Future Smarti integration can consume its API.

---

# 173. Future local API

Design search core as reusable service/library.

Optionally expose disabled-by-default local IPC/API in future.

For now no public TCP server is required.

But Rust search core should not be tightly coupled to React.

---

# 174. Potential future Smarti integration

Keep interfaces clean so Smarti could later call:

`search(query, filters, topK)`

and receive:

- file.
- chunk.
- score.
- excerpt.
- page/lines/timestamp.

Do not implement Smarti integration now.

---

# 175. No external indexing service

No Windows Search dependency for semantic index.

Windows Search can optionally be consulted for metadata only, but Smarti Local Search must work independently.

---

# 176. Installer size

A large installer is acceptable because it includes the AI model and runtimes.

Do not sacrifice offline/self-contained behavior merely כדי להגיע למתקין קטן.

Still avoid duplicate model copies.

---

# 177. Model duplication

Bundle one full official 740M LiteRT model by default.

Do not bundle 270M + 440M + 740M simultaneously unless testing proves a meaningful benefit.

The full model supports on-demand modality loading.

---

# 178. Search startup

App shell should appear quickly even if model takes longer.

Initialize UI and metadata DB first.

Model initialization happens asynchronously.

Quick Search requires model prewarm once app is resident.

---

# 179. Index first value

Priority of first scan:

1. enumerate metadata.
2. filename/path searchable.
3. text.
4. code.
5. documents.
6. images.
7. audio.
8. video.

Users should get useful search within minutes, not after all videos finish.

---

# 180. Media indexing policy

Very large media libraries may take a long time.

Show estimated remaining items.

User can pause only Media stage while Text remains complete.

Allow per-modality toggle without deleting unrelated vectors.

---

# 181. Index status per file

Possible states:

- metadata_only.
- queued.
- extracting.
- embedding.
- indexed.
- partial.
- skipped.
- error.
- offline.
- deleted/tombstoned.

Do not expose all technical state names verbatim in UI.

---

# 182. Partial success

A PDF may have text indexed but visual page embedding failed.

State = partial.

Search text continues to work.

Do not discard all successful work בגלל one failed modality.

---

# 183. Atomic content updates

When modified file is re-indexed:

retain old searchable version until new chunks are ready if feasible.

Then atomically switch mappings.

Avoid period where file disappears from search unnecessarily.

---

# 184. Search scoring debug

Advanced diagnostics can show components:

- lexical.
- semantic.
- filename.
- path.
- final fused score.

Not regular UI.

Useful for tuning.

---

# 185. UX polish

No excessive cards.

No huge empty whitespace.

No shadows around every row.

No neon overload.

Premium feeling comes from:

- spacing.
- typography.
- hierarchy.
- fast response.
- subtle surfaces.
- consistent icons.
- restrained blue accent.
- smooth panels.

---

# 186. Primary buttons

Use text+icon only where action needs explicit meaning:

- "הוסף תיקייה"
- "התחל אינדוקס"
- destructive confirmations.

Repeated toolbar actions should be icons.

---

# 187. Destructive actions

Examples:

- Clear index.
- Remove location + data.
- Reset settings.

Use danger role.

Require confirmation where destructive.

Pause/Stop are not "danger red" unless they actually destroy data.

---

# 188. Dialogs

Use shared dialog primitive.

Initial focus sensible.

Escape closes unless operation cannot safely close.

Focus returns to opener.

Destructive dialog:

Cancel focused first.

---

# 189. Menus

No X inside menus.

Open button toggles.

Outside click closes.

Escape closes.

Arrow navigation.

Fit content where practical.

Viewport constrained.

---

# 190. Scrollbars

Use supplied quiet scrollbar style.

No giant custom scrollbar.

Reserve gutter when necessary to avoid layout jump.

---

# 191. Focus

Do not use glowing bright-blue input border on every focus.

Follow supplied neutral focus style.

Keyboard focus must still be visible.

---

# 192. Search field action icons

Reserve space inside input so text never goes underneath icons.

RTL:

search icon physical right.

secondary actions physical left.

Paths/content direction isolated.

---

# 193. Performance UI

Do not put 12 metrics on main screen.

At top of Index:

three or four concise values.

Technical details expandable.

---

# 194. Hardware UI

Settings Performance example:

`מצב ביצועים: אוטומטי`

`טקסט: Intel NPU`

`תמונה: GPU`

`אודיו: NPU`

Button icon:

"בדוק מחדש"

If only CPU:

do not make it look like failure.

---

# 195. Real measured values

Any number displayed as speed/latency must be from actual local measurements.

Do not display Google's benchmark as if it belongs to user's machine.

---

# 196. Search query length

Short interactive query uses smallest practical supported model input signature.

Bulk chunks use suitable larger signature.

If LiteRT API allows selecting signature/input length efficiently, exploit it.

Do not initialize an 8192-token path for every two-word search if avoidable.

---

# 197. Chunk sizing and model signatures

Target text chunk size should align with efficient supported input signature, preferably around 512 tokens.

Longer contextual chunks only when they materially improve structured document/code semantics.

---

# 198. Vision token budget

Balanced mode:

prefer lower supported vision token budget for bulk images/video frames.

Quality mode:

higher supported budget.

Do not mix different settings accidentally within one index without recording configuration/version.

---

# 199. Index config fingerprint

Store hash of:

- model version.
- dimension.
- vision token config.
- task prompts.
- chunker version.
- schema.

Used to determine whether rebuild is required.

---

# 200. Final Definition of Done

הפרויקט נחשב גמור רק כאשר:

1. יש source code מלא.
2. Tauri app נבנה ב־Release.
3. Windows installer נוצר.
4. המתקין כולל EmbeddingGemma 2.
5. אין model download בזמן runtime.
6. CPU fallback עובד.
7. GPU/NPU auto probing קיים.
8. משתמש יכול לבחור folders/drives.
9. metadata search עובד.
10. text semantic search עובד.
11. code semantic search עובד.
12. PDF/Office extraction עובד.
13. image semantic search עובד.
14. audio indexing/search עובד.
15. video segment indexing/search עובד.
16. query-by-image עובד.
17. hybrid search עובד.
18. preview עובד.
19. timestamps/pages/lines מוצגים.
20. initial indexing ניתן לעצירה/המשך.
21. incremental indexing עובד.
22. restart אינו מחייב full rescan.
23. Quick Search global shortcut עובד.
24. system tray עובד.
25. Explorer integration עובד כאשר מופעל.
26. Hebrew RTL נראה נכון.
27. Light/Dark/System עובדים.
28. Tabler SVG משמש בכל ה־UI.
29. אין buttons מדומים.
30. אין cloud calls.
31. בדיקות עיקריות עוברות.
32. build scripts מתועדים.
33. third-party licenses כלולים.
34. installer עבר smoke test.
35. final report מציין מה נבנה, מה נבדק, binaries paths וכל limitation אמיתי שנותר.

---

# 201. הוראת ביצוע אחרונה

אל תחזיר לי רק תוכנית.

בצע את התוכנית.

קרא קודם את קובצי העיצוב המצורפים.

בנה את כל הפרויקט.

הרץ בדיקות.

תקן failures סבירים.

בנה Release.

צור installer.

אל תבצע push או publish לשום repository מרוחק אלא אם התבקש במפורש.

בסיום, החזר דו"ח קצר בעברית הכולל:

- מה מומש.
- ארכיטקטורה שנבחרה.
- backend/accelerators.
- בדיקות שבוצעו ותוצאותיהן.
- גודל המודל והמתקין.
- נתיבי source/build/installer.
- limitations אמיתיים בלבד.
- הוראות קצרות להפעלה ולבנייה מחדש.