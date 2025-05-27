use config::AppConfig;

mod api;
pub mod config;
mod gpt;

pub struct PayTypeChange {
    pub date: String,
    pub pay_type: String,
}

pub async fn execute_prompt(config: &AppConfig, prompt: &str) -> Result<PayTypeChange, String> {
    println!("{}", format!("Calling GPT with prompt: {}", prompt));

    let gpt_result: Result<gpt::GptFunctionCall, Box<dyn std::error::Error>> =
        gpt::call_gpt(&config.gpt_api_key, &prompt).await;

    let gpt::GptFunctionCall { arguments } = match gpt_result {
        Ok(result) => result,
        Err(e) => return Err(format!("Error calling GPT: {}", e)),
    };

    match handle_api_call(config, &arguments).await {
        Ok(response) => Ok(response),
        Err(e) => Err(format!("Error handling API call: {}", e)),
    }
}

async fn handle_api_call(
    config: &AppConfig,
    function_call_arguments: &str,
) -> Result<PayTypeChange, String> {
    println!(
        "{}",
        format!("handle api call: {}", &function_call_arguments)
    );

    let args: serde_json::Value = serde_json::from_str(function_call_arguments)
        .map_err(|e| format!("Failed to parse function call arguments: {}", e))?;

    let date_str = args["date"].as_str().unwrap(); //TODO: handle errors
    let pay_type = args["pay_type"].as_str().unwrap();

    let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .expect("Invalid date format, expected YYYY-MM-DD");

    println!("Setting pay type '{}' for date {}", pay_type, date);
    api::set_pay_type(config, date, pay_type)
        .await
        .map_err(|e| e.to_string())?;

    Ok(PayTypeChange {
        date: date_str.to_string(),
        pay_type: pay_type.to_string(),
    })
}
