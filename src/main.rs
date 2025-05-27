use agent::{
    ConversationMessage, Role,
    config::{AppConfig, load_config, save_config},
};
use eframe::egui::{self, Id};
use std::sync::{Arc, Mutex};

fn main() {
    let options = eframe::NativeOptions::default();
    if let Err(e) = eframe::run_native(
        "Personal Agent",
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
    ebms_url: String,
    username: String,
    password: String,
    employee_id: String,
    gpt_api_key: String,
    is_logged_in: bool,

    //main screen
    focused: bool,
    prompt: String,
    pub output: Arc<Mutex<Vec<String>>>,
    current_conversation: Arc<Mutex<Vec<ConversationMessage>>>,
    is_working: Arc<Mutex<bool>>,
}

impl Default for AgentApp {
    fn default() -> Self {
        let config = load_config();
        Self {
            ebms_url: config.ebms_url.clone(),
            username: config.ebms_username.clone(),
            password: config.ebms_password.clone(),
            employee_id: config.employee_id.clone(),
            gpt_api_key: config.gpt_api_key.clone(),
            is_logged_in: !config.ebms_username.is_empty(),
            focused: false,
            config,
            prompt: String::new(),
            output: Arc::new(Mutex::new(vec![])),
            current_conversation: Arc::new(Mutex::new(vec![])),
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
                        ui.label("EBMS API URL:");
                        ui.add_sized(
                            [300.0, 24.0],
                            egui::TextEdit::singleline(&mut self.ebms_url).hint_text(
                                "https://ecc[SERIAL NO].servicebus.windows.net/MyEbms/ECC/OData",
                            ),
                        );
                        ui.end_row();

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
                        ebms_url: self.ebms_url.clone(),
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
            ui.horizontal(|ui| {
                let id = Id::new("my_input_box");

                if !self.focused {
                    ctx.memory_mut(|mem| mem.request_focus(id));
                    self.focused = true;
                }
                ui.add(
                    egui::TextEdit::singleline(&mut self.prompt)
                        .hint_text("Type your prompt here...")
                        .id(id),
                );
                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button(">").clicked() || enter_pressed {
                    ctx.memory_mut(|mem| mem.request_focus(id));
                    let prompt = self.prompt.clone();
                    // Spawn the async task in a background thread
                    let config: AppConfig = self.config.clone();
                    {
                        let mut is_working = self.is_working.lock().unwrap();
                        *is_working = true;
                    }
                    let is_working_clone = self.is_working.clone();
                    let output_ref_clone = self.output.clone();
                    let conversation_clone = self.current_conversation.clone();
                    std::thread::spawn(move || {
                        execute_prompt(config, prompt, output_ref_clone, conversation_clone);
                        let mut is_working = is_working_clone.lock().unwrap();
                        *is_working = false;
                    });
                    self.prompt.clear();
                }
                if *self.is_working.lock().unwrap() {
                    ui.label("Working...");
                }
            });
            ui.add_space(16.0);
            if let Ok(response_lock) = self.output.lock() {
                for response in response_lock.iter().rev() {
                    ui.label(response);
                }
            }
            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                if ui.button("Log Out").clicked() {
                    self.log_out();
                }
            });
        });
    }

    fn log_out(&mut self) {
        self.prompt.clear();
        self.output.lock().unwrap().clear();
        self.current_conversation.lock().unwrap().clear();
        self.is_logged_in = false;
        self.config = AppConfig::empty();
        save_config(&self.config);
    }
}

fn execute_prompt(
    config: AppConfig,
    prompt: String,
    output: Arc<Mutex<Vec<String>>>,
    current_conversation: Arc<Mutex<Vec<ConversationMessage>>>,
) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        output
            .lock()
            .unwrap()
            .push(format!(">> {}", prompt.clone()));

        let conversation = {
            let lock = current_conversation.lock().unwrap();
            Some(lock.clone())
        }; //release lock immediately
        let result = agent::execute_prompt(&config, &prompt, &conversation).await;
        let mut output_lock = output.lock().unwrap();
        let mut conversation_lock = current_conversation.lock().unwrap();
        match result {
            Ok(pay_type_change) => {
                let text = format!(
                    "Set pay type for {} to {}",
                    pay_type_change.date, pay_type_change.pay_type
                );
                conversation_lock.clear();
                output_lock.push(text);
            }
            Err(e) => {
                conversation_lock.push(ConversationMessage::new(Role::User, prompt.clone()));
                match e {
                    agent::PayTypeError::GptError(msg) => {
                        conversation_lock.push(ConversationMessage::new(Role::Agent, msg.clone()));
                        output_lock.push(format!("Agent: {}", msg));
                    }
                    agent::PayTypeError::EbmsError(msg) => {
                        output_lock.push(format!("EBMS: {}", msg));
                    }
                }
            }
        }
    });
}
