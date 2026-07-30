use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use reqwest::{Client, StatusCode, Url};
use serde_json::{Value, json};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::Workstation;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionRequest {
    pub session_id: Uuid,
    pub source_ipv4: Ipv4Addr,
    pub workstation: Workstation,
    pub expires_at: i64,
}

#[derive(Debug, Error)]
pub enum AdmissionError {
    #[error("invalid UniFi configuration: {0}")]
    InvalidConfig(String),
    #[error("UniFi request failed")]
    Request(#[source] reqwest::Error),
    #[error("UniFi returned HTTP {0}")]
    Http(StatusCode),
    #[error("UniFi response did not contain a firewall policy id")]
    MissingPolicyId,
    #[error("UniFi firewall policy id is not a UUID")]
    InvalidPolicyId,
    #[error("admission backend state is unavailable")]
    State,
}

#[async_trait]
pub trait AdmissionBackend: Send + Sync {
    async fn open(&self, request: &AdmissionRequest) -> Result<String, AdmissionError>;
    async fn close(&self, lease_id: &str) -> Result<(), AdmissionError>;
}

#[derive(Clone, Default)]
pub struct MemoryAdmission {
    leases: Arc<Mutex<HashMap<String, AdmissionRequest>>>,
    closed: Arc<Mutex<Vec<String>>>,
}

impl MemoryAdmission {
    pub fn active_leases(&self) -> Result<Vec<AdmissionRequest>, AdmissionError> {
        self.leases
            .lock()
            .map(|leases| leases.values().cloned().collect())
            .map_err(|_| AdmissionError::State)
    }

    pub fn closed_lease_ids(&self) -> Result<Vec<String>, AdmissionError> {
        self.closed
            .lock()
            .map(|leases| leases.clone())
            .map_err(|_| AdmissionError::State)
    }
}

#[async_trait]
impl AdmissionBackend for MemoryAdmission {
    async fn open(&self, request: &AdmissionRequest) -> Result<String, AdmissionError> {
        let lease_id = format!("memory-{}", Uuid::new_v4());
        self.leases
            .lock()
            .map_err(|_| AdmissionError::State)?
            .insert(lease_id.clone(), request.clone());
        Ok(lease_id)
    }

    async fn close(&self, lease_id: &str) -> Result<(), AdmissionError> {
        self.leases
            .lock()
            .map_err(|_| AdmissionError::State)?
            .remove(lease_id);
        self.closed
            .lock()
            .map_err(|_| AdmissionError::State)?
            .push(lease_id.to_owned());
        Ok(())
    }
}

#[derive(Clone)]
pub struct UnifiAdmission {
    client: Client,
    policies_url: Url,
    api_key: String,
    external_zone_id: Uuid,
    internal_zone_id: Uuid,
}

impl UnifiAdmission {
    pub fn new(
        base_url: &str,
        site_id: Uuid,
        api_key: impl Into<String>,
        external_zone_id: Uuid,
        internal_zone_id: Uuid,
    ) -> Result<Self, AdmissionError> {
        let api_key = api_key.into();
        let mut url = Url::parse(base_url)
            .map_err(|error| AdmissionError::InvalidConfig(error.to_string()))?;
        if url.scheme() != "https" {
            return Err(AdmissionError::InvalidConfig(
                "the UniFi API URL must use HTTPS".to_owned(),
            ));
        }
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(AdmissionError::InvalidConfig(
                "the UniFi API URL cannot contain credentials, a query, or a fragment".to_owned(),
            ));
        }
        if api_key.is_empty()
            || site_id.is_nil()
            || external_zone_id.is_nil()
            || internal_zone_id.is_nil()
            || external_zone_id == internal_zone_id
        {
            return Err(AdmissionError::InvalidConfig(
                "API key and distinct non-nil site/zone UUIDs are required".to_owned(),
            ));
        }
        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }
        let policies_url = url
            .join(&format!("v1/sites/{site_id}/firewall/policies"))
            .map_err(|error| AdmissionError::InvalidConfig(error.to_string()))?;
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(AdmissionError::Request)?;
        Ok(Self {
            client,
            policies_url,
            api_key,
            external_zone_id,
            internal_zone_id,
        })
    }

    fn policy_body(&self, request: &AdmissionRequest) -> Value {
        json!({
            "enabled": true,
            "name": format!("ReplayDesktop {}", request.session_id),
            "description": format!(
                "Ephemeral direct admission; expires at Unix time {}",
                request.expires_at
            ),
            "action": {
                "type": "ALLOW",
                "allowReturnTraffic": true
            },
            "source": {
                "zoneId": self.external_zone_id,
                "trafficFilter": {
                    "type": "IP_ADDRESS",
                    "ipAddressFilter": {
                        "type": "IP_ADDRESSES",
                        "matchOpposite": false,
                        "items": [{
                            "type": "IP_ADDRESS",
                            "value": request.source_ipv4.to_string()
                        }]
                    }
                }
            },
            "destination": {
                "zoneId": self.internal_zone_id,
                "trafficFilter": {
                    "type": "IP_ADDRESS",
                    "ipAddressFilter": {
                        "type": "IP_ADDRESSES",
                        "matchOpposite": false,
                        "items": [{
                            "type": "IP_ADDRESS",
                            "value": request.workstation.lan_ipv4.to_string()
                        }]
                    },
                    "portFilter": {
                        "type": "PORTS",
                        "matchOpposite": false,
                        "items": [{
                            "type": "PORT_NUMBER",
                            "value": request.workstation.kymux_port
                        }]
                    }
                }
            },
            "ipProtocolScope": {
                "ipVersion": "IPV4",
                "protocolFilter": {
                    "type": "NAMED_PROTOCOL",
                    "matchOpposite": false,
                    "protocol": {"name": "udp"}
                }
            },
            "connectionStateFilter": ["NEW"],
            "loggingEnabled": true
        })
    }
}

#[async_trait]
impl AdmissionBackend for UnifiAdmission {
    async fn open(&self, request: &AdmissionRequest) -> Result<String, AdmissionError> {
        let response = self
            .client
            .post(self.policies_url.clone())
            .header("X-API-Key", &self.api_key)
            .json(&self.policy_body(request))
            .send()
            .await
            .map_err(AdmissionError::Request)?;
        let status = response.status();
        if !status.is_success() {
            return Err(AdmissionError::Http(status));
        }
        let body: Value = response.json().await.map_err(AdmissionError::Request)?;
        let policy_id = body
            .get("id")
            .or_else(|| body.get("data").and_then(|data| data.get("id")))
            .and_then(Value::as_str)
            .ok_or(AdmissionError::MissingPolicyId)?;
        Uuid::parse_str(policy_id)
            .map(|id| id.to_string())
            .map_err(|_| AdmissionError::InvalidPolicyId)
    }

    async fn close(&self, lease_id: &str) -> Result<(), AdmissionError> {
        let policy_id = Uuid::parse_str(lease_id).map_err(|_| AdmissionError::InvalidPolicyId)?;
        let mut url = self.policies_url.clone();
        url.path_segments_mut()
            .map_err(|()| {
                AdmissionError::InvalidConfig(
                    "UniFi API URL cannot accept a policy path".to_owned(),
                )
            })?
            .push(&policy_id.to_string());
        let response = self
            .client
            .delete(url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(AdmissionError::Request)?;
        let status = response.status();
        if !status.is_success() && status != StatusCode::NOT_FOUND {
            return Err(AdmissionError::Http(status));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unifi_requires_verified_https_transport() {
        assert!(matches!(
            UnifiAdmission::new(
                "http://192.0.2.1/proxy/network/integration/",
                Uuid::new_v4(),
                "secret",
                Uuid::new_v4(),
                Uuid::new_v4()
            ),
            Err(AdmissionError::InvalidConfig(_))
        ));
    }

    #[test]
    fn unifi_policy_is_narrowly_scoped_to_source_host_and_udp_destination() {
        let external_zone_id = Uuid::new_v4();
        let internal_zone_id = Uuid::new_v4();
        let adapter = UnifiAdmission::new(
            "https://gateway.example.test/proxy/network/integration/",
            Uuid::new_v4(),
            "secret",
            external_zone_id,
            internal_zone_id,
        )
        .expect("adapter");
        let request = AdmissionRequest {
            session_id: Uuid::new_v4(),
            source_ipv4: Ipv4Addr::new(203, 0, 113, 7),
            workstation: Workstation {
                id: Uuid::new_v4(),
                name: "render-01".to_owned(),
                lan_ipv4: Ipv4Addr::new(192, 168, 20, 41),
                kymux_port: 47990,
                wan_port: 47990,
                certificate_sha256: "ab".repeat(32),
                active: true,
            },
            expires_at: 1_800_000_000,
        };
        let policy = adapter.policy_body(&request);
        assert_eq!(policy["action"]["type"], "ALLOW");
        assert_eq!(
            policy["source"]["trafficFilter"]["ipAddressFilter"]["items"][0]["value"],
            "203.0.113.7"
        );
        assert_eq!(
            policy["destination"]["trafficFilter"]["ipAddressFilter"]["items"][0]["value"],
            "192.168.20.41"
        );
        assert_eq!(
            policy["destination"]["trafficFilter"]["portFilter"]["items"][0]["value"],
            47990
        );
        assert_eq!(
            policy["ipProtocolScope"]["protocolFilter"]["protocol"]["name"],
            "udp"
        );
        assert_eq!(policy["connectionStateFilter"], json!(["NEW"]));
    }
}
