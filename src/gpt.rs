use crate::PayType;

use super::{ConversationMessage, FunctionCall, Role};
use crate::api::format_pay_code;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use strum::IntoEnumIterator;

impl Role {
    fn as_str(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
        }
    }
}

//these classes represent the structure of the GPT API response and are just here for deserialization
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
    pub function_call: Option<FunctionCall>,
    pub content: Option<String>,
}

pub async fn call_gpt(
    api_key: &str,
    prompt: &str,
    conversation: &Option<Vec<ConversationMessage>>,
) -> Result<FunctionCall, Box<dyn std::error::Error>> {
    let client = Client::new();
    //this is important to let gpt know the context, otherwise it gets confused by "today", "wednesday", etc.
    let now = chrono::Local::now();
    let today = now.format("%A, %Y-%m-%d").to_string(); // e.g. "Monday, 2025-05-26"

    let mut full_conversation: Vec<ConversationMessage> = vec![ConversationMessage::new_content(
        Role::Assistant,
        format!(
            "You are a helpful assistant that can set pay types for employees. \
             If no pay type is specified, use Salary by default. \
             Today's date is {}, the week begins on Sunday \
             If the user asks you to undo a change and the record shows that you made a change, you should set it back to what you originally said it was.",
            today
        ),
    )];

    if let Some(conv) = conversation {
        if !conv.is_empty() {
            full_conversation.extend(conv.clone());
        }
    }

    full_conversation.push(ConversationMessage::new_content(
        Role::User,
        prompt.to_string(),
    ));

    let body = json!({
        "model": "gpt-4",
        "messages": full_conversation
            .iter()
            .map(|msg| {
                let mut obj = serde_json::json!({
                    "role": msg.role.as_str(),
                    "content": msg.content,
                });
                if let Some(fc) = &msg.function_call {
                    if let serde_json::Value::Object(ref mut map) = obj {
                        map.insert(
                            "function_call".to_string(),
                            serde_json::json!({
                                "name": fc.name,
                                "arguments": fc.arguments
                            }),
                        );
                    }
                }
                obj
            })
            .collect::<Vec<_>>(),
        "functions": get_functions_metadata(),
        "function_call": "auto"
    });

    println!("Calling GPT with body: {}", body);

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
        "description": "Set a pay type for a set of dates",
        "parameters": {
            "type": "object",
            "properties": {
                "dates": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "description": "A date to apply the pay type (format: YYYY-MM-DD)"
                    },
                    "description": "The dates to apply the pay type (format: YYYY-MM-DD)",
                    "minItems": 1
                },
                "pay_type": {
                    "type": "string",
                    "enum": PayType::iter().map(|pt| pt.to_string()).collect::<Vec<_>>(),
                    "description": &format!(
                        "One of: {}. Salary by default. See this mapping for details: {}",
                        PayType::iter()
                            .map(|pt| pt.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                        PayType::iter()
                    .map(|pt| format!("{} is {}", &pt.to_string(), format_pay_code(&pt)))
                    .collect::<Vec<_>>()
                    .join(", ")
                    )
                }
            },
            "required": ["dates", "pay_type"],
            "additionalProperties": false
        }
    })]
}
