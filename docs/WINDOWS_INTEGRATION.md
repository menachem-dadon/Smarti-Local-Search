# Windows integration

MSVC runtime DLLs are deployed beside the app executable from the licensed build-tools redistributable directory. The Release import table requires MSVCP140.dll; the runtime manifest records exact hashes. Installed users do not need a separate Visual C++ runtime download or an elevated global runtime install.

Per-user NSIS installation does not require Python, Rust, Node or a separate FFmpeg/PDFium installation. Tauri bundles the offline WebView2 installer. The main window preserves Windows caption controls; a separate hidden quick window opens on configurable Ctrl+Alt+Space and Escape hides it.

Tray actions open the main/quick window, pause/resume indexing and exit. Close-to-tray is configurable. Single-instance handling forwards Explorer requests to the main window. Initial process arguments are also read after frontend listeners are registered.

Optional Explorer integration is in HKCU `Software/Classes/*/shell/SmartiLocalSearch` and `Directory/shell/SmartiLocalSearch`. Commands use the exact quoted executable and quoted file path; user input is never passed through a command shell. Autostart uses the user's Run key and `--tray`. Uninstall removes these integration keys and preserves private indexed data.

File actions resolve indexed IDs and canonical paths before ShellExecute or Explorer reveal. Clipboard paths are copied through the native plugin. Clipboard images are saved only to the private cache for query inference. Asset scope starts empty and grants only requested preview files. HTML/document source is rendered as text; PDFs render through PDFium images, never active browser documents.

Native USN journal checkpoints are used when access is available. Access-denied/non-NTFS/offline roots use metadata reconciliation and the filesystem watcher. Power policy uses GetSystemPowerStatus. Quick-window bounds use the actual monitor work area and scale factor, including high DPI and negative monitor coordinates. Oversized main windows are fitted to that work area; a saved main window that already fits is retained. Quick-window saved-state restoration is disabled.

Manual release acceptance must cover normal/maximized/relaunch, RTL/LTR, narrow/wide, light/dark/system, tray, shortcut, file opening/reveal, Explorer invocation and offline first launch. Signing and SmartScreen reputation are not configured.
