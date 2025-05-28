use std::fmt::Display;

use chrono::Datelike;
use config::AppConfig;
use strum_macros::EnumIter;

mod api;
pub mod config;
mod gpt;

#[derive(EnumIter, Debug, Clone)]
pub enum PayType {
    Sick,
    Vacation,
    Holiday,
    Salary,
    Parental,
}

impl std::str::FromStr for PayType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Sick" => Ok(PayType::Sick),
            "Vacation" => Ok(PayType::Vacation),
            "Holiday" => Ok(PayType::Holiday),
            "Salary" => Ok(PayType::Salary),
            "Parental" => Ok(PayType::Parental),
            _ => Err(()),
        }
    }
}

impl ToString for PayType {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

pub struct PayTypeChange {
    pub date: chrono::NaiveDate,
    pub old_pay_type: String,
    pub pay_type: PayType,
}

impl Display for PayTypeChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let now = chrono::Local::now().naive_local().date();
        let formatted_dates = if let Some(date) = self.date.into() {
            if date.year() == now.year() {
                vec![format!("{}", date.format("%a %B %d"))]
            } else {
                vec![format!("{}", date.format("%a %B %d, %Y"))]
            }
        } else {
            vec![]
        };

        let from = self.old_pay_type.to_string();
        let to = self.pay_type.to_string();
        if from == to {
            return write!(
                f,
                "Pay type for {} was already set to {}",
                formatted_dates.join(", "),
                from
            );
        }
        write!(
            f,
            "Set pay type for {} from {} to {}",
            formatted_dates.join(", "),
            from,
            to,
        )
    }
}

pub enum PayTypeError {
    GptError(String),
    EbmsError(String),
}

#[derive(Clone)]
pub enum Role {
    User,
    Agent,
}

#[derive(Clone)]
pub struct ConversationMessage {
    role: Role,
    content: String,
}

impl ConversationMessage {
    pub fn new(role: Role, content: String) -> Self {
        Self { role, content }
    }
}

pub async fn execute_prompt(
    config: &AppConfig,
    prompt: &str,
    conversation: &Option<Vec<ConversationMessage>>,
) -> Result<Vec<PayTypeChange>, PayTypeError> {
    println!("{}", format!("Calling GPT with prompt: {}", prompt));

    let gpt_result: Result<gpt::GptFunctionCall, Box<dyn std::error::Error>> =
        gpt::call_gpt(&config.gpt_api_key, &prompt, conversation).await;

    let gpt::GptFunctionCall { arguments } = match gpt_result {
        Ok(result) => result,
        Err(e) => return Err(PayTypeError::GptError(e.to_string())),
    };

    match handle_api_call(config, &arguments).await {
        Ok(response) => Ok(response),
        Err(e) => Err(PayTypeError::EbmsError(e.to_string())),
    }
}

async fn handle_api_call(
    config: &AppConfig,
    function_call_arguments: &str,
) -> Result<Vec<PayTypeChange>, String> {
    println!(
        "{}",
        format!("handle api call: {}", &function_call_arguments)
    );

    let args: serde_json::Value = serde_json::from_str(function_call_arguments)
        .map_err(|e| format!("Failed to parse function call arguments: {}", e))?;

    let date_values = args["dates"]
        .as_array()
        .ok_or_else(|| "Missing or invalid 'dates' field, expected array".to_string())?;

    let mut dates = Vec::new();
    for date_val in date_values {
        let date_str = date_val
            .as_str()
            .ok_or_else(|| "Invalid date value in 'dates' array".to_string())?;
        let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| format!("Invalid date format, expected YYYY-MM-DD: {}", e))?;
        dates.push(date);
    }

    let pay_type_str = args["pay_type"]
        .as_str()
        .ok_or_else(|| "Missing pay_type field".to_string())?;
    let pay_type = pay_type_str
        .parse::<PayType>()
        .map_err(|_| format!("Invalid pay type returned from agent: {}", pay_type_str))?;

    println!("Setting pay type '{}' for dates {:?}", pay_type_str, dates);
    let result = api::set_pay_type(config, &dates, &pay_type)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}
