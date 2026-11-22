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

    #[cfg(feature = "source-openrice")]
    #[test]
    fn encodes_queries() {
        assert_eq!(
            crate::sources::openrice::percent_encode("dim sum"),
            "dim+sum"
        );
        assert_eq!(crate::sources::openrice::percent_encode("a/b"), "a%2Fb");
    }

    #[cfg(feature = "source-openrice")]
    #[test]
    fn parses_openrice_item() {
        let value: serde_json::Value = serde_json::from_str(
            r#"{"name":"Sushi N","address":"1 Road","district":{"name":"Tsim Sha Tsui"},
                "categories":[{"name":"Japanese"}],"scoreOverall":4.5,"reviewCount":10,
                "priceUI":"$101-200","openNow":true,"urlUI":"/en/hongkong/r-sushi-n"}"#,
        )
        .unwrap();
        let item = crate::sources::openrice::item_value(&value);
        assert_eq!(item.name, "Sushi N");
        assert_eq!(item.district, "Tsim Sha Tsui");
        assert_eq!(item.categories, vec!["Japanese"]);
        assert!(item.url.starts_with("https://www.openrice.com/"));
    }

    #[cfg(feature = "source-openrice")]
    #[test]
    fn encodes_unicode_and_reserved_bytes() {
        assert_eq!(crate::sources::openrice::percent_encode("é"), "%C3%A9");
        assert_eq!(crate::sources::openrice::percent_encode("a&b"), "a%26b");
        assert_eq!(
            crate::sources::openrice::percent_encode("A-Z_a.z~"),
            "A-Z_a.z~"
        );
        assert_eq!(crate::sources::openrice::percent_encode(""), "");
    }

    #[cfg(feature = "source-openrice")]
    #[test]
    fn openrice_item_defaults_missing_fields() {
        let value = serde_json::json!({ "name": "  Cafe  " });
        let item = crate::sources::openrice::item_value(&value);
        assert_eq!(item.name, "Cafe");
        assert_eq!(item.address, "");
        assert_eq!(item.district, "");
        assert!(item.categories.is_empty());
        assert_eq!(item.score, None);
        assert_eq!(item.review_count, 0);
        assert!(!item.open_now);
        assert_eq!(item.url, "");
    }

    #[cfg(feature = "source-openrice")]
    #[test]
    fn openrice_hides_zero_scores() {
        let value = serde_json::json!({
            "name": "New Place",
            "scoreOverall": 0.0,
            "urlUI": ""
        });
        let item = crate::sources::openrice::item_value(&value);
        assert_eq!(item.score, None);
        assert_eq!(item.url, "");
    }

    #[cfg(feature = "source-foodpanda")]
    #[test]
    fn foodpanda_item_reads_cuisine_shapes_and_falls_back_to_code_url() {
        let value = serde_json::json!({
            "code": "abc123",
            "name": "Noodle House",
            "address": "3 Test Street",
            "cuisines": ["Thai", { "name": "Noodles" }, { "title": "Curry" }],
            "rating": 4.2,
            "review_number": 7,
            "budget": 100,
            "minimum_order_amount": 50.0,
            "minimum_delivery_fee": 5.5,
            "minimum_delivery_time": 25.0,
            "distance": 1.234,
            "web_path": null
        });
        let item = crate::sources::foodpanda::item_value(&value);
        assert_eq!(item.code, "abc123");
        assert_eq!(item.cuisines, vec!["Thai", "Noodles", "Curry"]);
        assert_eq!(item.rating, Some(4.2));
        assert_eq!(item.review_count, 7);
        assert_eq!(item.budget, Some(100));
        assert_eq!(item.minimum_order, Some(50.0));
        assert_eq!(item.delivery_fee, Some(5.5));
        assert_eq!(item.delivery_minutes, Some(25.0));
        assert_eq!(item.distance_km, Some(1.234));
        assert_eq!(item.url, "https://foodpanda.hk/restaurant/abc123");
    }

    #[cfg(feature = "source-foodpanda")]
    #[test]
    fn foodpanda_defaults_missing_metrics() {
        let value = serde_json::json!({ "code": "x", "name": "Place", "rating": 0.0 });
        let item = crate::sources::foodpanda::item_value(&value);
        assert_eq!(item.rating, None);
        assert_eq!(item.review_count, 0);
        assert_eq!(item.budget, None);
        assert_eq!(item.minimum_order, None);
        assert_eq!(item.distance_km, None);
        assert!(item.cuisines.is_empty());
    }

    #[test]
    fn filters_licences_by_district_address_and_type_code() {
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
        filter_licences(&mut report, None, Some("YAU TSIM"), None);
        assert_eq!(report.count, 1);
        assert_eq!(report.restaurants[0].name, "Corner Cafe");

        filter_licences(&mut report, Some("nathan"), None, None);
        assert_eq!(report.count, 1);

        filter_licences(&mut report, None, None, Some("rl"));
        assert_eq!(report.count, 0);
    }
}
