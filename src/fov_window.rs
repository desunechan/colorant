//! Real-time FOV (Field of View) preview window for debugging capture
use eframe::egui;
use image::RgbImage;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;

/// Real-time FOV preview window
pub struct FovWindow {
    latest_frame: Arc<Mutex<Option<RgbImage>>>,
    window_title: String,
    fps_counter: f32,
    last_update: Instant,
    frame_count: u32,
}

impl FovWindow {
    /// Create a new FOV window
    pub fn new(latest_frame: Arc<Mutex<Option<RgbImage>>>) -> Self {
        Self {
            latest_frame,
            window_title: "🎯 FOV Preview - Colorant Rust".to_string(),
            fps_counter: 0.0,
            last_update: Instant::now(),
            frame_count: 0,
        }
    }

    /// Run the FOV window (blocking)
    pub fn run(self) -> Result<(), eframe::Error> {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 600.0])
                .with_min_inner_size([400.0, 300.0])
                .with_title(self.window_title.clone())
                .with_resizable(true)
                .with_decorations(true),
            ..Default::default()
        };

        eframe::run_native(
            "FOV Preview - Colorant Rust",
            options,
            Box::new(|_cc| Ok(Box::new(self))), // FIXED: Added Ok()
        )
    }

    /// Get current frame from capture (with FPS calculation)
    fn get_current_frame(&mut self) -> Option<RgbImage> {
        let frame = self.latest_frame.lock().clone();
        
        // Update FPS counter
        self.frame_count += 1;
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f32();
        
        if elapsed >= 1.0 {
            self.fps_counter = self.frame_count as f32 / elapsed;
            self.frame_count = 0;
            self.last_update = now;
        }
        
        frame
    }
}

impl eframe::App for FovWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update window title with FPS
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "🎯 FOV Preview - {:.1} FPS",
            self.fps_counter
        )));

        // Request repaint at 60 FPS for smooth updates
        ctx.request_repaint_after(std::time::Duration::from_millis(16));

        // Main UI
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🎮 Real-time FOV Preview");
            
            // Stats row
            ui.horizontal(|ui| {
                ui.label("📊 Status:");
                ui.colored_label(
                    egui::Color32::GREEN,
                    if self.latest_frame.lock().is_some() {
                        "🟢 ACTIVE"
                    } else {
                        "🔴 NO FRAME"
                    },
                );
                ui.label(format!("| 🎯 FPS: {:.1}", self.fps_counter));
                ui.label("| 🖥️ Drag to resize");
            });

            ui.separator();

            // Get current frame
            if let Some(frame) = self.get_current_frame() {
                let width = frame.width() as usize;
                let height = frame.height() as usize;
                
                // Display frame info
                ui.horizontal(|ui| {
                    ui.label("📐 Resolution:");
                    ui.monospace(format!("{}x{}", width, height));
                    ui.label("| 📦 Pixels:");
                    ui.monospace(format!("{}", width * height));
                });

                // Convert RGB image to RGBA for egui
                let rgba: Vec<u8> = frame
                    .pixels()
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect();

                // Create texture
                let texture = ctx.load_texture(
                    "fov_preview",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [width, height],
                        &rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                );

                // Display the image with aspect ratio preservation
                let available_size = ui.available_size();
                let aspect_ratio = width as f32 / height as f32;
                
                let display_size = if available_size.x / available_size.y > aspect_ratio {
                    // Height is limiting factor
                    egui::vec2(available_size.y * aspect_ratio, available_size.y)
                } else {
                    // Width is limiting factor
                    egui::vec2(available_size.x, available_size.x / aspect_ratio)
                };

                // FIXED: Use Image widget with size parameter
                let image = egui::Image::new(&texture)
                    .fit_to_exact_size(display_size);
                
                let response = ui.add(image);
                let image_rect = response.rect;

                // Pixel info on hover
                if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
                    if image_rect.contains(pointer_pos) {
                        // Calculate pixel coordinates
                        let rel_x = (pointer_pos.x - image_rect.left()) / image_rect.width();
                        let rel_y = (pointer_pos.y - image_rect.top()) / image_rect.height();
                        
                        if (0.0..=1.0).contains(&rel_x) && (0.0..=1.0).contains(&rel_y) {
                            let pixel_x = (rel_x * width as f32) as u32;
                            let pixel_y = (rel_y * height as f32) as u32;
                            
                            if pixel_x < width as u32 && pixel_y < height as u32 {
                                let idx = ((pixel_y * width as u32 + pixel_x) * 3) as usize;
                                if idx + 2 < rgba.len() {
                                    let r = rgba[idx];
                                    let g = rgba[idx + 1];
                                    let b = rgba[idx + 2];
                                    
                                    egui::Window::new("Pixel Info")
                                        .fixed_pos(pointer_pos + egui::vec2(10.0, 10.0))
                                        .resizable(false)
                                        .title_bar(false)
                                        .show(ctx, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label("📍");
                                                ui.label(format!("X: {}, Y: {}", pixel_x, pixel_y));
                                            });
                                            ui.horizontal(|ui| {
                                                ui.colored_label(
                                                    egui::Color32::from_rgb(255, 0, 0),
                                                    format!("R: {}", r),
                                                );
                                                ui.colored_label(
                                                    egui::Color32::from_rgb(0, 255, 0),
                                                    format!("G: {}", g),
                                                );
                                                ui.colored_label(
                                                    egui::Color32::from_rgb(0, 0, 255),
                                                    format!("B: {}", b),
                                                );
                                            });
                                            ui.colored_label(
                                                egui::Color32::from_rgb(r, g, b),
                                                "█ Sample",
                                            );
                                        });
                                }
                            }
                        }
                    }
                }
            } else {
                // No frame available
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("⏳ Waiting for capture...");
                        ui.add(egui::Spinner::new().size(50.0));
                        ui.label("Ensure your capture is running and FOV is configured correctly.");
                    });
                });
            }

            // Help section
            ui.separator();
            ui.collapsing("ℹ️ Help", |ui| {
                ui.label("🎮 Controls:");
                ui.indent("controls", |ui| {
                    ui.label("• 🖱️ Hover over image to see pixel values");
                    ui.label("• 🔄 Window auto-updates at 60 FPS");
                    ui.label("• 📐 Drag edges to resize window");
                    ui.label("• 🎯 FPS counter shows capture performance");
                });
                ui.label("⚠️ Debug Tips:");
                ui.indent("tips", |ui| {
                    ui.label("• If no image appears, check capture coordinates");
                    ui.label("• Low FPS may indicate capture performance issues");
                    ui.label("• Black screen may mean capture region is off-screen");
                });
            });
        });
    }
}

/// Launch FOV window - MUST be called from main thread
pub fn launch_fov_window(latest_frame: Arc<Mutex<Option<RgbImage>>>) {
    // Store the handle for the main thread to use
    // In your main(), you would call run_fov_window_blocking
    std::thread::spawn(move || {
        // Just hold the reference, window must be created in main thread
        let _handle = latest_frame;
        println!("[FOV] NOTE: Window must be created from main() function");
    });
}

/// Blocking version for direct integration
pub fn run_fov_window_blocking(latest_frame: Arc<Mutex<Option<RgbImage>>>) -> Result<(), eframe::Error> {
    let window = FovWindow::new(latest_frame);
    window.run()
}