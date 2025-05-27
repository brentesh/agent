use super::{AppConfig, PayType};
use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ApiPYTMDETResponse {
    pub value: Vec<PYTMDET>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PYTMDET {
    #[serde(rename = "AUTOID")]
    pub autoid: String,
}

pub async fn set_pay_type(
    config: &AppConfig,
    date: &NaiveDate,
    pay_type: &PayType,
) -> Result<String, Box<dyn std::error::Error>> {
    let pay_code = format_pay_code(pay_type)
        .ok_or_else(|| format!("Invalid pay type: {}", pay_type.to_string()))?;
    let pytmdet_autoid = get_pytmdet_autoid(config, date).await?;
    let body = get_body(&pytmdet_autoid, pay_code);

    let url = format!(
        "{}/TimeDetailManager(c2e90ee5-3e20-473c-9b2c-979a6a2ce6e2)/Model.Entities.ModifyTimeEntries",
        config.ebms_url
    );
    println!("PATCH {}\n{}", url, body);

    let client = reqwest::Client::new();
    let res = client
        .post(&url)
        .basic_auth(
            config.ebms_username.clone(),
            Some(config.ebms_password.clone()),
        )
        .json(&body)
        .send()
        .await?;

    if res.status().is_success() {
        Ok(format!("Pay type '{}' set for {}", pay_code, date))
    } else {
        let text = res.text().await?;
        eprintln!("Error setting pay type: {}", text);
        Err(format!("Error setting pay type: {}", text).into())
    }
}

async fn get_pytmdet_autoid(
    config: &AppConfig,
    date: &NaiveDate,
) -> Result<String, Box<dyn std::error::Error>> {
    let date_str = date.format("%Y-%m-%d").to_string();
    let client = reqwest::Client::new();
    let url = format!(
        "{}/PYTMDET?$filter=ID eq '{}' and DATE eq {}T00:00:00Z&$select=AUTOID",
        config.ebms_url, config.employee_id, date_str
    );
    let res = client
        .get(&url)
        .basic_auth(
            config.ebms_username.clone(),
            Some(config.ebms_password.clone()),
        ) // Use OAuth2 or env var
        .send()
        .await?;

    if !res.status().is_success() {
        let text = res.text().await?;
        return Err(format!("Error getting time detail: {}", text).into());
    }

    let response: ApiPYTMDETResponse = res.json().await?;
    let details = response.value;
    if details.is_empty() {
        return Err(format!("No time detail found for date: {}", date).into());
    }

    let autoid = &details[0].autoid;
    if autoid.is_empty() {
        return Err(format!("No time detail found for date: {}", date).into());
    }

    println!("Found PYTMDET AUTOID: {}", autoid);
    Ok(autoid.clone())
}

fn format_pay_code(pay_type: &PayType) -> Option<&'static str> {
    match pay_type {
        PayType::Sick => Some("Vac-SAL"),
        PayType::Vacation => Some("Vac-SAL"),
        PayType::Holiday => Some("Hol-SAL"),
        PayType::Salary => Some("Salary"),
        PayType::Parental => Some("Par-SAL"),
    }
}

fn get_body(autoid: &str, pay_type: &str) -> serde_json::Value {
    serde_json::json!({
        "ModifyEntries": [
            {
                "AUTOID": autoid,
                "PayType": pay_type
            }
        ]
    })
}
