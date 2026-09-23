use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use std::time::{Duration, Instant};

struct FourierApp {
    // ------------------------------------------------------------
    // User inputs
    // ------------------------------------------------------------

    a0_input: String,
    an_input: String,
    bn_input: String,

    num_terms: usize,
    current_n: usize,

    // ------------------------------------------------------------
    // Animation
    // ------------------------------------------------------------

    running: bool,
    animation_speed: f32,
    last_update: Instant,

    // ------------------------------------------------------------
    // Plot
    // ------------------------------------------------------------

    x_min: f64,
    x_max: f64,
    x_points: usize,

    x_values: Vec<f64>,
    y_values: Vec<f64>,

    // ------------------------------------------------------------
    // Compiled coefficients
    // ------------------------------------------------------------

    a0: Option<f64>,

    // These are boxed closures created by meval::Expr::bind().
    an_func: Option<Box<dyn Fn(f64) -> f64>>,
    bn_func: Option<Box<dyn Fn(f64) -> f64>>,

    // ------------------------------------------------------------
    // State
    // ------------------------------------------------------------

    error: Option<String>,
    coefficients_dirty: bool,
}

impl Default for FourierApp {
    fn default() -> Self {
        let x_min = -std::f64::consts::PI;
        let x_max = std::f64::consts::PI;
        let x_points = 1000;

        let x_values = (0..x_points)
            .map(|i| {
                let t = i as f64 / (x_points - 1) as f64;
                x_min + t * (x_max - x_min)
            })
            .collect();

        Self {
            a0_input: "2/pi".to_string(),
            an_input: "2*sin(n)/(pi*n)".to_string(),
            bn_input: "0".to_string(),

            num_terms: 50,
            current_n: 0,

            running: false,
            animation_speed: 10.0,
            last_update: Instant::now(),

            x_min,
            x_max,
            x_points,

            x_values,
            y_values: vec![0.0; x_points],

            a0: None,
            an_func: None,
            bn_func: None,

            error: None,
            coefficients_dirty: true,
        }
    }
}

impl FourierApp {
    // ============================================================
    // Compile coefficient expressions
    // ============================================================

    fn compile_coefficients(&mut self) -> Result<(), String> {
        // --------------------------------------------------------
        // a0
        // --------------------------------------------------------

        let a0_expr = self
            .a0_input
            .parse::<meval::Expr>()
            .map_err(|e| {
                format!("Invalid a₀: {}", e)
            })?;

        let a0 = a0_expr
            .eval()
            .map_err(|e| {
                format!("Invalid a₀: {}", e)
            })?;

        // --------------------------------------------------------
        // an
        // --------------------------------------------------------

        let an_expr = self
            .an_input
            .parse::<meval::Expr>()
            .map_err(|e| {
                format!("Invalid aₙ: {}", e)
            })?;

        let an_func = an_expr
            .bind("n")
            .map_err(|e| {
                format!("Invalid aₙ: {}", e)
            })?;

        // --------------------------------------------------------
        // bn
        // --------------------------------------------------------

        let bn_expr = self
            .bn_input
            .parse::<meval::Expr>()
            .map_err(|e| {
                format!("Invalid bₙ: {}", e)
            })?;

        let bn_func = bn_expr
            .bind("n")
            .map_err(|e| {
                format!("Invalid bₙ: {}", e)
            })?;

        // --------------------------------------------------------
        // Store compiled functions
        // --------------------------------------------------------

        self.a0 = Some(a0);

        self.an_func = Some(Box::new(an_func));
        self.bn_func = Some(Box::new(bn_func));

        self.coefficients_dirty = false;
        self.error = None;

        Ok(())
    }

    // ============================================================
    // Reset Fourier series
    // ============================================================

    fn reset(&mut self) {
        self.current_n = 0;
        self.running = false;

        self.y_values.fill(0.0);

        self.error = None;
    }

    // ============================================================
    // Rebuild x coordinates
    // ============================================================

    fn rebuild_x(&mut self) {
        self.x_values = (0..self.x_points)
            .map(|i| {
                let t =
                    i as f64 / (self.x_points - 1) as f64;

                self.x_min
                    + t * (self.x_max - self.x_min)
            })
            .collect();

        self.y_values = vec![0.0; self.x_points];
    }

    // ============================================================
    // Initialize the series
    //
    // f(x) = a0/2
    //        + a1 cos(x) + b1 sin(x)
    //        + ...
    // ============================================================

    fn initialize_series(&mut self) -> Result<(), String> {
        if self.coefficients_dirty {
            self.compile_coefficients()?;
        }

        let a0 = self
            .a0
            .ok_or("a₀ has not been compiled")?;

        // Constant Fourier term.
        let constant = a0 / 2.0;

        self.y_values.fill(constant);

        self.current_n = 0;

        Ok(())
    }

    // ============================================================
    // Add exactly ONE Fourier term
    //
    // This is the main performance optimization.
    // ============================================================

    fn add_term(&mut self, n: usize) -> Result<(), String> {
        let an_func = self
            .an_func
            .as_ref()
            .ok_or("aₙ has not been compiled")?;

        let bn_func = self
            .bn_func
            .as_ref()
            .ok_or("bₙ has not been compiled")?;

        let n_f = n as f64;

        // Evaluate the coefficients ONLY ONCE.
        let an = an_func(n_f);
        let bn = bn_func(n_f);

        // Add this Fourier term to the existing curve.
        for (i, x) in self.x_values.iter().enumerate() {
            let angle = n_f * *x;

            self.y_values[i] +=
                an * angle.cos()
                + bn * angle.sin();
        }

        Ok(())
    }

    // ============================================================
    // Advance by one term
    // ============================================================

    fn advance_one_term(&mut self) -> Result<(), String> {
        if self.current_n >= self.num_terms {
            self.running = false;
            return Ok(());
        }

        let next_n = self.current_n + 1;

        self.add_term(next_n)?;

        self.current_n = next_n;

        if self.current_n >= self.num_terms {
            self.running = false;
        }

        Ok(())
    }

    // ============================================================
    // Animation
    // ============================================================

    fn update_animation(&mut self) {
        if !self.running {
            return;
        }

        let seconds_per_term =
            1.0 / self.animation_speed as f64;

        if self.last_update.elapsed()
            >= Duration::from_secs_f64(seconds_per_term)
        {
            if let Err(e) = self.advance_one_term() {
                self.error = Some(e);
                self.running = false;
            }

            self.last_update = Instant::now();
        }
    }

    // ============================================================
    // Sidebar
    // ============================================================

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Fourier Series");

                ui.separator();

                // ------------------------------------------------
                // a0
                // ------------------------------------------------

                ui.label("a₀");

                if ui
                    .text_edit_singleline(
                        &mut self.a0_input,
                    )
                    .changed()
                {
                    self.coefficients_dirty = true;
                }

                // ------------------------------------------------
                // an
                // ------------------------------------------------

                ui.add_space(6.0);

                ui.label("aₙ");

                if ui
                    .text_edit_singleline(
                        &mut self.an_input,
                    )
                    .changed()
                {
                    self.coefficients_dirty = true;
                }

                // ------------------------------------------------
                // bn
                // ------------------------------------------------

                ui.add_space(6.0);

                ui.label("bₙ");

                if ui
                    .text_edit_singleline(
                        &mut self.bn_input,
                    )
                    .changed()
                {
                    self.coefficients_dirty = true;
                }

                ui.add_space(10.0);

                ui.separator();

                // ------------------------------------------------
                // Number of terms
                // ------------------------------------------------

                ui.horizontal(|ui| {
                    ui.label("Terms:");

                    ui.add(
                        egui::DragValue::new(
                            &mut self.num_terms,
                        )
                        .range(1..=10000),
                    );
                });

                ui.label(format!(
                    "Current N: {}",
                    self.current_n
                ));

                // ------------------------------------------------
                // Animation speed
                // ------------------------------------------------

                ui.add_space(8.0);

                ui.label("Animation speed");

                ui.add(
                    egui::Slider::new(
                        &mut self.animation_speed,
                        0.1..=250.0,
                    )
                    .suffix(" terms/s"),
                );

                // ------------------------------------------------
                // Buttons
                // ------------------------------------------------

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    let button_text = if self.running {
                        "Pause"
                    } else {
                        "Start"
                    };

                    if ui.button(button_text).clicked() {
                        if self.running {
                            self.running = false;
                        } else {
                            match self.initialize_series() {
                                Ok(()) => {
                                    self.running = true;
                                    self.last_update =
                                        Instant::now();
                                }

                                Err(e) => {
                                    self.error = Some(e);
                                }
                            }
                        }
                    }

                    if ui.button("Reset").clicked() {
                        self.reset();
                    }
                });

                // ------------------------------------------------
                // X range
                // ------------------------------------------------

                ui.separator();

                ui.label("X range");

                let mut range_changed = false;

                ui.horizontal(|ui| {
                    range_changed |= ui
                        .add(
                            egui::DragValue::new(
                                &mut self.x_min,
                            )
                            .speed(0.1),
                        )
                        .changed();

                    ui.label("to");

                    range_changed |= ui
                        .add(
                            egui::DragValue::new(
                                &mut self.x_max,
                            )
                            .speed(0.1),
                        )
                        .changed();
                });

                if range_changed {
                    self.rebuild_x();
                    self.reset();
                }

                // ------------------------------------------------
                // Resolution
                // ------------------------------------------------

                ui.separator();

                ui.label("X resolution");

                if ui
                    .add(
                        egui::Slider::new(
                            &mut self.x_points,
                            10..=3000,
                        ),
                    )
                    .changed()
                {
                    self.rebuild_x();
                    self.reset();
                }

                ui.small(format!(
                    "{} points",
                    self.x_points
                ));

                // ------------------------------------------------
                // Error
                // ------------------------------------------------

                if let Some(error) = &self.error {
                    ui.separator();

                    ui.colored_label(
                        egui::Color32::RED,
                        error,
                    );
                }

                // ------------------------------------------------
                // Help
                // ------------------------------------------------

                ui.separator();

                ui.label("Examples:");

                ui.monospace(
                    "2*sin(n)/(pi*n)",
                );

                ui.monospace(
                    "4*(-1)^n/n^2",
                );

                ui.monospace(
                    "2*pi^2/3",
                );
            });
    }

    // ============================================================
    // Plot
    // ============================================================

    fn plot(&mut self, ui: &mut egui::Ui) {
        let points: PlotPoints = self
            .x_values
            .iter()
            .zip(self.y_values.iter())
            .map(|(&x, &y)| [x, y])
            .collect();

        let line = Line::new(
            format!("N = {}", self.current_n),
            points,
        )
        .width(2.0);

        Plot::new("fourier_plot")
            .allow_drag(true)
            .allow_zoom(true)
            .allow_scroll(true)
            .show(ui, |plot_ui| {
                plot_ui.line(line);
            });
    }
}

// ================================================================
// eframe application
// ================================================================

impl eframe::App for FourierApp {
    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        self.update_animation();

        self.sidebar(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Fourier Series Approximation");

            ui.label(format!(
                "N = {}",
                self.current_n
            ));

            ui.add_space(10.0);

            self.plot(ui);
        });

        // Only continuously repaint during animation.
        if self.running {
            ctx.request_repaint();
        }
    }
}

// ================================================================
// Main
// ================================================================

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 700.0])
            .with_title("Fourier Series Visualizer"),

        ..Default::default()
    };

    eframe::run_native(
        "Fourier Series Visualizer",
        options,
        Box::new(|_cc| {
            Ok(Box::new(FourierApp::default()))
        }),
    )
}