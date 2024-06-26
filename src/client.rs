use crate::collection::Collection;
use crate::error::ChromaClientError;
use reqwest::header::{HeaderMap, ACCEPT, CONTENT_TYPE};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

/// Chroma Client instance.
#[derive(Debug, Clone)]
pub struct ChromaClient {
    path: String,
    client: Client,
    headers: HeaderMap,
    tenant: String,
    database: String,
}

impl ChromaClient {
    /// Creates a new ChromaClient instance.
    pub fn new(params: ChromaClientParams) -> Self {
        let http = if params.ssl { "https" } else { "http" };
        let mut headers = params.headers.unwrap_or(HeaderMap::new());
        headers.insert(ACCEPT, "application/json".parse().unwrap());
        let settings = params.settings.unwrap_or(Settings::default());

        ChromaClient {
            path: format!("{}://{}:{}", http, params.host, params.port),
            client: Client::new(),
            headers,
            tenant: settings.tenant,
            database: settings.database,
        }
    }

    async fn check_pre_flight_status(&self) -> Result<(), ChromaClientError> {
        let res = self
            .client
            .get(&format!("{}/api/v1/pre-flight-checks", self.path))
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(|e| ChromaClientError::RequestError(e))?;

        if res.status().is_success() {
            Ok(())
        } else {
            let error_message = format!("Preflight request failed, status: {}", res.status());
            Err(ChromaClientError::PreflightError(error_message))
        }
    }

    fn get_url(&self, path: &str) -> Result<Url, ChromaClientError> {
        Url::parse(&format!("{}/{}", self.path, path)).map_err(ChromaClientError::UrlParseError)
    }

    fn get_url_with_params(&self, path: &str) -> Result<Url, ChromaClientError> {
        Url::parse_with_params(
            &format!("{}/{}", self.path, path),
            &[
                ("tenant", self.tenant.clone()),
                ("database", self.database.clone()),
            ],
        )
        .map_err(ChromaClientError::UrlParseError)
    }

    /// Get the current time in nanoseconds since epoch. Used to check if the server is alive.
    pub async fn heartbeat(&self) -> Result<u64, ChromaClientError> {
        self.check_pre_flight_status().await?;
        let url = self.get_url("api/v1/heartbeat")?;

        let res = self
            .client
            .get(url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(|e| ChromaClientError::RequestError(e))?;

        let res_text = res
            .text()
            .await
            .map_err(|e| ChromaClientError::ResponseError(e))?;

        let body_json: HeartbeatResponse = serde_json::from_str(&res_text)
            .map_err(|e| ChromaClientError::ResponseParseError(e))?;

        Ok(body_json.nanosecond_heartbeat)
    }

    /// Create a new collection with the given name and metadata.
    pub async fn create_collection(
        &self,
        name: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<Collection, ChromaClientError> {
        self.check_pre_flight_status().await?;
        let url = self.get_url_with_params("api/v1/collections")?;

        let mut headers = self.headers.clone();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

        let request_body = CreateCollectionRequest {
            name: name.to_string(),
            metadata: Some(metadata).unwrap_or(None),
            get_or_create: false,
        };

        let response = self
            .client
            .post(url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await
            .map_err(ChromaClientError::RequestError)?;

        let response_text = response
            .text()
            .await
            .map_err(|e| ChromaClientError::ResponseError(e))?;

        let response_json: CreateCollectionResponse = serde_json::from_str(&response_text)
            .map_err(|e| ChromaClientError::ResponseParseError(e))?;

        Ok(Collection {
            name: response_json.name,
            id: response_json.id,
            metadata: response_json.metadata,
        })
    }

    /// Get a collection with the given name.
    pub async fn get_collection(&self, name: &str) -> Result<Collection, ChromaClientError> {
        self.check_pre_flight_status().await?;
        let url = self.get_url_with_params(&format!("api/v1/collections/{}", name))?;

        let response = self
            .client
            .get(url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(ChromaClientError::RequestError)?;

        let response_text = response
            .text()
            .await
            .map_err(|e| ChromaClientError::ResponseError(e))?;

        let response_json: Collection = serde_json::from_str(&response_text)
            .map_err(|e| ChromaClientError::ResponseParseError(e))?;

        Ok(response_json)
    }

    /// Get or create a collection with the given name and metadata.
    pub async fn get_or_create_collection(
        &self,
        name: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<Collection, ChromaClientError> {
        self.check_pre_flight_status().await?;
        let url = self.get_url_with_params("api/v1/collections")?;

        let mut headers = self.headers.clone();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

        let request_body = CreateCollectionRequest {
            name: name.to_string(),
            metadata: Some(metadata).unwrap_or(None),
            get_or_create: true,
        };

        let response = self
            .client
            .post(url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await
            .map_err(ChromaClientError::RequestError)?;

        let response_text = response
            .text()
            .await
            .map_err(|e| ChromaClientError::ResponseError(e))?;

        let response_json: CreateCollectionResponse = serde_json::from_str(&response_text)
            .map_err(|e| ChromaClientError::ResponseParseError(e))?;

        Ok(Collection {
            name: response_json.name,
            id: response_json.id,
            metadata: response_json.metadata,
        })
    }

    /// Delete a collection with the given name.
    pub async fn delete_collection(&self, name: &str) -> Result<(), ChromaClientError> {
        self.check_pre_flight_status().await?;
        let url = self.get_url_with_params(&format!("api/v1/collections/{}", name))?;

        let mut headers = self.headers.clone();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

        let response = self
            .client
            .delete(url)
            .headers(headers)
            .send()
            .await
            .map_err(ChromaClientError::RequestError)?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_message = format!(
                "Failed to delete collection with status code: {}",
                response.status()
            );
            Err(ChromaClientError::ResponseStatusError(error_message))
        }
    }

    /// List all collections.
    pub async fn list_collections(&self) -> Result<Vec<Collection>, ChromaClientError> {
        self.check_pre_flight_status().await?;
        let url = self.get_url_with_params("api/v1/collections")?;

        let response = self
            .client
            .get(url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(ChromaClientError::RequestError)?;

        if response.status().is_success() {
            let response_text = response
                .text()
                .await
                .map_err(|e| ChromaClientError::ResponseError(e))?;

            let response_json: ListCollectionsResponse = serde_json::from_str(&response_text)
                .map_err(|e| ChromaClientError::ResponseParseError(e))?;

            Ok(response_json)
        } else {
            let error_message = format!(
                "Failed to list collections with status code: {}",
                response.status()
            );
            Err(ChromaClientError::ResponseStatusError(error_message))
        }
    }

    /// Resets the database. This will delete all collections and entries.
    pub async fn reset(&self) -> Result<(), ChromaClientError> {
        self.check_pre_flight_status().await?;
        let url = self.get_url("api/v1/reset")?;

        let response = self
            .client
            .post(url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(ChromaClientError::RequestError)?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_message = format!(
                "Failed to reset with status code: {} - make sure `ALLOW_RESET=TRUE`",
                response.status()
            );
            Err(ChromaClientError::ResponseStatusError(error_message))
        }
    }

    /// Get the version of Chroma.
    pub async fn version(&self) -> Result<String, ChromaClientError> {
        self.check_pre_flight_status().await?;
        let url = self.get_url("api/v1/version")?;

        let res = self
            .client
            .get(url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(|e| ChromaClientError::RequestError(e))?;

        let res_text = res
            .text()
            .await
            .map_err(|e| ChromaClientError::ResponseError(e))?;

        Ok(res_text)
    }
}

/// The parameters to create a new client.
pub struct ChromaClientParams {
    pub host: String,
    pub port: String,
    pub ssl: bool,
    pub headers: Option<HeaderMap>,
    pub settings: Option<Settings>,
}

impl Default for ChromaClientParams {
    fn default() -> Self {
        ChromaClientParams {
            host: String::from("localhost"),
            port: String::from("8000"),
            ssl: false,
            headers: None,
            settings: Some(Settings::default()),
        }
    }
}

/// The settings for a client.
pub struct Settings {
    pub tenant: String,
    pub database: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            tenant: String::from("default_tenant"),
            database: String::from("default_database"),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct HeartbeatResponse {
    #[serde(rename = "nanosecond heartbeat")]
    nanosecond_heartbeat: u64,
}

#[derive(Serialize, Deserialize)]
struct CreateCollectionRequest {
    name: String,
    metadata: Option<HashMap<String, String>>,
    get_or_create: bool,
}

#[derive(Serialize, Deserialize)]
struct CreateCollectionResponse {
    name: String,
    id: String,
    metadata: Option<Value>,
    tenant: String,
    database: String,
}

// No need to derive Deserialize for a Vec
type ListCollectionsResponse = Vec<Collection>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn heartbeat() {
        let client = ChromaClient::new(ChromaClientParams::default());

        let default: u64 = 0;
        let hb = match client.heartbeat().await {
            Ok(hb) => hb,
            Err(ChromaClientError::RequestError(e)) => {
                eprintln!("Error during heartbeat: {}", e);
                default
            }
            Err(e) => {
                eprintln!("Unexpected error during heartbeat: {}", e);
                default
            }
        };

        assert_ne!(hb, default);
    }

    #[tokio::test]
    async fn create_and_delete() {
        let client = ChromaClient::new(ChromaClientParams::default());

        let default = Collection {
            name: "default-collection".into(),
            id: "null".into(),
            metadata: None,
        };

        let new_collection = match client.create_collection("john-doe-collection", None).await {
            Ok(new_collection) => new_collection,
            Err(ChromaClientError::RequestError(e)) => {
                eprintln!("Error during create_collection: {}", e);
                default
            }
            Err(e) => {
                eprintln!("Unexpected error during create_collection: {}", e);
                default
            }
        };

        assert_eq!(new_collection.name, "john-doe-collection");

        match client.delete_collection(&new_collection.name).await {
            Ok(_) => {}
            Err(ChromaClientError::RequestError(e)) => {
                eprintln!("Error during delete_collection: {}", e);
            }
            Err(e) => {
                eprintln!("Unexpected error during delete_collection: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn get_or_create_and_delete() {
        let client = ChromaClient::new(ChromaClientParams::default());

        let default = Collection {
            name: "default-collection".into(),
            id: "null".into(),
            metadata: None,
        };

        let new_collection = match client
            .get_or_create_collection("john-doe-g-or-c-collection", None)
            .await
        {
            Ok(new_collection) => new_collection,
            Err(ChromaClientError::RequestError(e)) => {
                eprintln!("Error during get_or_create_collection: {}", e);
                default
            }
            Err(e) => {
                eprintln!("Unexpected error during get_or_create_collection: {}", e);
                default
            }
        };

        assert_eq!(new_collection.name, "john-doe-g-or-c-collection");

        match client.delete_collection(&new_collection.name).await {
            Ok(_) => {}
            Err(ChromaClientError::RequestError(e)) => {
                eprintln!("Error during delete_collection: {}", e);
            }
            Err(e) => {
                eprintln!("Unexpected error during delete_collection: {}", e);
            }
        }
    }
}
