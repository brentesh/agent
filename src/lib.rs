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
}

pub struct DataResponse {
    pub content: String,
}

impl PayTypeChange {
    pub fn to_string(&self) -> String {
        let now = chrono::Local::now().naive_local().date();
        let formatted_date = if self.date.year() == now.year() {
            format!("{}", self.date.format("%a %B %d"))
        } else {
            format!("{}", self.date.format("%a %B %d, %Y"))
        };

        let from = self.old_pay_type.to_string();
        let to = format_pay_code(&self.pay_type);
        if from == to {
            return format!(
                "Pay type for {} was already set to {}",
                formatted_date, from
            );
        }
        format!(
            "Set pay type for {} from {} to {}",
            formatted_date, from, to,
        )
    }
}

pub fn format_pay_code(pay_type: &PayType) -> &'static str {
    match pay_type {
        PayType::Sick => "Sick-Sal",
        PayType::Vacation => "Vac-SAL",
        PayType::Holiday => "Hol-SAL",
        PayType::Salary => "Salary",
        PayType::Parental => "Par-SAL",
    }
}

pub enum AgentResponse {
    FunctionCall(FunctionCall),
    Message(String),
}

pub enum ExecutionResult {
    Success(ExecutionSuccess),
    Message(String),
}

pub struct ExecutionSuccess {
    pub content: String,
    pub function_call: FunctionCall,
}

pub enum ExecutionError {
    AgentError(String),
    EbmsError(String),
}

pub async fn execute_prompt(
    config: &AppConfig,
    prompt: &str,
    conversation: &Vec<ConversationMessage>,
) -> Result<ExecutionResult, ExecutionError> {
    println!("{}", format!("Calling GPT with prompt: {}", prompt));

    let gpt_result = gpt::call_gpt(&config.gpt_api_key, &prompt, conversation).await;

    match gpt_result {
        Err(e) => return Err(ExecutionError::AgentError(e.to_string())),
        Ok(AgentResponse::Message(content)) => return Ok(ExecutionResult::Message(content)),
        Ok(AgentResponse::FunctionCall(function_call)) => {
            match handle_function(config, &function_call).await {
                Ok(response) => Ok(ExecutionResult::Success(ExecutionSuccess {
                    content: response,
                    function_call: function_call,
                })),
                Err(e) => Err(ExecutionError::EbmsError(e)),
            }
        }
    }
}

async fn handle_function(
    config: &AppConfig,
    function_call: &FunctionCall,
) -> Result<String, String> {
    match function_call.name.as_str() {
        "set_pay_type" => {
            if function_call.arguments.is_empty() {
                return Err("Function call 'set_pay_type' requires arguments".to_string());
            }
            Ok(handle_pay_type_function(config, &function_call.arguments)
                .await
                .map(|changes| {
                    changes
                        .iter()
                        .map(|change| change.to_string())
                        .collect::<Vec<String>>()
                        .join("\n")
                })?)
        }
        "get_odata_url" => Ok(handle_odata_url_function(config, &function_call.arguments).await?),
        _ => Err(format!("Unknown function call: {}", function_call.name)),
    }
}

async fn handle_pay_type_function(
    config: &AppConfig,
    args: &str,
) -> Result<Vec<PayTypeChange>, String> {
    let args: serde_json::Value = serde_json::from_str(&args)
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
    let result = api::set_pay_type::set_pay_type(config, &dates, &pay_type)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

async fn handle_odata_url_function(_config: &AppConfig, args: &str) -> Result<String, String> {
    let args: serde_json::Value = serde_json::from_str(&args)
        .map_err(|e| format!("Failed to parse function call arguments: {}", e))?;
    let url = args["url"]
        .as_str()
        .ok_or_else(|| "Missing url field".to_string())?;

    let response = api::get_odata_content::get_odata_content(_config, &url)
        .await
        .map_err(|e| e.to_string())?;
    Ok(response.content)
}
