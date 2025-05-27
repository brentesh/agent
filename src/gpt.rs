use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

//these classes represent the structure of the GPT API response and are just here for deserialization
#[derive(Debug, Deserialize, Clone)]
pub struct GptFunctionCall {
    pub arguments: String,
}

#[derive(Debug, Deserialize)]
pub struct GptApiResponse {
    pub choices: Vec<GptChoice>,
}

#[derive(Debug, Deserialize)]
pub struct GptChoice {
    pub message: GptMessage,
}

#[derive(Debug, Deserialize)]
pub struct GptMessage {
    pub function_call: Option<GptFunctionCall>,
    pub content: Option<String>,
}

pub async fn call_gpt(
    api_key: &str,
    prompt: &str,
) -> Result<GptFunctionCall, Box<dyn std::error::Error>> {
    let client = Client::new();
    //this is important to let gpt know the context, otherwise it gets confused by "today", "wednesday", etc.
    let now = chrono::Local::now();
    let today = now.format("%A, %Y-%m-%d").to_string(); // e.g. "Monday, 2025-05-26"

    let full_prompt = format!(
        "You are a helpful assistant that can set pay types for employees \
         If no pay type is specified, use Salary by default \
         Today's date is {} \
         Here is the prompt: {}",
        today, prompt
    );
    let body = json!({
        "model": "gpt-4",
        "messages": [
            { "role": "user", "content": full_prompt }
        ],
        "functions": get_functions_metadata(),
        "function_call": "auto"
    });

    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        let err = res.text().await?;
        return Err(format!("GPT API Error: {}", err).into());
    }

    let response: GptApiResponse = res.json().await?;

    let choice = response.choices.first();
    match choice {
        Some(c) => {
            let message = &c.message;
            match &message.function_call {
                Some(fc) => Ok(fc.clone()),
                None => Err(message
                    .content
                    .clone()
                    .unwrap_or_else(|| "No function call found in GPT response".into())
                    .into()),
            }
        }
        None => Err("No choices returned from GPT API".into()),
    }
}

fn get_functions_metadata() -> Vec<serde_json::Value> {
    vec![json!({
        "name": "set_pay_type",
        "description": "Set a pay type for a specific date",
        "parameters": {
            "type": "object",
            "properties": {
                "date": {
                    "type": "string",
                    "description": "The date to apply the pay type (format: YYYY-MM-DD)"
                },
                "pay_type": {
                    "type": "string",
                    "enum": ["Sick", "Vacation", "Holiday", "Salary"],
                    "description": "One of: Sick, Vacation, Holiday, or Salary, Salary by default"
                }
            },
            "required": ["date", "pay_type"]
        }
    })]
}
