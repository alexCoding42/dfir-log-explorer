use dfir_log_core::{
    CaseSummary, Engine, EngineError, EventDetail, ImportProgress, ParserMode, Query, QueryPage,
    SavedSearch, SourceSummary,
};
use std::{
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{ipc::Channel, Manager, State};
use tauri_plugin_dialog::DialogExt;

struct AppState {
    engine: Arc<Mutex<Option<Engine>>>,
    selected: Arc<Mutex<Option<PathBuf>>>,
    cancelled: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
}

struct BusyGuard(Arc<AtomicBool>);
impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn lock_error() -> String {
    "The local session is unavailable. Restart the application.".into()
}

#[tauri::command]
async fn pick_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    if state.busy.load(Ordering::Acquire) {
        return Err("Wait for the current import to finish or cancel it.".into());
    }
    let selected = Arc::clone(&state.selected);
    tauri::async_runtime::spawn_blocking(move || {
        let file = app
            .dialog()
            .file()
            .add_filter(
                "Log and text files",
                &["log", "txt", "access", "out", "jsonl", "ndjson", "json"],
            )
            .add_filter("All files", &["*"])
            .blocking_pick_file();
        let Some(file) = file else { return Ok(None) };
        let path = file
            .into_path()
            .map_err(|_| "Select a local file.".to_string())?;
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        *selected.lock().map_err(|_| lock_error())? = Some(path);
        Ok(Some(name))
    })
    .await
    .map_err(|_| lock_error())?
}

#[tauri::command]
async fn import_file(
    mode: ParserMode,
    on_progress: Channel<ImportProgress>,
    state: State<'_, AppState>,
) -> Result<SourceSummary, String> {
    let path = state
        .selected
        .lock()
        .map_err(|_| lock_error())?
        .clone()
        .ok_or_else(|| "Choose a source file first.".to_string())?;
    start_import(state, Some(path), mode, on_progress).await
}

#[tauri::command]
async fn import_demo(
    on_progress: Channel<ImportProgress>,
    state: State<'_, AppState>,
) -> Result<SourceSummary, String> {
    start_import(state, None, ParserMode::Auto, on_progress).await
}

async fn start_import(
    state: State<'_, AppState>,
    path: Option<PathBuf>,
    mode: ParserMode,
    on_progress: Channel<ImportProgress>,
) -> Result<SourceSummary, String> {
    state
        .busy
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "An import is already running.".to_string())?;
    state.cancelled.store(false, Ordering::Relaxed);
    let engine = Arc::clone(&state.engine);
    let cancelled = Arc::clone(&state.cancelled);
    let guard = BusyGuard(Arc::clone(&state.busy));
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let is_demo = path.is_none();
        let mut sample = None;
        let source_path = match path {
            Some(path) => path,
            None => {
                let mut file = tempfile::Builder::new()
                    .prefix("synthetic-apache-")
                    .suffix(".log")
                    .tempfile()
                    .map_err(|_| "Unable to create the local sample.".to_string())?;
                file.write_all(include_bytes!("../../tests/fixtures/apache-mixed.log"))
                    .map_err(|_| "Unable to write the local sample.".to_string())?;
                let path = file.path().to_path_buf();
                sample = Some(file);
                path
            }
        };
        let mut locked = engine.lock().map_err(|_| lock_error())?;
        let engine = locked.as_mut().ok_or_else(lock_error)?;
        let identity = if is_demo {
            Some((
                "apache-mixed.log (synthetic sample)",
                "Bundled synthetic fixture",
            ))
        } else {
            None
        };
        let summary = engine
            .import_with_identity(
                &source_path,
                mode,
                &cancelled,
                |progress| {
                    if on_progress.send(progress).is_err() {
                        cancelled.store(true, Ordering::Relaxed);
                    }
                },
                identity,
            )
            .map_err(|error| error.to_string())?;
        drop(sample);
        Ok(summary)
    })
    .await
    .map_err(|_| lock_error())?
}

#[tauri::command]
fn cancel_import(state: State<'_, AppState>) {
    state.cancelled.store(true, Ordering::Relaxed);
}

#[tauri::command]
async fn query_events(query: Query, state: State<'_, AppState>) -> Result<QueryPage, String> {
    if state.busy.load(Ordering::Acquire) {
        return Err("Wait for the import to finish.".into());
    }
    let engine = Arc::clone(&state.engine);
    tauri::async_runtime::spawn_blocking(move || {
        engine
            .lock()
            .map_err(|_| lock_error())?
            .as_ref()
            .ok_or_else(lock_error)?
            .query(&query)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| lock_error())?
}

#[tauri::command]
async fn event_detail(id: u64, state: State<'_, AppState>) -> Result<EventDetail, String> {
    if state.busy.load(Ordering::Acquire) {
        return Err("Wait for the import to finish.".into());
    }
    let engine = Arc::clone(&state.engine);
    tauri::async_runtime::spawn_blocking(move || {
        engine
            .lock()
            .map_err(|_| lock_error())?
            .as_ref()
            .ok_or_else(lock_error)?
            .detail(id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| lock_error())?
}

#[tauri::command]
async fn case_summary(state: State<'_, AppState>) -> Result<CaseSummary, String> {
    if state.busy.load(Ordering::Acquire) {
        return Err("Wait for the current operation to finish.".into());
    }
    let engine = Arc::clone(&state.engine);
    tauri::async_runtime::spawn_blocking(move || {
        engine
            .lock()
            .map_err(|_| lock_error())?
            .as_ref()
            .ok_or_else(lock_error)?
            .case_summary()
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| lock_error())?
}

#[tauri::command]
async fn save_case(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<CaseSummary>, String> {
    change_case(app, state, true).await
}

#[tauri::command]
async fn open_case(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<CaseSummary>, String> {
    change_case(app, state, false).await
}

async fn change_case(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    save: bool,
) -> Result<Option<CaseSummary>, String> {
    state
        .busy
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "Wait for the current operation to finish.".to_string())?;
    let guard = BusyGuard(Arc::clone(&state.busy));
    let engine = Arc::clone(&state.engine);
    let selected = Arc::clone(&state.selected);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let picker = app.dialog().file().add_filter("DFIR case", &["dfir"]);
        let file = if save {
            picker
                .set_file_name("investigation.dfir")
                .blocking_save_file()
        } else {
            picker.blocking_pick_file()
        };
        let Some(file) = file else {
            return Ok(None);
        };
        let mut path = file
            .into_path()
            .map_err(|_| "Select a local case file.".to_string())?;
        if save && path.extension().is_none() {
            path.set_extension("dfir");
        }
        let mut locked = engine.lock().map_err(|_| lock_error())?;
        let summary = if save {
            locked
                .as_mut()
                .ok_or_else(lock_error)?
                .save_case(&path)
                .map_err(|error| error.to_string())?
        } else {
            let opened = Engine::open_case(&path).map_err(|error| error.to_string())?;
            let summary = opened.case_summary().map_err(|error| error.to_string())?;
            *locked = Some(opened);
            summary
        };
        *selected.lock().map_err(|_| lock_error())? = None;
        Ok(Some(summary))
    })
    .await
    .map_err(|_| lock_error())?
}

#[tauri::command]
async fn close_case(state: State<'_, AppState>) -> Result<CaseSummary, String> {
    state
        .busy
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "Wait for the current operation to finish.".to_string())?;
    let guard = BusyGuard(Arc::clone(&state.busy));
    let engine = Arc::clone(&state.engine);
    let selected = Arc::clone(&state.selected);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let new = Engine::new().map_err(|error| error.to_string())?;
        let summary = new.case_summary().map_err(|error| error.to_string())?;
        *engine.lock().map_err(|_| lock_error())? = Some(new);
        *selected.lock().map_err(|_| lock_error())? = None;
        Ok(summary)
    })
    .await
    .map_err(|_| lock_error())?
}

async fn with_search_engine<T: Send + 'static>(
    state: State<'_, AppState>,
    exclusive: bool,
    operation: impl FnOnce(&Engine) -> Result<T, EngineError> + Send + 'static,
) -> Result<T, String> {
    let guard = if exclusive {
        state
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "Wait for the current operation to finish.".to_string())?;
        Some(BusyGuard(Arc::clone(&state.busy)))
    } else {
        if state.busy.load(Ordering::Acquire) {
            return Err("Wait for the current operation to finish.".into());
        }
        None
    };
    let engine = Arc::clone(&state.engine);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let locked = engine.lock().map_err(|_| lock_error())?;
        operation(locked.as_ref().ok_or_else(lock_error)?).map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| lock_error())?
}

#[tauri::command]
async fn saved_searches(state: State<'_, AppState>) -> Result<Vec<SavedSearch>, String> {
    with_search_engine(state, false, |engine| engine.saved_searches()).await
}
#[tauri::command]
async fn put_saved_search(
    id: Option<u64>,
    name: String,
    query: Query,
    state: State<'_, AppState>,
) -> Result<SavedSearch, String> {
    with_search_engine(state, true, move |engine| {
        engine.put_saved_search(id, &name, query)
    })
    .await
}
#[tauri::command]
async fn delete_saved_search(id: u64, state: State<'_, AppState>) -> Result<(), String> {
    with_search_engine(state, true, move |engine| engine.delete_saved_search(id)).await
}
#[tauri::command]
async fn saved_search_query(id: u64, state: State<'_, AppState>) -> Result<Query, String> {
    with_search_engine(state, false, move |engine| engine.saved_search_query(id)).await
}

pub fn run() {
    let engine = match Engine::new() {
        Ok(engine) => engine,
        Err(_) => {
            eprintln!("Unable to initialize local session storage. Check available disk space and temporary-directory access.");
            std::process::exit(1);
        }
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            engine: Arc::new(Mutex::new(Some(engine))),
            selected: Arc::new(Mutex::new(None)),
            cancelled: Arc::new(AtomicBool::new(false)),
            busy: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            pick_file,
            import_file,
            import_demo,
            cancel_import,
            query_events,
            event_detail,
            case_summary,
            save_case,
            open_case,
            close_case,
            saved_searches,
            put_saved_search,
            delete_saved_search,
            saved_search_query
        ])
        .build(tauri::generate_context!())
        .expect("Unable to start the desktop application")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                let state = app.state::<AppState>();
                state.cancelled.store(true, Ordering::Relaxed);
                // Tauri exits the process without normal stack unwinding. Take
                // and drop the engine here so its private temporary directory
                // is removed on normal exit, including after a cancelled import.
                if let Ok(mut session) = state.engine.lock() {
                    session.take();
                };
            }
        });
}
