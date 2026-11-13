//! Development smoke check: render the actual native UI, switch views with keys,
//! and save GPU screenshots under artifacts/. No desktop input is injected.
#[path = "../src/app.rs"]
mod app;
use eframe::egui;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

struct Smoke {
    app: app::ImageCompare,
    phase: usize,
    since: Instant,
    waiting: bool,
    output: PathBuf,
}
impl eframe::App for Smoke {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let shot = ctx.input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(shot) = shot {
            let name = [
                "01-side-by-side.png",
                "02-one-by-one.png",
                "03-next-image.png",
                "04-wraparound.png",
            ][self.phase];
            let bytes: Vec<_> = shot
                .pixels
                .iter()
                .flat_map(|pixel| pixel.to_array())
                .collect();
            image::save_buffer(
                self.output.join(name),
                &bytes,
                shot.width() as u32,
                shot.height() as u32,
                image::ColorType::Rgba8,
            )
            .unwrap();
            println!("Saved {name}");
            self.phase += 1;
            self.waiting = false;
            self.since = Instant::now();
            if self.phase == 4 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                let key = if self.phase == 1 {
                    egui::Key::Tab
                } else {
                    egui::Key::ArrowRight
                };
                ctx.input_mut(|input| {
                    input.events.push(egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::default(),
                    })
                });
            }
        }
        self.app.update(ctx, frame);
        if !self.waiting && self.phase < 4 && self.since.elapsed() > Duration::from_secs(2) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.waiting = true;
        }
        ctx.request_repaint_after(Duration::from_millis(50));
    }
}
fn main() -> eframe::Result {
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("artifacts");
    std::fs::create_dir_all(&output).unwrap();
    let paths: Vec<_> = ["first image.png", "second image.png"]
        .iter()
        .map(|name| output.join(name))
        .collect();
    for (index, path) in paths.iter().enumerate() {
        let mut image = image::RgbaImage::new(800, 500);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let stripe = if (x / 100 + y / 100) % 2 == 0 { 30 } else { 0 };
            *pixel = if index == 0 {
                image::Rgba([45 + stripe, (100 + y / 5) as u8, (100 + x / 8) as u8, 255])
            } else {
                image::Rgba([(100 + x / 8) as u8, 65 + stripe, (100 + y / 5) as u8, 255])
            };
        }
        image.save(path).unwrap();
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_title("ImageCompare render check"),
        ..Default::default()
    };
    eframe::run_native(
        "ImageCompare smoke",
        options,
        Box::new(move |cc| {
            Ok(Box::new(Smoke {
                app: app::ImageCompare::new(cc, paths),
                phase: 0,
                since: Instant::now(),
                waiting: false,
                output,
            }))
        }),
    )
}
