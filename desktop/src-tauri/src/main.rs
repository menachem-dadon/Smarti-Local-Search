#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod smoke;
mod windows_integration;
use smarti_search_core::{Engine, types::*};
use std::{path::PathBuf, sync::Arc};
use tauri::{
    Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_notification::NotificationExt;

struct TrayLabels {
    quick: MenuItem<tauri::Wry>,
    pause: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}
impl TrayLabels {
    fn apply(&self, language: &str) -> tauri::Result<()> {
        let he = language == "he";
        self.quick.set_text(if he {
            "חיפוש מהיר"
        } else {
            "Quick Search"
        })?;
        self.pause.set_text(if he {
            "השהיה / המשך אינדוקס"
        } else {
            "Pause / Resume indexing"
        })?;
        self.quit.set_text(if he { "יציאה" } else { "Exit" })
    }
}
fn apply_tray_language(app: &tauri::AppHandle, language: &str) -> tauri::Result<()> {
    if let Some(labels) = app.try_state::<TrayLabels>() {
        labels.apply(language)?;
    }
    Ok(())
}

#[tauri::command]
async fn search(
    state: tauri::State<'_, Arc<Engine>>,
    request: SearchRequest,
) -> Result<SearchResponse, String> {
    let e = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || e.search(request).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn command(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Engine>>,
    action: String,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let e = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        dispatch(&app, &e, &action, args).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
fn dispatch(
    app: &tauri::AppHandle,
    e: &Arc<Engine>,
    action: &str,
    args: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let id = || {
        args["id"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("Missing item ID"))
    };
    let value = match action {
        "initial_input" => input_from_args(std::env::args().collect()),
        "get_settings" => serde_json::to_value(e.settings()?)?,
        "reset_settings" => {
            let previous = e.settings()?;
            let defaults = Settings {
                onboarded: previous.onboarded,
                dimensions: previous.dimensions,
                vision_tokens: previous.vision_tokens,
                audio_seconds: previous.audio_seconds,
                video_seconds: previous.video_seconds,
                video_fps: previous.video_fps,
                index_path: previous.index_path.clone(),
                ..Settings::default()
            };
            dispatch(
                app,
                e,
                "update_settings",
                serde_json::json!({"settings":defaults}),
            )?;
            serde_json::to_value(defaults)?
        }
        "get_roots" => serde_json::to_value(e.roots()?)?,
        "get_status" => serde_json::to_value(e.status()?)?,
        "get_activity" => serde_json::to_value(e.store.lock().unwrap().activity()?)?,
        "get_preview" => {
            let p = e.preview(id()?)?;
            if let Some(ref file) = p.asset {
                app.asset_protocol_scope().allow_file(file)?;
            }
            serde_json::to_value(p)?
        }
        "pdf_page" => {
            let file = e.store.lock().unwrap().file(id()?)?;
            anyhow::ensure!(file.kind == "pdf", "Select an indexed PDF");
            let result = e
                .inference
                .call(
                    "render_pdf_page",
                    serde_json::json!({"path":file.path,"page":args["page"].as_u64().unwrap_or(1)}),
                    3,
                )?
                .data;
            if let Some(asset) = result["asset"].as_str() {
                app.asset_protocol_scope().allow_file(asset)?;
            }
            result
        }
        "add_root" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Folder path required"))?;
            serde_json::json!(e.add_root(&PathBuf::from(path))?)
        }
        "remove_root" => {
            e.remove_root(id()?)?;
            serde_json::Value::Null
        }

        "start_index" => {
            e.start_index(args["root"].as_i64())?;
            serde_json::Value::Null
        }
        "index_control" => {
            e.control(args["control"].as_str().unwrap_or("pause"))?;
            serde_json::Value::Null
        }
        "rebuild_index" => {
            e.rebuild()?;
            serde_json::Value::Null
        }
        "clear_index" => {
            e.clear()?;
            serde_json::Value::Null
        }
        "retry" => {
            e.retry(id()?)?;
            serde_json::Value::Null
        }
        "compact_index" => {
            e.compact()?;
            serde_json::Value::Null
        }
        "storage_info" => serde_json::json!({"path":e.data,"database_bytes":e.status()?.bytes}),
        "move_index" => {
            let path = PathBuf::from(
                args["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Destination required"))?,
            );
            e.control("stop")?;
            let destination = e.move_index(&path)?;
            let home = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default())
                .join("Smarti Local Search");
            std::fs::create_dir_all(&home)?;
            std::fs::write(
                home.join("location.json"),
                serde_json::to_vec(&serde_json::json!({"path":destination}))?,
            )?;
            app.restart();
        }
        "index_health" => e.health()?,
        "third_party_notices" => serde_json::json!(std::fs::read_to_string(
            e.resources.join("licenses/THIRD_PARTY_NOTICES.md")
        )?),
        "hardware" => {
            let mut h = Engine::hardware();
            h["power"] = windows_integration::power();
            h
        }
        "benchmark" => {
            let report = e
                .inference
                .call("benchmark", serde_json::json!({}), 2)?
                .data;
            let key = format!(
                "{}:{}:{}",
                e.settings()?.fingerprint(),
                Engine::hardware()["fingerprint"],
                env!("CARGO_PKG_VERSION")
            );
            e.store.lock().unwrap().db.execute(
                "INSERT OR REPLACE INTO benchmark(fingerprint,result,measured) VALUES(?1,?2,?3)",
                rusqlite::params![
                    key,
                    serde_json::to_string(&report)?,
                    smarti_search_core::store::now()
                ],
            )?;
            report
        }
        "update_settings" => {
            let s: Settings = serde_json::from_value(args["settings"].clone())?;
            let old = e.settings()?;
            s.validate()?;
            let integration_changed = s.autostart != old.autostart
                || s.explorer_menu != old.explorer_menu
                || s.language != old.language;
            let result = (|| -> anyhow::Result<()> {
                if s.shortcut != old.shortcut || s.quick_search != old.quick_search {
                    set_shortcut(app, &s)?;
                }
                if integration_changed {
                    windows_integration::configure(s.autostart, s.explorer_menu, &s.language)?;
                }
                apply_tray_language(app, &s.language)?;
                e.update_settings(s.clone(), args["rebuild"].as_bool().unwrap_or(false))?;
                Ok(())
            })();
            if let Err(err) = result {
                let _ = set_shortcut(app, &old);
                let _ = apply_tray_language(app, &old.language);
                if integration_changed {
                    let _ = windows_integration::configure(
                        old.autostart,
                        old.explorer_menu,
                        &old.language,
                    );
                }
                return Err(err);
            }
            serde_json::to_value(s)?
        }
        "open_path" | "reveal_path" | "copy_path" => {
            let f = e.store.lock().unwrap().file(id()?)?;
            let path = PathBuf::from(f.path.clone());
            match action {
                "open_path" => windows_integration::open(&path)?,
                "reveal_path" => windows_integration::reveal(&path)?,
                _ => {
                    use tauri_plugin_clipboard_manager::ClipboardExt;
                    app.clipboard().write_text(f.path)?
                }
            }
            serde_json::Value::Null
        }
        "cancel_search" => {
            e.latest_query.fetch_max(
                args["id"].as_u64().unwrap_or(0) + 1,
                std::sync::atomic::Ordering::Relaxed,
            );
            serde_json::Value::Null
        }
        "hide_quick" => {
            if let Some(w) = app.get_webview_window("quick") {
                w.hide()?;
            }
            serde_json::Value::Null
        }
        "show_main" => {
            show_main(app);
            app.emit_to("main", "search-input", args)?;
            serde_json::Value::Null
        }
        "initial_locations" => {
            let user = std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .unwrap_or_default();
            let entire = args["entire"].as_bool().unwrap_or(false);
            let paths = if entire {
                ('C'..='Z')
                    .map(|d| PathBuf::from(format!("{d}:\\")))
                    .filter(|p| p.is_dir())
                    .collect::<Vec<_>>()
            } else {
                [
                    "Desktop",
                    "Documents",
                    "Downloads",
                    "Pictures",
                    "Videos",
                    "Music",
                    "source",
                    "Projects",
                ]
                .iter()
                .map(|d| user.join(d))
                .filter(|p| p.is_dir())
                .collect()
            };
            serde_json::to_value(
                paths
                    .into_iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect::<Vec<_>>(),
            )?
        }
        "clipboard_image" => {
            use tauri_plugin_clipboard_manager::ClipboardExt;
            let image = app.clipboard().read_image()?;
            let file = e.data.join("cache/query-image.png");
            let png =
                image::RgbaImage::from_raw(image.width(), image.height(), image.rgba().to_vec())
                    .ok_or_else(|| anyhow::anyhow!("Invalid clipboard image"))?;
            png.save(&file)?;
            serde_json::json!(file.to_string_lossy())
        }
        "export_diagnostics" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Export path required"))?;
            let report = serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"hardware":Engine::hardware(),"health":e.health()?,"activity":e.store.lock().unwrap().activity()?,"model":serde_json::from_slice::<serde_json::Value>(&std::fs::read(e.resources.join("models/model-manifest.json"))?)?});
            std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
            serde_json::Value::Null
        }
        _ => anyhow::bail!("Unknown native command: {action}"),
    };
    Ok(value)
}
fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
fn show_quick(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("quick") {
        if let Ok(Some(m)) = w.current_monitor() {
            let scale = m.scale_factor();
            let area = m.work_area();
            let (width, height, x, y) = palette_bounds(
                area.position.x as f64 / scale,
                area.position.y as f64 / scale,
                area.size.width as f64 / scale,
                area.size.height as f64 / scale,
            );
            let _ = w.set_size(tauri::LogicalSize::new(width, height));
            let _ = w.set_position(tauri::LogicalPosition::new(x, y));
        }
        let _ = app.emit_to("quick", "quick-open", ());
        let _ = w.show();
        let _ = w.set_focus();
    }
}
fn palette_bounds(
    left: f64,
    top: f64,
    available_width: f64,
    available_height: f64,
) -> (f64, f64, f64, f64) {
    let width = 780_f64.min((available_width - 32.).max(480.));
    let height = 520_f64.min((available_height - 32.).max(280.));
    (
        width,
        height,
        left + (available_width - width) / 2.,
        top + ((available_height - height) / 2.).clamp(0., 80.),
    )
}
fn fit_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main")
        && let Some(monitor) = window.current_monitor()?
    {
        let area = monitor.work_area();
        let outer = window.outer_size()?;
        if outer.width > area.size.width || outer.height > area.size.height {
            let scale = monitor.scale_factor();
            window.set_size(tauri::LogicalSize::new(
                1200_f64.min(area.size.width as f64 / scale - 32.),
                780_f64.min(area.size.height as f64 / scale - 64.),
            ))?;
            let resized = window.outer_size()?;
            window.set_position(tauri::PhysicalPosition::new(
                area.position.x + area.size.width.saturating_sub(resized.width) as i32 / 2,
                area.position.y + area.size.height.saturating_sub(resized.height) as i32 / 2,
            ))?;
        }
    }
    Ok(())
}
fn set_shortcut(app: &tauri::AppHandle, s: &Settings) -> anyhow::Result<()> {
    app.global_shortcut().unregister_all()?;
    if s.quick_search {
        app.global_shortcut().register(s.shortcut.as_str())?;
    }
    Ok(())
}
fn external_args(app: &tauri::AppHandle, args: Vec<String>) {
    show_main(app);
    let value = input_from_args(args);
    if !value.is_null() {
        let _ = app.emit_to("main", "search-input", value);
    }
}
fn input_from_args(args: Vec<String>) -> serde_json::Value {
    for (i, arg) in args.iter().enumerate() {
        if ["--folder", "--similar"].contains(&arg.as_str())
            && let Some(path) = args.get(i + 1)
            && let Ok(path) = std::fs::canonicalize(path)
        {
            let value = smarti_search_core::engine::display_path(&path);
            return serde_json::json!({"query":if arg=="--folder"{format!("path:\"{value}\"")}else{String::new()},"media":if arg=="--similar"{Some(value)}else{None}});
        }
    }
    serde_json::Value::Null
}
fn main() {
    let isolated_smoke = std::env::args()
        .any(|arg| arg.starts_with("--smoke-test=") || arg.starts_with("--upgrade-probe="));
    assert!(
        !isolated_smoke || std::env::var_os("SMARTI_SEARCH_DATA_DIR").is_some(),
        "Native validation requires a private SMARTI_SEARCH_DATA_DIR"
    );
    let mut builder = tauri::Builder::default();
    if !isolated_smoke {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            external_args(app, args)
        }));
        builder = builder.plugin(
            tauri_plugin_window_state::Builder::default()
                .with_denylist(&["quick"])
                .build(),
        );
    }
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        show_quick(app);
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let handle = app.handle().clone();
            let default_base = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default())
                .join("Smarti Local Search");
            let relocated = std::fs::read(default_base.join("location.json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .and_then(|v| v["path"].as_str().map(PathBuf::from));
            let base = std::env::var_os("SMARTI_SEARCH_DATA_DIR")
                .map(PathBuf::from)
                .or(relocated.clone())
                .unwrap_or_else(|| default_base.clone());
            if isolated_smoke && (base == default_base || relocated.as_ref() == Some(&base)) {
                return Err(anyhow::anyhow!("Native validation must not use the personal index location").into());
            }
            let resources = if cfg!(debug_assertions) {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources")
            } else {
                app.path().resource_dir()?.join("resources")
            };
            let emit = handle.clone();
            let engine = Engine::open(
                base,
                resources,
                Arc::new(move |name, data| {
                    let _ = emit.emit(name, data);
                    if name=="index-complete"
                        && let Some(engine)=emit.try_state::<Arc<Engine>>()
                        && let Ok(settings)=engine.settings()
                        && settings.notifications && settings.onboarded {
                                    let first=engine.store.lock().unwrap().db.query_row("SELECT value FROM settings WHERE key='initial_notification'",[],|r|r.get::<_,String>(0)).is_err();
                                    if first {let _=emit.notification().builder().title("Smarti Local Search").body(if settings.language=="he"{"האינדוקס הראשוני הושלם. הקבצים שלך מוכנים לחיפוש."}else{"Initial indexing finished. Your files are ready to search."}).show();let _=engine.store.lock().unwrap().db.execute("INSERT OR REPLACE INTO settings VALUES('initial_notification','sent')",[]);}
                    }
                }),
            )?;
            let mut settings = engine.settings()?;
            if isolated_smoke && !settings.onboarded {
                settings.shortcut = "Ctrl+Alt+Shift+F11".into();
                engine.store.lock().unwrap().save_settings(&settings)?;
            }
            if !isolated_smoke {
                // The previous uninstaller may remove Explorer/autostart keys;
                // restore them from the preserved settings at the new location.
                if let Err(error) = windows_integration::configure(settings.autostart, settings.explorer_menu, &settings.language) {
                    engine.store.lock().unwrap().log("error", None, "", &format!("Windows integration unavailable: {error}"))?;
                }
            }
            fit_main_window(&handle)?;
            if let Err(error) = set_shortcut(&handle, &settings) {
                engine.store.lock().unwrap().log(
                    "error",
                    None,
                    "",
                    &format!("Global shortcut unavailable: {error}"),
                )?;
            }
            app.manage(engine.clone());
            let arguments:Vec<String>=std::env::args().collect();
            if let Some(report)=arguments.iter().find_map(|value|value.strip_prefix("--smoke-test=")) {
                let corpus=arguments.iter().find_map(|value|value.strip_prefix("--smoke-root=")).ok_or_else(||anyhow::anyhow!("--smoke-root is required"))?;
                smoke::start(handle.clone(),engine.clone(),PathBuf::from(report),PathBuf::from(corpus));
            }
            if let Some(report)=arguments.iter().find_map(|value|value.strip_prefix("--upgrade-probe=")) {
                smoke::upgrade_probe(engine.clone(),PathBuf::from(report));
            }
            let quick = MenuItem::with_id(app, "quick", "Quick Search", true, None::<&str>)?;
            let main = MenuItem::with_id(app, "main", "Smarti Local Search", true, None::<&str>)?;
            let pause =
                MenuItem::with_id(app, "pause", "Pause / Resume indexing", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quick, &main, &pause, &quit])?;
            let labels = TrayLabels { quick, pause, quit };
            labels.apply(&settings.language)?;
            app.manage(labels);
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Smarti Local Search")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quick" => show_quick(app),
                    "main" => show_main(app),
                    "pause" => {
                        let e = app.state::<Arc<Engine>>();
                        let _ = e.control(if e.paused.load(std::sync::atomic::Ordering::Relaxed) {
                            "resume"
                        } else {
                            "pause"
                        });
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            if std::env::args().any(|a| a == "--tray")
                && let Some(w) = app.get_webview_window("main")
            {
                w.hide()?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "quick" {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    let e = window.app_handle().state::<Arc<Engine>>();
                    if e.settings().is_ok_and(|s| s.minimize_to_tray) {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        window.app_handle().exit(0);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![search, command])
        .run(tauri::generate_context!())
        .expect("Smarti Local Search could not start");
}

#[cfg(test)]
mod window_tests {
    #[test]
    fn palette_fits_high_dpi_work_areas_and_negative_monitor_coordinates() {
        for scale in [1., 1.25, 1.5, 2.] {
            for (pixels_width, pixels_height) in [(1920., 1040.), (1366., 728.)] {
                let available_width = pixels_width / scale;
                let available_height = pixels_height / scale;
                let left = -available_width;
                let top = 24.;
                let (width, height, x, y) =
                    super::palette_bounds(left, top, available_width, available_height);
                assert!(width >= 480. && height >= 280.);
                assert!(x >= left && y >= top);
                assert!(x + width <= left + available_width);
                assert!(y + height <= top + available_height);
            }
        }
    }
}
