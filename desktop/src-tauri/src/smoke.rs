//! Installed-app self-test: real native commands and bundled runtime only.
use anyhow::{Result, ensure};
use smarti_search_core::{Engine, types::*};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::{Listener, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub fn upgrade_probe(app: tauri::AppHandle, engine: Arc<Engine>, report: PathBuf) {
    std::thread::spawn(move || {
        let result =
            wait_for_frontend(&app).and_then(|()| wait(&engine, |s| s.inference["ready"] == true));
        let payload = serde_json::json!({"passed":result.is_ok(),"frontend_ready":result.is_ok(),"error":result.err().map(|error|format!("{error:#}")),"version":env!("CARGO_PKG_VERSION"),"executable":std::env::current_exe().ok(),"data":engine.data});
        let _ = std::fs::write(report, serde_json::to_vec_pretty(&payload).unwrap());
        // Stay alive until the installer closes the application and its job.
    });
}

fn wait_for_frontend(app: &tauri::AppHandle) -> Result<()> {
    let (send, receive) = std::sync::mpsc::channel();
    let listener = app.listen("native-ui-probe", move |event| {
        let _ = send.send(event.payload().to_owned());
    });
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| anyhow::anyhow!("Native main webview missing"))?;
    window.eval(
        r#"(() => {
            const timer = setInterval(() => {
                const error = document.querySelector('.global-errors [role="alert"]');
                if (error || document.querySelector('#main-search')) {
                    clearInterval(timer);
                    window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {
                        event: 'native-ui-probe',
                        payload: {ready: !error, error: error?.textContent || null}
                    });
                }
            }, 100);
        })()"#,
    )?;
    let result = receive.recv_timeout(Duration::from_secs(240));
    app.unlisten(listener);
    let payload: serde_json::Value = serde_json::from_str(&result?)?;
    ensure!(
        payload["ready"] == true,
        "Native frontend bootstrap failed: {}",
        payload["error"]
    );
    Ok(())
}

pub fn start(app: tauri::AppHandle, engine: Arc<Engine>, report: PathBuf, corpus: PathBuf) {
    std::thread::spawn(move || {
        let started = Instant::now();
        let result = run(&app, &engine, &corpus);
        let success = result.is_ok();
        let payload = match result {
            Ok(data) => {
                serde_json::json!({"passed":true,"elapsed_seconds":started.elapsed().as_secs_f64(),"evidence":data})
            }
            Err(error) => serde_json::json!({"passed":false,"error":format!("{error:#}")}),
        };
        if let Some(parent) = report.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&report, serde_json::to_vec_pretty(&payload).unwrap());
        app.exit(if success { 0 } else { 1 });
    });
}
fn wait(engine: &Engine, predicate: impl Fn(&IndexStatus) -> bool) -> Result<()> {
    let start = Instant::now();
    loop {
        let status = engine.status()?;
        if predicate(&status) {
            return Ok(());
        }
        ensure!(
            start.elapsed() < Duration::from_secs(240),
            "Native runtime test timed out: {status:?}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}
fn run(
    app: &tauri::AppHandle,
    engine: &Arc<Engine>,
    corpus: &std::path::Path,
) -> Result<serde_json::Value> {
    ensure!(corpus.is_dir(), "Smoke corpus directory is required");
    wait_for_frontend(app)?;
    ensure!(
        engine
            .resources
            .join("inference/smarti-local-search-inference.exe")
            .is_file(),
        "Packaged inference host missing"
    );
    wait(engine, |s| s.inference["ready"] == true)?;
    let mut settings = engine.settings()?;
    settings.onboarded = true;
    settings.notifications = false;
    settings.minimize_to_tray = false;
    engine.update_settings(settings, false)?;
    let existing = engine
        .roots()?
        .iter()
        .any(|root| std::fs::canonicalize(&root.path).ok() == std::fs::canonicalize(corpus).ok());
    if !existing {
        engine.add_root(corpus)?;
    }
    engine.start_index(None)?;
    wait(engine, |s| s.files >= 11 && s.pending == 0 && !s.running)?;
    let status = engine.status()?;
    ensure!(
        status.errors == 0 && status.semantic_files >= 10,
        "Bundled corpus indexing failed: {status:?}"
    );
    let query = |id, text: &str, media| SearchRequest {
        id,
        query: text.into(),
        mode: "smart".into(),
        filters: Filters::default(),
        semantic_pass: true,
        media,
    };
    let exact = engine.search(SearchRequest {
        mode: "exact".into(),
        semantic_pass: false,
        ..query(1, "auth.rs", None)
    })?;
    ensure!(
        exact
            .results
            .first()
            .is_some_and(|r| r.file.name == "auth.rs"),
        "Native filename search failed"
    );
    let semantic = engine.search(query(2, "a child playing by the sea", None))?;
    ensure!(
        semantic
            .results
            .iter()
            .take(4)
            .any(|r| ["beach.png", "ocean.txt"].contains(&r.file.name.as_str())),
        "Native semantic search failed"
    );
    let image = engine.search(query(
        3,
        "",
        Some(corpus.join("beach.png").to_string_lossy().into_owned()),
    ))?;
    ensure!(
        image
            .results
            .iter()
            .take(3)
            .any(|r| r.file.name == "beach.png"),
        "Native image search failed"
    );
    for name in ["tone.wav", "beach.mp4"] {
        let media = engine.search(query(
            4,
            "",
            Some(corpus.join(name).to_string_lossy().into_owned()),
        ))?;
        ensure!(
            media
                .results
                .iter()
                .take(3)
                .any(|result| result.file.name == name
                    && result
                        .matches
                        .iter()
                        .any(|chunk| chunk.start.is_some() && chunk.end.is_some())),
            "Native media similarity/timestamps failed for {name}"
        );
    }
    let pdf = engine.search(query(5, "search.pdf", None))?;
    let id = pdf
        .results
        .iter()
        .find(|r| r.file.name == "search.pdf")
        .ok_or_else(|| anyhow::anyhow!("PDF missing"))?
        .file
        .id;
    let page = crate::dispatch(
        app,
        engine,
        "pdf_page",
        serde_json::json!({"id":id,"page":1}),
    )?;
    ensure!(
        page["asset"]
            .as_str()
            .is_some_and(|path| std::path::Path::new(path).is_file()),
        "Native PDF preview failed"
    );
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| anyhow::anyhow!("Main native window missing"))?;
    ensure!(window.is_visible()?, "Main native window is not visible");
    let size = window.inner_size()?;
    let shortcut = engine.settings()?.shortcut;
    ensure!(
        app.global_shortcut().is_registered(shortcut.as_str()),
        "Global shortcut registration failed"
    );
    ensure!(
        app.try_state::<crate::TrayLabels>().is_some(),
        "Native tray menu missing"
    );
    crate::show_quick(app);
    let quick = app
        .get_webview_window("quick")
        .ok_or_else(|| anyhow::anyhow!("Quick Search window missing"))?;
    ensure!(quick.is_visible()?, "Quick Search window did not open");
    quick.hide()?;
    engine.control("stop")?;
    Ok(
        serde_json::json!({"status":status,"health":engine.health()?,"native_window":{"visible":true,"width":size.width,"height":size.height},"shortcut_registered":true,"quick_search_window_opened":true,"tray_menu_created":true,"filename_results":exact.results.len(),"semantic_results":semantic.results.len(),"image_results":image.results.len(),"audio_video_similarity_and_timestamps":true,"pdf_page":page["page"],"resources":engine.resources,"data":engine.data,"app_executable":std::env::current_exe()?}),
    )
}
