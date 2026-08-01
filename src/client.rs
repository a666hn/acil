use reqwest::Client;

use crate::config::ServiceConfig;

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    email: String,
    api_token: String,
}

impl ApiClient {
    pub fn new(config: &ServiceConfig) -> Self {
        Self {
            client: Client::new(),
            base_url: config.base_url.trim_end_matches('/').to_string(),
            email: config.email.clone(),
            api_token: config.api_token.clone(),
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .get(self.url(path))
            .basic_auth(&self.email, Some(&self.api_token))
    }

    pub fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .post(self.url(path))
            .basic_auth(&self.email, Some(&self.api_token))
    }

    pub fn put(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .put(self.url(path))
            .basic_auth(&self.email, Some(&self.api_token))
    }

    pub fn delete(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .delete(self.url(path))
            .basic_auth(&self.email, Some(&self.api_token))
    }
}
