use super::super::{AppConfig, DataResponse};

pub async fn get_odata_content(
    config: &AppConfig,
    url: &str,
) -> Result<DataResponse, Box<dyn std::error::Error>> {
    let url = format!("{}/{}", config.ebms_url, url);
    println!("GET {}", url);

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .basic_auth(
            config.ebms_username.clone(),
            Some(config.ebms_password.clone()),
        )
        .send()
        .await?;

    if res.status().is_success() {
        Ok(DataResponse {
            content: res.json().await?,
        })
    } else {
        let text = res.text().await?;
        Err(format!("Error getting data: {}", text).into())
    }
}
