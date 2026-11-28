pub mod app;

use crate::app::Playground;
fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        native_options,
        Box::new(|cc| Ok(Box::new(Playground::new(cc)))),
    );
}
