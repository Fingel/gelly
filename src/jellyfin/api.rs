use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AuthenticateResponse {
    #[serde(rename = "AccessToken")]
    pub access_token: String,
    #[serde(rename = "User")]
    pub user: User,
}

#[derive(Debug, Deserialize)]
pub struct User {
    #[serde(rename = "Id")]
    pub id: String,
}
