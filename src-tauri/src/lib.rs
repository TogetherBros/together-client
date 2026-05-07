use tauri::{Emitter, Manager};

// ── Window helpers ────────────────────────────────────────────────────────────

fn restore_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_always_on_top(false);
        let _ = w.set_ignore_cursor_events(false);
        let _ = w.set_size(tauri::Size::Logical(tauri::LogicalSize { width: 460.0, height: 660.0 }));
        let _ = w.center();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn enter_overlay(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") { let _ = w.hide(); }
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.set_ignore_cursor_events(true);
        let _ = w.show();
    }
    Ok(())
}

#[tauri::command]
fn leave_overlay(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("overlay") { let _ = w.hide(); }
    restore_window(&app);
    Ok(())
}

#[tauri::command]
fn get_active_app() -> String {
    get_active_app_internal()
}

// ── Active app detection (Windows) ───────────────────────────────────────────

#[cfg(target_os = "windows")]
fn get_active_app_internal() -> String {
    use winapi::shared::minwindef::DWORD;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::winbase::QueryFullProcessImageNameW;
    use winapi::um::winnt::PROCESS_QUERY_LIMITED_INFORMATION;
    use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() { return "접속 중".to_string(); }

        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 512);
        let title = if title_len > 0 {
            String::from_utf16_lossy(&title_buf[..title_len as usize])
        } else { String::new() };

        let mut pid: DWORD = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 { return "접속 중".to_string(); }

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() { return "접속 중".to_string(); }

        let mut exe_buf = [0u16; 512];
        let mut size: DWORD = 512;
        let ok = QueryFullProcessImageNameW(handle, 0, exe_buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);

        if ok == 0 || size == 0 { return "접속 중".to_string(); }

        let path = String::from_utf16_lossy(&exe_buf[..size as usize]);
        let exe = path.split('\\').last().unwrap_or("")
            .to_lowercase()
            .trim_end_matches(".exe")
            .to_string();

        exe_to_app_name(&exe, &title)
    }
}

#[cfg(not(target_os = "windows"))]
fn get_active_app_internal() -> String { "접속 중".to_string() }

fn exe_to_app_name(exe: &str, title: &str) -> String {
    let is_browser = matches!(exe, "chrome"|"msedge"|"firefox"|"whale"|"opera"|"brave"|"vivaldi");
    if is_browser {
        let tl = title.to_lowercase();
        if tl.contains("youtube") { return "유튜브 시청 중".to_string(); }
        if tl.contains("netflix") { return "넷플릭스 시청 중".to_string(); }
        if tl.contains("twitch")  { return "트위치 시청 중".to_string(); }
        if tl.contains("github")  { return "GitHub 보는 중".to_string(); }
        if tl.contains("notion")  { return "Notion 작업 중".to_string(); }
        if tl.contains("figma")   { return "Figma 작업 중".to_string(); }
        let browser = match exe {
            "chrome"=>"Chrome","msedge"=>"Edge","firefox"=>"Firefox",
            "whale"=>"Whale","opera"=>"Opera","brave"=>"Brave",_=>"브라우저",
        };
        return format!("{} 실행 중", browser);
    }
    let name = match exe {
        "idea64"|"idea"=>"IntelliJ IDEA","code"=>"VS Code",
        "pycharm64"|"pycharm"=>"PyCharm","webstorm64"|"webstorm"=>"WebStorm",
        "clion64"|"clion"=>"CLion","devenv"=>"Visual Studio",
        "discord"=>"Discord","spotify"=>"Spotify","vlc"=>"VLC",
        "notepad"=>"메모장","notepad++"=>"Notepad++","explorer"=>"파일 탐색기",
        "slack"=>"Slack","zoom"=>"Zoom","obs64"|"obs32"|"obs"=>"OBS",
        "steam"=>"Steam","javaw"|"minecraft"|"minecraftlauncher"=>"마인크래프트",
        "windowsterminal"|"terminal"=>"터미널","powershell"=>"PowerShell",
        "cmd"=>"명령 프롬프트","kakaotalk"=>"카카오톡",
        _=>return format!("{} 실행 중", capitalize_first(exe)),
    };
    format!("{} 실행 중", name)
}

fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

// ── App setup ─────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;

            // overlay 창 — hidden 상태로 미리 생성, 항상 passthrough 유지
            tauri::WebviewWindowBuilder::new(
                app,
                "overlay",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .maximized(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .visible(false)
            .build()?;

            // main 창 닫기 → 숨기기
            let app_handle2 = app.handle().clone();
            if let Some(window) = app.get_webview_window("main") {
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(w) = app_handle2.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
                });
            }

            let show  = MenuItem::with_id(app, "show",  "열기",     true, None::<&str>)?;
            let leave = MenuItem::with_id(app, "leave", "방 나가기", true, None::<&str>)?;
            let quit  = MenuItem::with_id(app, "quit",  "종료",      true, None::<&str>)?;
            let menu  = Menu::with_items(app, &[&show, &leave, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("overlay") { let _ = w.hide(); }
                        restore_window(app);
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("open-chat", ());
                        }
                    }
                    "leave" => {
                        if let Some(w) = app.get_webview_window("overlay") { let _ = w.hide(); }
                        restore_window(app);
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("leave-overlay", ());
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            enter_overlay,
            leave_overlay,
            get_active_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
