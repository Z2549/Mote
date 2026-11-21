//! Phone remote. Four full-width keys, one status dot, nothing else.

mod app;

pub use app::Phone;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(android_app: winit::platform::android::activity::AndroidApp) {
    let options = eframe::NativeOptions {
        android_app: Some(android_app),
        ..Default::default()
    };
    let _ = eframe::run_native("Mote", options, Box::new(|cc| Ok(Box::new(Phone::new(cc)))));
}
