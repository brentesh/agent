use super::{AppConfig, PayType, PayTypeChange};
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
    #[serde(rename = "DATE")]
    pub date: String,
    #[serde(rename = "PAY_LEVEL")]
    pub pay_type: String,
}

impl PYTMDET {
    pub fn get_date(&self) -> Option<NaiveDate> {
        if self.date.len() >= 10 {
            NaiveDate::parse_from_str(&self.date[..10], "%Y-%m-%d").ok()
        } else {
            None
        }
    }
}

pub async fn set_pay_type(
    config: &AppConfig,
    dates: &Vec<NaiveDate>,
    pay_type: &PayType,
) -> Result<Vec<PayTypeChange>, Box<dyn std::error::Error>> {
    let pay_code = format_pay_code(pay_type);
    let pytmdets: Vec<PYTMDET> = get_pytmdets(config, dates).await?;
    if pytmdets.is_empty() {
        return Err(format!("No time details found for the specified dates").into());
    }

    let pytmdets_to_change: Vec<&PYTMDET> =
        pytmdets.iter().filter(|d| d.pay_type != pay_code).collect();

    if pytmdets_to_change.is_empty() {
        return Ok(output(dates, pay_type, &pytmdets));
    }

    let autoids_to_change: Vec<String> = pytmdets_to_change
        .iter()
        .map(|d| d.autoid.clone())
        .collect();
    let body = get_body(&autoids_to_change, pay_code);
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
        Ok(output(&dates, &pay_type, &pytmdets))
    } else {
        let text = res.text().await?;
        Err(format!("Error setting pay type: {}", text).into())
    }
}

fn output(
    dates: &Vec<NaiveDate>,
    pay_type: &PayType,
    pytmdets: &Vec<PYTMDET>,
) -> Vec<PayTypeChange> {
    let mut changes = Vec::new();
    println!("Outputting for dates dates: {:?}", dates);
    for date in dates {
        if let Some(old) = pytmdets.iter().find_map(|d| {
            if d.get_date() == Some(*date) {
                Some(d)
            } else {
                None
            }
        }) {
            println!(
                "Found existing pay type for date {}: {}",
                date, old.pay_type
            );
            changes.push(PayTypeChange {
                date: *date,
                old_pay_type: old.pay_type.clone(),
                pay_type: pay_type.clone(),
            });
        }
    }
    changes
}

async fn get_pytmdets(
    config: &AppConfig,
    dates: &Vec<NaiveDate>,
) -> Result<Vec<PYTMDET>, Box<dyn std::error::Error>> {
    if dates.is_empty() {
        return Ok(Vec::new());
    }

    let date_strs: Vec<String> = dates
        .iter()
        .map(|d| d.format("%Y-%m-%d").to_string())
        .collect();
    let client = reqwest::Client::new();
    // Build a filter string for multiple dates using 'or'
    let date_filters: Vec<String> = date_strs
        .iter()
        .map(|date_str| format!("DATE eq {}T00:00:00Z", date_str))
        .collect();
    let filter = format!(
        "ID eq '{}' and ({})",
        config.employee_id,
        date_filters.join(" or ")
    );
    let url = format!(
        "{}/PYTMDET?$filter={}&$select=AUTOID,DATE,PAY_LEVEL",
        config.ebms_url, filter
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
        return Err(format!("No time details found to change").into());
    }

    println!(
        "Found PYTMDET AUTOIDs: {:?}",
        details.iter().map(|d| &d.autoid).collect::<Vec<_>>()
    );
    Ok(details)
}

pub fn format_pay_code(pay_type: &PayType) -> &'static str {
    match pay_type {
        PayType::Sick => "Vac-SAL",
        PayType::Vacation => "Vac-SAL",
        PayType::Holiday => "Hol-SAL",
        PayType::Salary => "Salary",
        PayType::Parental => "Par-SAL",
    }
}

fn get_body(autoids: &Vec<String>, pay_type: &str) -> serde_json::Value {
    serde_json::json!({
        "ModifyEntries": autoids.iter().map(|autoid| {
            serde_json::json!({
            "AUTOID": autoid,
            "PayType": pay_type
            })
        }).collect::<Vec<_>>()
    })
}
