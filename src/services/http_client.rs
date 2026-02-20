use crate::config;
use crate::error::{parse_plan_limit_error, CliError};
use crate::services::auth::{load_auth_session, AuthSession};
use crate::services::device::get_device_fingerprint;

pub struct SekuireClient {
    client: reqwest::Client,
    base_url: String,
    auth: AuthSession,
}

impl SekuireClient {
    pub async fn from_registry(registry_url: &str) -> Result<Self, CliError> {
        let auth = load_auth_session()
            .await
            .map_err(|_| CliError::NotAuthenticated)?;
        Ok(Self {
            client: reqwest::Client::new(),
            base_url: registry_url.trim_end_matches('/').to_string(),
            auth,
        })
    }

    #[allow(dead_code)]
    pub async fn from_default() -> Result<Self, CliError> {
        Self::from_registry(config::DEFAULT_REGISTRY_URL).await
    }

    fn apply_auth(&self, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref key) = self.auth.api_key {
            req = req.header("X-API-Key", key);
            let fp = self
                .auth
                .device_fingerprint
                .as_deref()
                .map(|s| s.to_string())
                .unwrap_or_else(get_device_fingerprint);
            req = req.header("X-Device-Fingerprint", fp);
        } else if let Some(ref token) = self.auth.token {
            req = req.bearer_auth(token);
        }
        req
    }

    async fn check_plan_limit(
        &self,
        response: reqwest::Response,
    ) -> Result<reqwest::Response, CliError> {
        let status = response.status().as_u16();
        if status == 402 {
            let body = response.text().await.unwrap_or_default();
            if let Some(plan_err) = parse_plan_limit_error(status, &body) {
                return Err(plan_err);
            }
            return Err(CliError::RegistryError { status, body });
        }
        Ok(response)
    }

    pub async fn get(&self, path: &str) -> Result<reqwest::Response, CliError> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.client.get(&url);
        let req = self.apply_auth(req);
        Ok(req.send().await?)
    }

    pub async fn get_checked(&self, path: &str) -> Result<reqwest::Response, CliError> {
        let resp = self.get(path).await?;
        self.check_plan_limit(resp).await
    }

    pub async fn post<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<reqwest::Response, CliError> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.client.post(&url).json(body);
        let req = self.apply_auth(req);
        Ok(req.send().await?)
    }

    pub async fn post_checked<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<reqwest::Response, CliError> {
        let resp = self.post(path, body).await?;
        self.check_plan_limit(resp).await
    }
}

#[allow(dead_code)]
pub fn unauthenticated_client() -> reqwest::Client {
    reqwest::Client::new()
}
