use std::fmt::Display;

use api::format_pay_code;
use chrono::Datelike;
use config::AppConfig;
use conversation_message::{ConversationMessage, FunctionCall};
use strum_macros::EnumIter;

mod api;
pub mod config;
pub mod conversation_message;
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
    pub function_call: Option<FunctionCall>,
}

impl Display for PayTypeChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let now = chrono::Local::now().naive_local().date();
        let formatted_date = if self.date.year() == now.year() {
            format!("{}", self.date.format("%a %B %d"))
        } else {
            format!("{}", self.date.format("%a %B %d, %Y"))
        };

        let from = self.old_pay_type.to_string();
        let to = format_pay_code(&self.pay_type);
        if from == to {
            return write!(
                f,
                "Pay type for {} was already set to {}",
                formatted_date, from
            );
        }
        write!(
            f,
            "Set pay type for {} from {} to {}",
            formatted_date, from, to,
        )
    }
}

impl PayTypeChange {
    pub fn get_function_call(&self) -> Option<String> {
        let from = self.old_pay_type.to_string();
        let to = format_pay_code(&self.pay_type);
        if from == to {
            return None;
        }
        return Some(format!(
            "I set the pay type for {} from {} to {}",
            self.date.format("%a %B %d, %Y"),
            from,
            to,
        ));
    }
}

pub enum PayTypeError {
    GptError(String),
    EbmsError(String),
}

pub async fn execute_prompt(
    config: &AppConfig,
    prompt: &str,
    conversation: &Option<Vec<ConversationMessage>>,
) -> Result<Vec<PayTypeChange>, PayTypeError> {
    println!("{}", format!("Calling GPT with prompt: {}", prompt));

    let gpt_result: Result<FunctionCall, Box<dyn std::error::Error>> =
        gpt::call_gpt(&config.gpt_api_key, &prompt, conversation).await;

    let function_call = match gpt_result {
        Ok(result) => result,
        Err(e) => return Err(PayTypeError::GptError(e.to_string())),
    };

    match handle_api_call(config, &function_call).await {
        Ok(response) => Ok(response),
        Err(e) => Err(PayTypeError::EbmsError(e.to_string())),
    }
}

async fn handle_api_call(
    config: &AppConfig,
    function_call: &FunctionCall,
) -> Result<Vec<PayTypeChange>, String> {
    let args: serde_json::Value = serde_json::from_str(&function_call.arguments)
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
    // Serialize the GPT function call to a JSON string for logging or debugging
    let result = api::set_pay_type(config, &dates, &pay_type, &function_call)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}
