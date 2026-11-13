use eframe::egui::{self, Color32, RichText, TextureHandle, Vec2};
use imagecompare::{
    loader::{self, DecodedImage},
    model::{Comparison, ViewMode},
};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

struct Entry {
    path: PathBuf,
    state: ImageState,
}

enum ImageState {
    Loading,
    Ready {
        texture: TextureHandle,
        dimensions: [u32; 2],
        preview: bool,
    },
    Failed(String),
}

struct Loaded {
    generation: u64,
    index: usize,
    image: Result<DecodedImage, String>,
}

pub struct ImageCompare {
    comparison: Comparison,
    entries: Vec<Entry>,
    generation: u64,
    jobs: Sender<(u64, usize, PathBuf)>,
    results: Receiver<Loaded>,
}

impl ImageCompare {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: Vec<PathBuf>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(14.0, 9.0);
        style.visuals.selection.bg_fill = Color32::from_rgb(44, 95, 130);
        cc.egui_ctx.set_style(style);
        let (jobs, requests) = mpsc::channel::<(u64, usize, PathBuf)>();
        // Bound decoded images waiting for upload so opening a large batch stays reasonable.
        let (completed, results) = mpsc::sync_channel(2);
        let ctx = cc.egui_ctx.clone();
        let max_side = ctx.input(|i| i.max_texture_side) as u32;
        std::thread::spawn(move || {
            while let Ok((generation, index, path)) = requests.recv() {
                let image = loader::decode(&path, max_side.min(8192));
                if completed
                    .send(Loaded {
                        generation,
                        index,
                        image,
                    })
                    .is_err()
                {
                    break;
                }
                ctx.request_repaint();
            }
        });
        let mut app = Self {
            comparison: Comparison::default(),
            entries: Vec::new(),
            generation: 0,
            jobs,
            results,
        };
        app.add(paths);
        app
    }

    fn add(&mut self, paths: Vec<PathBuf>) {
        for path in self.comparison.add(paths) {
            let index = self.entries.len();
            self.entries.push(Entry {
                path: path.clone(),
                state: ImageState::Loading,
            });
            let _ = self.jobs.send((self.generation, index, path));
        }
    }

    fn choose(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .set_title("Choose images to compare")
            .add_filter("Images", loader::EXTENSIONS)
            .pick_files()
        {
            self.add(paths);
        }
    }

    fn clear(&mut self) {
        self.generation += 1;
        self.comparison = Comparison::default();
        self.entries.clear();
    }

    fn collect_images(&mut self, ctx: &egui::Context) {
        while let Ok(loaded) = self.results.try_recv() {
            if loaded.generation != self.generation {
                continue;
            }
            if let Some(entry) = self.entries.get_mut(loaded.index) {
                entry.state = match loaded.image {
                    Ok(decoded) => {
                        let preview = decoded.size != decoded.original_size.map(|v| v as usize);
                        let image =
                            egui::ColorImage::from_rgba_unmultiplied(decoded.size, &decoded.pixels);
                        let texture = ctx.load_texture(
                            format!("{}:{}", self.generation, loaded.index),
                            image,
                            egui::TextureOptions::LINEAR,
                        );
                        ImageState::Ready {
                            texture,
                            dimensions: decoded.original_size,
                            preview,
                        }
                    }
                    Err(message) => ImageState::Failed(message),
                };
            }
        }
    }

    fn image_panel(ui: &mut egui::Ui, entry: &Entry, number: usize) {
        let border = Color32::from_gray(55);
        egui::Frame::new()
            .fill(Color32::from_rgb(24, 27, 32))
            .stroke(egui::Stroke::new(1.0_f32, border))
            .inner_margin(12.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                let name = entry.path.file_name().unwrap_or_default().to_string_lossy();
                ui.add(
                    egui::Label::new(
                        RichText::new(format!("{:02}  {}", number + 1, name)).strong(),
                    )
                    .truncate(),
                )
                .on_hover_text(entry.path.display().to_string());
                match &entry.state {
                    ImageState::Ready {
                        dimensions,
                        preview,
                        ..
                    } => {
                        ui.label(
                            RichText::new(format!(
                                "{} × {} px{}",
                                dimensions[0],
                                dimensions[1],
                                if *preview { " · scaled preview" } else { "" }
                            ))
                            .small()
                            .color(Color32::from_gray(150)),
                        );
                    }
                    ImageState::Loading => {
                        ui.label(
                            RichText::new("Loading image…")
                                .small()
                                .color(Color32::from_gray(150)),
                        );
                    }
                    ImageState::Failed(_) => {
                        ui.label(
                            RichText::new("Unable to load")
                                .small()
                                .color(Color32::from_gray(150)),
                        );
                    }
                }
                let space = ui.available_size().max(Vec2::splat(1.0));
                let (rect, _) = ui.allocate_exact_size(space, egui::Sense::hover());
                match &entry.state {
                    ImageState::Ready { texture, .. } => {
                        let size = texture.size_vec2();
                        let scale = (rect.width() / size.x).min(rect.height() / size.y);
                        let image_rect = egui::Rect::from_center_size(rect.center(), size * scale);
                        // Checkerboard makes transparent areas visible.
                        let painter = ui.painter().with_clip_rect(image_rect);
                        let tile = 16.0;
                        for row in 0..(image_rect.height() / tile).ceil() as usize {
                            for col in 0..(image_rect.width() / tile).ceil() as usize {
                                let color =
                                    Color32::from_gray(if (row + col) % 2 == 0 { 40 } else { 49 });
                                painter.rect_filled(
                                    egui::Rect::from_min_size(
                                        image_rect.min
                                            + Vec2::new(col as f32 * tile, row as f32 * tile),
                                        Vec2::splat(tile),
                                    ),
                                    0.0,
                                    color,
                                );
                            }
                        }
                        painter.image(
                            texture.id(),
                            image_rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    ImageState::Loading => {
                        ui.put(
                            egui::Rect::from_center_size(rect.center(), Vec2::splat(28.0)),
                            egui::Spinner::new(),
                        );
                    }
                    ImageState::Failed(error) => {
                        ui.scope_builder(
                            egui::UiBuilder::new().max_rect(rect.shrink(16.0)),
                            |ui| {
                                ui.colored_label(
                                    Color32::from_rgb(245, 150, 140),
                                    "This image could not be opened.",
                                );
                                ui.label(error);
                            },
                        );
                    }
                }
            });
    }
}

impl eframe::App for ImageCompare {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.collect_images(ctx);
        let dropped = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect::<Vec<_>>()
        });
        self.add(dropped);
        if !ctx.wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
                self.comparison.toggle();
            }
            if self.comparison.mode == ViewMode::Single {
                if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                    self.comparison.previous();
                }
                if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                    self.comparison.next();
                }
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
                self.choose();
            }
        }
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.heading("ImageCompare");
                ui.separator();
                if ui.button("Add images…").clicked() {
                    self.choose();
                }
                ui.separator();
                let mut mode = self.comparison.mode;
                ui.selectable_value(&mut mode, ViewMode::SideBySide, "Side by side");
                ui.selectable_value(&mut mode, ViewMode::Single, "One by one");
                self.comparison.set_mode(mode);
                if !self.entries.is_empty() && ui.button("Clear").clicked() {
                    self.clear();
                }
            });
            ui.add_space(8.0);
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "{} image{}",
                    self.entries.len(),
                    if self.entries.len() == 1 { "" } else { "s" }
                ));
                ui.separator();
                ui.weak(
                    "Tab: switch view    •    Left / Right: previous / next    •    Ctrl+O: add images",
                );
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.entries.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.30);
                    ui.heading("See the differences.");
                    ui.add_space(12.0);
                    ui.label("Drop images here, or choose files to start comparing.");
                    ui.add_space(16.0);
                    if ui.button("Choose images…").clicked() {
                        self.choose();
                    }
                    ui.add_space(12.0);
                    ui.weak("PNG · JPEG · WebP · GIF · BMP · TIFF · ICO");
                });
            } else if self.comparison.mode == ViewMode::SideBySide {
                let width = ((ui.available_width() - (self.entries.len() - 1) as f32 * 10.0)
                    / self.entries.len() as f32)
                    .max(230.0);
                let height = (ui.available_height() - 18.0).max(150.0);
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (index, entry) in self.entries.iter().enumerate() {
                                ui.allocate_ui_with_layout(
                                    Vec2::new(width, height),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        Self::image_panel(ui, entry, index);
                                    },
                                );
                            }
                        });
                    });
            } else {
                ui.horizontal(|ui| {
                    if ui.button("Previous").clicked() {
                        self.comparison.previous();
                    }
                    ui.label(format!(
                        "{} / {}",
                        self.comparison.selected + 1,
                        self.entries.len()
                    ));
                    if ui.button("Next").clicked() {
                        self.comparison.next();
                    }
                });
                let index = self.comparison.selected;
                Self::image_panel(ui, &self.entries[index], index);
            }
        });
    }
}
