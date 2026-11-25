#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod demo;
mod library;
mod state;
mod theme;
mod widgets;

use std::sync::Arc;

fn main() -> eframe::Result {
    
    
    if std::env::args().any(|arg| arg == "--make-demo") {
        let dir = state::default_music_dir();
        let _ = std::fs::create_dir_all(&dir);
        return match demo::generate(&dir) {
            Ok(_) => Ok(()),
            Err(message) => {
                eprintln!("Não foi possível gerar a faixa de demonstração: {message}");
                std::process::exit(1);
            }
        };
    }

    let config_dir = state::config_dir();
    std::panic::set_hook(Box::new(move |info| {
        let _ = std::fs::create_dir_all(&config_dir);
        let _ = std::fs::write(config_dir.join("crash.log"), format!("{info}\n"));
    }));

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(initial_window_size())
            .with_min_inner_size([940.0, 600.0])
            .with_title("ResoRust")
            .with_icon(Arc::new(theme::app_icon())),
        renderer: eframe::Renderer::Glow,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "ResoRust",
        options,
        Box::new(|cc| Ok(Box::new(app::ResoRustApp::new(cc)))),
    )
}


fn initial_window_size() -> [f32; 2] {
    #[cfg(target_os = "windows")]
    {
        let (work_w, work_h) = work_area();
        let width = (work_w - 80.0).clamp(940.0, 1200.0);
        let height = (work_h - 96.0).clamp(600.0, 780.0);
        [width, height]
    }

    #[cfg(not(target_os = "windows"))]
    {
        [1200.0, 780.0]
    }
}

#[cfg(target_os = "windows")]
fn work_area() -> (f32, f32) {
    use std::ffi::c_void;

    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    const SPI_GETWORKAREA: u32 = 0x0030;

    unsafe extern "system" {
        fn SystemParametersInfoW(
            action: u32,
            param: u32,
            data: *mut c_void,
            win_ini: u32,
        ) -> i32;
    }

    let mut rect = Rect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };

    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            &mut rect as *mut Rect as *mut c_void,
            0,
        )
    };

    if ok != 0 && rect.right > rect.left && rect.bottom > rect.top {
        (
            (rect.right - rect.left) as f32,
            (rect.bottom - rect.top) as f32,
        )
    } else {
        (1280.0, 800.0)
    }
}
