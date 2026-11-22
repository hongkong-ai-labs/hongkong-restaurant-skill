//! Restaurant models and source orchestration.
//!
//! FEHD, OpenRice and Foodpanda each live in `src/sources/` behind a Cargo
//! feature. OpenRice and Foodpanda are undocumented consumer endpoints, not
//! partner APIs; they remain subject to each platform's terms.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const BROWSER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0 Safari/537.36";

/// One FEHD licensed restaurant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Licence {
    pub licence_no: String,
    pub name: String,
    pub address: String,
    pub district: String,
    pub licence_type: String,
    pub licence_type_code: String,
    pub expiry_date: String,
    pub info: String,
}

/// The FEHD licence list.
#[derive(Debug, Clone, Serialize)]
pub struct LicenceReport {
    pub generation_date: String,
    pub count: usize,
    pub restaurants: Vec<Licence>,
}

/// One OpenRice search result.
#[derive(Debug, Clone, Serialize)]
pub struct OpenRiceRestaurant {
    pub name: String,
    pub name_other_lang: String,
    pub address: String,
    pub address_other_lang: String,
    pub district: String,
    pub categories: Vec<String>,
    pub score: Option<f64>,
    pub review_count: u64,
    pub price: String,
    pub open_now: bool,
    pub url: String,
}

/// One Foodpanda vendor.
#[derive(Debug, Clone, Serialize)]
pub struct FoodpandaRestaurant {
    pub code: String,
    pub name: String,
    pub address: String,
    pub cuisines: Vec<String>,
    pub rating: Option<f64>,
    pub review_count: u64,
    pub budget: Option<u64>,
    pub minimum_order: Option<f64>,
    pub delivery_fee: Option<f64>,
    pub delivery_minutes: Option<f64>,
    pub distance_km: Option<f64>,
    pub url: String,
}

/// Cached restaurant client.
pub struct Client {
    http: reqwest::Client,
    cache: Mutex<HashMap<String, (Instant, String)>>,
}

impl Client {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(BROWSER_AGENT)
            .timeout(Duration::from_secs(45))
            .build()?;
        Ok(Self {
            http,
            cache: Mutex::new(HashMap::new()),
        })
    }

    // Used by the compiled-in source modules.
    #[allow(dead_code)]
    pub(crate) async fn get_text(&self, key: &str, url: &str, ttl: Duration) -> Result<String> {
        self.get_text_with(key, url, ttl, &[]).await
    }

    #[allow(dead_code)]
    pub(crate) async fn get_text_with(
        &self,
        key: &str,
        url: &str,
        ttl: Duration,
        headers: &[(&str, &str)],
    ) -> Result<String> {
        if let Some((at, body)) = self
            .cache
            .lock()
            .expect("restaurant cache poisoned")
            .get(key)
            .cloned()
            && at.elapsed() < ttl
        {
            return Ok(body);
        }
        let mut request = self.http.get(url);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let response = request
            .send()
            .await
            .with_context(|| format!("requesting {url}"))?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            bail!("{url} returned HTTP {status}");
        }
        self.cache
            .lock()
            .expect("restaurant cache poisoned")
            .insert(key.to_string(), (Instant::now(), body.clone()));
        Ok(body)
    }

    /// Fetch and parse the official FEHD restaurant-licence XML.
    pub async fn licences(&self) -> Result<LicenceReport> {
        #[cfg(feature = "source-fehd")]
        {
            return crate::sources::fehd::licences(self).await;
        }
        #[cfg(not(feature = "source-fehd"))]
        bail!("FEHD source is not compiled into this binary")
    }

    /// Search OpenRice's public restaurant JSON.
    pub async fn openrice(&self, query: &str, limit: usize) -> Result<Vec<OpenRiceRestaurant>> {
        #[cfg(feature = "source-openrice")]
        {
            return crate::sources::openrice::search(self, query, limit).await;
        }
        #[cfg(not(feature = "source-openrice"))]
        {
            let _ = (query, limit);
            bail!("OpenRice source is not compiled into this binary")
        }
    }

    /// List Foodpanda vendors delivering near a coordinate.
    pub async fn foodpanda(
        &self,
        latitude: f64,
        longitude: f64,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<FoodpandaRestaurant>> {
        #[cfg(feature = "source-foodpanda")]
        {
            return crate::sources::foodpanda::nearby(self, latitude, longitude, query, limit)
                .await;
        }
        #[cfg(not(feature = "source-foodpanda"))]
        {
            let _ = (latitude, longitude, query, limit);
            bail!("Foodpanda source is not compiled into this binary")
        }
    }
}

/// Filter the licence list by name/address, district and licence type.
pub fn filter_licences(
    report: &mut LicenceReport,
    query: Option<&str>,
    district: Option<&str>,
    kind: Option<&str>,
) {
    let query = query.map(|value| value.to_lowercase());
    let district = district.map(|value| value.to_lowercase());
    let kind = kind.map(|value| value.to_lowercase());
    report.restaurants.retain(|restaurant| {
        if let Some(query) = &query
            && !restaurant.name.to_lowercase().contains(query)
            && !restaurant.address.to_lowercase().contains(query)
        {
            return false;
        }
        if let Some(district) = &district
            && !restaurant.district.to_lowercase().contains(district)
        {
            return false;
        }
        if let Some(kind) = &kind
            && !restaurant.licence_type.to_lowercase().contains(kind)
            && !restaurant.licence_type_code.to_lowercase().contains(kind)
        {
            return false;
        }
        true
    });
    report.count = report.restaurants.len();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_licences() {
        let mut report = LicenceReport {
            generation_date: "2026-11-21".into(),
            count: 2,
            restaurants: vec![
                Licence {
                    licence_no: "1".into(),
                    name: "Jade Garden".into(),
                    address: "1 Queensway".into(),
                    district: "Central/Western".into(),
                    licence_type: "General Restaurant Licence".into(),
                    licence_type_code: "RL".into(),
                    expiry_date: "2027-01-01".into(),
                    info: String::new(),
                },
                Licence {
                    licence_no: "2".into(),
                    name: "Corner Cafe".into(),
                    address: "2 Nathan Road".into(),
                    district: "Yau Tsim".into(),
                    licence_type: "Light Refreshment Restaurant Licence".into(),
                    licence_type_code: "RR".into(),
                    expiry_date: "2027-01-01".into(),
                    info: String::new(),
                },
            ],
        };
        filter_licences(&mut report, Some("jade"), None, None);
        assert_eq!(report.count, 1);
        filter_licences(&mut report, None, None, Some("rr"));
        assert_eq!(report.count, 0);
    }
}
