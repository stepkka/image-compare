#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

fn main() {
    if let Err(error) = run() {
        rfd::MessageDialog::new()
            .set_title("ImageCompare")
            .set_description(error.to_string())
            .set_level(rfd::MessageLevel::Error)
            .show();
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    #[cfg(windows)]
    let paths = if args
        .first()
        .is_some_and(|s| s == "--shell-server" || s == "-Embedding")
    {
        imagecompare::shell::receive_selection()?
    } else {
        args.into_iter().map(std::path::PathBuf::from).collect()
    };
    #[cfg(not(windows))]
    let paths = args.into_iter().map(std::path::PathBuf::from).collect();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("ImageCompare")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ImageCompare",
        options,
        Box::new(move |cc| Ok(Box::new(app::ImageCompare::new(cc, paths)))),
    )?;
    Ok(())
}
