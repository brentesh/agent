use agent::config::{AppConfig, load_config, save_config};
use eframe::egui;
use std::sync::{Arc, Mutex};

fn main() {
    let options = eframe::NativeOptions::default();
    if let Err(e) = eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| {
            Ok::<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>(Box::new(
                AgentApp::default(),
            ))
        }),
    ) {
        eprintln!("Failed to launch the app: {}", e);
    }
}

struct AgentApp {
    pub config: AppConfig,

    // Login form fields
    username: String,
    password: String,
    employee_id: String,
    gpt_api_key: String,
    is_logged_in: bool,

    //prompts
    prompt: String,
    pub gpt_responses: Arc<Mutex<Vec<String>>>,
    is_working: bool,
}

impl Default for AgentApp {
    fn default() -> Self {
        let config = load_config();
        Self {
            username: config.ebms_username.clone(),
            password: config.ebms_password.clone(),
            employee_id: config.employee_id.clone(),
            gpt_api_key: config.gpt_api_key.clone(),
            is_logged_in: !config.ebms_username.is_empty(),
            config,
            prompt: String::new(),
            gpt_responses: Arc::new(Mutex::new(vec![])),
            is_working: false,
        }
    }
}

impl eframe::App for AgentApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.is_logged_in {
            self.draw_main_ui(ctx, frame);
        } else {
            self.draw_login(ctx, frame);
        }
    }
}

impl AgentApp {
    fn draw_login(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Login");
            ui.vertical_centered(|ui| {
                egui::Grid::new("login_grid")
                    .spacing([16.0, 8.0])
                    .striped(true)
                    .min_col_width(120.0)
                    .show(ui, |ui| {
                        ui.label("Username:");
                        ui.add_sized(
                            [300.0, 24.0],
                            egui::TextEdit::singleline(&mut self.username),
                        );
                        ui.end_row();

                        ui.label("Password:");
                        ui.add_sized(
                            [300.0, 24.0],
                            egui::TextEdit::singleline(&mut self.password).password(true),
                        );
                        ui.end_row();

                        ui.label("Employee ID:");
                        ui.add_sized(
                            [300.0, 24.0],
                            egui::TextEdit::singleline(&mut self.employee_id),
                        );
                        ui.end_row();

                        ui.label("GPT API Key:");
                        ui.add_sized(
                            [300.0, 24.0],
                            egui::TextEdit::singleline(&mut self.gpt_api_key).password(true),
                        );
                        ui.end_row();
                    });
            });

            ui.add_space(10.0);
            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui
                .add_sized([120.0, 32.0], egui::Button::new("Log In"))
                .clicked()
                || enter_pressed
            {
                if !self.username.is_empty()
                    && !self.password.is_empty()
                    && !self.employee_id.is_empty()
                {
                    self.is_logged_in = true;

                    self.config = AppConfig {
                        ebms_username: self.username.clone(),
                        ebms_password: self.password.clone(),
                        employee_id: self.employee_id.clone(),
                        gpt_api_key: self.gpt_api_key.clone(),
                    };
                    save_config(&self.config);
                }
            }
        });
    }

    fn draw_main_ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Enter a prompt for the agent to execute!");
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.label("Prompt");
                ui.text_edit_singleline(&mut self.prompt).request_focus();
                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button(">").clicked() || enter_pressed {
                    let prompt = self.prompt.clone();
                    // Spawn the async task in a background thread
                    let config: AppConfig = self.config.clone();
                    self.is_working = true;
                    let is_working_ptr = Arc::new(Mutex::new(&mut self.is_working));
                    let gpt_responses_clone = self.gpt_responses.clone();
                    execute_prompt(config, prompt, is_working_ptr, gpt_responses_clone);
                }
                if self.is_working {
                    let time = ui.input(|i| i.time) as f32;
                    let angle = time * 2.0 * std::f32::consts::PI;
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                    ui.painter().add(egui::epaint::CircleShape {
                        center: rect.center(),
                        radius: 8.0,
                        fill: egui::Color32::TRANSPARENT,
                        stroke: egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255)),
                    });
                    // Draw arc manually since PathShape::arc does not exist
                    let center = rect.center();
                    let radius = 8.0;
                    let start_angle = angle;
                    let end_angle = angle + std::f32::consts::PI * 1.5;
                    let num_points = 32;
                    let mut points = Vec::with_capacity(num_points + 1);
                    for i in 0..=num_points {
                        let t = i as f32 / num_points as f32;
                        let theta = start_angle + t * (end_angle - start_angle);
                        points.push(center + egui::vec2(theta.cos(), theta.sin()) * radius);
                    }
                    ui.painter().add(egui::epaint::PathShape::line(
                        points,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255)),
                    ));
                }
            });
            ui.add_space(16.0);
            if let Ok(response_lock) = self.gpt_responses.lock() {
                for response in response_lock.iter() {
                    ui.label(response);
                }
            }
            ui.add_space(16.0);
            if ui.button("Log Out").clicked() {
                self.is_logged_in = false;
                self.config = AppConfig::empty();
                save_config(&self.config);
            }
        });
    }
}

fn execute_prompt(
    config: AppConfig,
    prompt: String,
    is_working_ptr: Arc<Mutex<&mut bool>>,
    gpt_responses: Arc<Mutex<Vec<String>>>,
) {
    let is_working_clone = is_working_ptr.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let result = agent::execute_prompt(&config, &prompt).await;
            let mut is_working = is_working_clone.lock().unwrap();
            **is_working = false;

            let mut response_lock = gpt_responses.lock().unwrap();
            match result {
                Ok(pay_type_change) => {
                    let text = format!(
                        "Set pay type for {} to {}",
                        pay_type_change.date, pay_type_change.pay_type
                    );
                    response_lock.push(text);
                }
                Err(e) => {
                    response_lock.push(e.to_string());
                }
            }
        });
    });
}
