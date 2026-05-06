use std::time::Duration;

use reqwest::Client;
use serde_json::Value;
use tracing::debug;

#[derive(Clone, Debug)]
pub struct BackendConfig {
    pub name: &'static str,
    pub base_url: String,
}

#[derive(Clone)]
pub struct BackendRegistry {
    pub backends: Vec<BackendConfig>,
    pub client: Client,
}

impl BackendRegistry {
    /// # Panics
    ///
    /// Panics if the underlying TLS backend fails to initialise.
    #[must_use]
    pub fn new(ollama_url: String, lmstudio_url: String, llamacpp_url: String) -> Self {
        Self {
            backends: vec![
                BackendConfig {
                    name: "ollama",
                    base_url: ollama_url,
                },
                BackendConfig {
                    name: "lmstudio",
                    base_url: lmstudio_url,
                },
                BackendConfig {
                    name: "llamacpp",
                    base_url: llamacpp_url,
                },
            ],
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("failed to build reqwest client"),
        }
    }

    pub async fn online_names(&self) -> Vec<&'static str> {
        let (a, b, c) = tokio::join!(
            self.is_online(&self.backends[0]),
            self.is_online(&self.backends[1]),
            self.is_online(&self.backends[2]),
        );
        [a, b, c]
            .into_iter()
            .zip(&self.backends)
            .filter_map(|(online, backend)| online.then_some(backend.name))
            .collect()
    }

    pub async fn is_online(&self, b: &BackendConfig) -> bool {
        self.client
            .get(format!("{}/v1/models", b.base_url))
            .timeout(Duration::from_millis(500))
            .send()
            .await
            .is_ok()
    }

    #[must_use]
    pub fn find(&self, name: &str) -> Option<&BackendConfig> {
        self.backends.iter().find(|b| b.name == name)
    }

    /// Resolve which backend to use. If `preferred` is given, validate it is online.
    /// Otherwise return the first online backend.
    ///
    /// # Errors
    ///
    /// Returns an error if the preferred backend is offline, or no backends are running.
    pub async fn resolve(&self, preferred: Option<&str>) -> anyhow::Result<&BackendConfig> {
        let online = self.online_names().await;

        if let Some(name) = preferred {
            if online.contains(&name) {
                return self
                    .find(name)
                    .ok_or_else(|| anyhow::anyhow!("backend '{name}' not configured"));
            }
            anyhow::bail!("backend '{name}' is not running");
        }

        for name in &online {
            if let Some(b) = self.find(name) {
                return Ok(b);
            }
        }
        anyhow::bail!(
            "no backends running — start Ollama (11434), LM Studio (1234), or llama-server (8080)"
        )
    }

    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response body is not valid JSON.
    pub async fn list_models(&self, b: &BackendConfig) -> anyhow::Result<Vec<String>> {
        let resp: Value = self
            .client
            .get(format!("{}/v1/models", b.base_url))
            .send()
            .await?
            .json()
            .await?;

        let models: Vec<String> = resp["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| m["id"].as_str().map(str::to_string))
            .collect();

        debug!(count = models.len(), "models listed");
        Ok(models)
    }
}
