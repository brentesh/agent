use agent::{
    config::{AppConfig, load_config, save_config},
    conversation_message::{ConversationMessage, Role},
};
use eframe::egui::{self, Id, RichText};
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
    pub output: Arc<Mutex<Vec<RichText>>>,
    current_conversation: Arc<Mutex<Vec<ConversationMessage>>>, //this allows you to chat with the agent but gets cleared on successful changes
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
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(10.0); // Adds a bit of margin to the bottom
                ui.horizontal(|ui| {
                    let id = Id::new("my_input_box");
                    if !self.focused {
                        ctx.memory_mut(|mem| mem.request_focus(id));
                        self.focused = true;
                    }
                    ui.add(
                        egui::TextEdit::singleline(&mut self.prompt)
                            .hint_text("Type your prompt here...")
                            .id(id)
                            .font(egui::TextStyle::Heading)
                            .desired_width(350.0),
                    );
                    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    // Add a button next to the input box
                    if ui
                        .add_sized(
                            [30.0, 25.0], // width, height (height matches TextEdit)
                            egui::Button::new(
                                egui::RichText::new(">").text_style(egui::TextStyle::Heading),
                            ),
                        )
                        .clicked()
                        || enter_pressed
                    {
                        ctx.memory_mut(|mem| mem.request_focus(id));
                        self.button_clicked();
                    }
                });
                if *self.is_working.lock().unwrap() {
                    self.ellipses_animation(ctx, ui);
                    ui.add_space(-25.0); // Add negative space to reduce vertical spacing
                } else {
                    ui.add_space(10.0);
                }
                if let Ok(output_lock) = self.output.lock() {
                    for rich_text in output_lock.iter().rev() {
                        ui.label(rich_text.clone());
                    }
                }
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                ui.add_space(6.0); // Adds a bit of margin to the bottom
                if ui
                    .add_sized(
                        [60.0, 25.0], // width, height (height matches button above)
                        egui::Button::new(egui::RichText::new("Log Out")),
                    )
                    .clicked()
                {
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

    fn button_clicked(&mut self) {
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
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(execute_prompt(
                config,
                prompt,
                output_ref_clone,
                conversation_clone,
            ));
            let mut is_working = is_working_clone.lock().unwrap();
            *is_working = false;
        });
        self.prompt.clear();
    }

    fn ellipses_animation(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let speed = 2.0;
        let dots = match ((ctx.input(|i| i.time) * speed) as usize) % 3 {
            0 => ".",
            1 => "..",
            _ => "...",
        };
        ui.label(RichText::new(dots).size(40.0));
    }
}

async fn execute_prompt(
    config: AppConfig,
    prompt: String,
    output: Arc<Mutex<Vec<RichText>>>,
    current_conversation: Arc<Mutex<Vec<ConversationMessage>>>,
) {
    // Add prompt to output (lock only for this)
    {
        let mut output_lock = output.lock().unwrap();
        output_lock.push(RichText::new(format!(">> {}", prompt.clone())));
    }

    // Clone conversation for use in async call (lock only for this)
    let conversation = {
        let lock = current_conversation.lock().unwrap();
        Some(lock.clone())
    }; // lock released here

    let result = agent::execute_prompt(&config, &prompt, &conversation).await;

    let conversation_update: Option<Vec<ConversationMessage>>;
    let mut output_messages: Vec<RichText> = Vec::new();
    match result {
        Ok(changes) => {
            let mut new_conversation: Vec<ConversationMessage> = vec![];
            for change in changes {
                let change_text = change.to_string();
                output_messages.push(RichText::new(change_text.to_string()).strong());
                if let Some(function_call) = change.function_call {
                    new_conversation.push(ConversationMessage::new_content(
                        Role::Assistant,
                        change_text,
                    ));
                    new_conversation.push(ConversationMessage::new_function_call(function_call));
                }
            }

            // On success, clear conversation restart with what actually happened - this allows the agent to know how to undo
            conversation_update = Some(new_conversation);
        }
        Err(e) => {
            // On error, add user prompt to conversation
            let mut new_conversation = {
                let lock = current_conversation.lock().unwrap();
                let mut cloned = lock.clone();
                cloned.push(ConversationMessage::new_content(Role::User, prompt.clone()));
                cloned
            };
            match e {
                agent::PayTypeError::GptError(msg) => {
                    new_conversation.push(ConversationMessage::new_content(
                        Role::Assistant,
                        msg.clone(),
                    ));
                    output_messages.push(RichText::new(format!("Agent: {}", msg)));
                }
                agent::PayTypeError::EbmsError(msg) => {
                    output_messages.push(RichText::new(format!("EBMS: {}", msg)));
                }
            }
            conversation_update = Some(new_conversation);
        }
    }

    // Apply updates (lock only for this)
    if let Some(new_conversation) = conversation_update {
        let mut conversation_lock = current_conversation.lock().unwrap();
        *conversation_lock = new_conversation;
    }

    if !output_messages.is_empty() {
        let mut output_lock = output.lock().unwrap();
        for new_output in output_messages {
            // Lock the output and push the new output
            output_lock.push(new_output);
        }
    }
}
