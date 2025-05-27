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
    is_working: Arc<Mutex<bool>>,
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
            is_working: Arc::new(Mutex::new(false)),
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
                ui.text_edit_singleline(&mut self.prompt).request_focus();
                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button(">").clicked() || enter_pressed {
                    let prompt = self.prompt.clone();
                    // Spawn the async task in a background thread
                    let config: AppConfig = self.config.clone();
                    {
                        let mut is_working = self.is_working.lock().unwrap();
                        *is_working = true;
                    }
                    let is_working_clone = self.is_working.clone();
                    let gpt_responses_clone = self.gpt_responses.clone();
                    std::thread::spawn(move || {
                        execute_prompt(config, prompt, gpt_responses_clone);
                        let mut is_working = is_working_clone.lock().unwrap();
                        *is_working = false;
                    });
                }
                if *self.is_working.lock().unwrap() {
                    ui.label("Working...");
                }
            });
            ui.add_space(16.0);
            if let Ok(response_lock) = self.gpt_responses.lock() {
                for response in response_lock.iter().rev() {
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

fn execute_prompt(config: AppConfig, prompt: String, gpt_responses: Arc<Mutex<Vec<String>>>) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let result = agent::execute_prompt(&config, &prompt).await;
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
}
