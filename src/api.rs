//! Restaurant data clients.
//!
//! Three very different sources are joined behind one CLI:
//!
//! - **FEHD restaurant licences** (official, stable XML),
//! - **OpenRice search** (public JSON endpoint used by the website),
//! - **Foodpanda nearby vendors** (public Delivery Hero listing endpoint).
//!
//! OpenRice and Foodpanda are undocumented consumer endpoints, not partner
//! APIs. They can change without notice and remain subject to each platform's
//! terms. Keeta has no public consumer API, so it is not scraped here.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const FEHD_URL: &str =
    "https://www.fehd.gov.hk/english/licensing/license/text/LP_Restaurants_EN.XML";
const OPENRICE_URL: &str = "https://www.openrice.com/api/pois";
const FOODPANDA_URL: &str = "https://disco.deliveryhero.io/listing/api/v1/pandora/vendors";
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

#[derive(Debug, Deserialize)]
struct FehdDoc {
    #[serde(rename = "GENERATION_DATE", default)]
    generation_date: String,
    #[serde(rename = "TYPE_CODE", default)]
    type_codes: Codes,
    #[serde(rename = "DIST_CODE", default)]
    district_codes: Codes,
    #[serde(rename = "LPS", default)]
    lps: Lps,
}

#[derive(Debug, Default, Deserialize)]
struct Codes {
    #[serde(rename = "CODE", default)]
    codes: Vec<Code>,
}

#[derive(Debug, Default, Deserialize)]
struct Code {
    #[serde(rename = "@ID", default)]
    id: String,
    #[serde(rename = "$text", default)]
    text: String,
}

#[derive(Debug, Default, Deserialize)]
struct Lps {
    #[serde(rename = "LP", default)]
    records: Vec<Lp>,
}

#[derive(Debug, Default, Deserialize)]
struct Lp {
    #[serde(rename = "TYPE", default)]
    licence_type: String,
    #[serde(rename = "DIST", default)]
    district: String,
    #[serde(rename = "LICNO", default)]
    licence_no: String,
    #[serde(rename = "SS", default)]
    name: String,
    #[serde(rename = "ADR", default)]
    address: String,
    #[serde(rename = "INFO", default)]
    info: String,
    #[serde(rename = "EXPDATE", default)]
    expiry: String,
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

    async fn get_text(&self, key: &str, url: &str, ttl: Duration) -> Result<String> {
        self.get_text_with(key, url, ttl, &[]).await
    }

    async fn get_text_with(
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
        let body = self
            .get_text("fehd", FEHD_URL, Duration::from_secs(6 * 3600))
            .await?;
        let doc: FehdDoc =
            quick_xml::de::from_str(&body).context("parsing FEHD restaurant licences")?;
        let types: HashMap<String, String> = doc
            .type_codes
            .codes
            .into_iter()
            .map(|code| (code.id, code.text.trim().to_string()))
            .collect();
        let districts: HashMap<String, String> = doc
            .district_codes
            .codes
            .into_iter()
            .map(|code| (code.id, code.text.trim().to_string()))
            .collect();
        let mut restaurants: Vec<Licence> = doc
            .lps
            .records
            .into_iter()
            .filter(|record| !record.name.trim().is_empty())
            .map(|record| Licence {
                licence_no: record.licence_no.trim().to_string(),
                name: record.name.trim().to_string(),
                address: record
                    .address
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" "),
                district: districts
                    .get(&record.district)
                    .cloned()
                    .unwrap_or(record.district),
                licence_type: types
                    .get(&record.licence_type)
                    .cloned()
                    .unwrap_or(record.licence_type.clone()),
                licence_type_code: record.licence_type,
                expiry_date: record.expiry.trim().to_string(),
                info: record.info.trim().to_string(),
            })
            .collect();
        restaurants.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(LicenceReport {
            generation_date: doc.generation_date.trim().to_string(),
            count: restaurants.len(),
            restaurants,
        })
    }

    /// Search OpenRice's public restaurant JSON.
    pub async fn openrice(&self, query: &str, limit: usize) -> Result<Vec<OpenRiceRestaurant>> {
        let want = limit.clamp(1, 150);
        let mut out = Vec::new();
        let mut page = 1;
        while out.len() < want && page <= 10 {
            let url = format!(
                "{OPENRICE_URL}?uiLang=en&uiCity=hongkong&page={page}&sortBy=Default&what={}",
                percent_encode(query)
            );
            let body = self.get_text(&url, &url, Duration::from_secs(300)).await?;
            let value: serde_json::Value =
                serde_json::from_str(&body).context("decoding OpenRice search")?;
            let results = value
                .pointer("/searchResult/paginationResult/results")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            if results.is_empty() {
                break;
            }
            for item in &results {
                out.push(openrice_item(item));
            }
            page += 1;
        }
        out.truncate(want);
        Ok(out)
    }

    /// List Foodpanda vendors delivering near a coordinate.
    pub async fn foodpanda(
        &self,
        latitude: f64,
        longitude: f64,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<FoodpandaRestaurant>> {
        let want = limit.clamp(1, 48);
        let url = format!(
            "{FOODPANDA_URL}?latitude={latitude}&longitude={longitude}&language_id=1&country=hk\
             &vertical=restaurants&customer_type=regular&configuration=Original&dynamic_pricing=0\
             &include=characteristics&limit={want}&offset=0"
        );
        let body = self
            .get_text_with(
                &url,
                &url,
                Duration::from_secs(120),
                &[("x-disco-client-id", "web")],
            )
            .await?;
        let value: serde_json::Value =
            serde_json::from_str(&body).context("decoding Foodpanda listing")?;
        let items = value
            .pointer("/data/items")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let needle = query.map(|value| value.to_lowercase());
        let mut out: Vec<FoodpandaRestaurant> = items
            .iter()
            .map(foodpanda_item)
            .filter(|vendor| {
                let Some(needle) = &needle else {
                    return true;
                };
                vendor.name.to_lowercase().contains(needle)
                    || vendor
                        .cuisines
                        .iter()
                        .any(|cuisine| cuisine.to_lowercase().contains(needle))
            })
            .collect();
        out.truncate(want);
        Ok(out)
    }
}

fn openrice_item(item: &serde_json::Value) -> OpenRiceRestaurant {
    let categories = item
        .get("categories")
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|category| category.get("name").and_then(serde_json::Value::as_str))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let path = item
        .get("urlUI")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    OpenRiceRestaurant {
        name: text(item, "name"),
        name_other_lang: text(item, "nameOtherLang"),
        address: text(item, "address"),
        address_other_lang: text(item, "addressOtherLang"),
        district: item
            .pointer("/district/name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        categories,
        score: item
            .get("scoreOverall")
            .and_then(serde_json::Value::as_f64)
            .filter(|score| *score > 0.0),
        review_count: item
            .get("reviewCount")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
        price: text(item, "priceUI"),
        open_now: item
            .get("openNow")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        url: if path.is_empty() {
            String::new()
        } else {
            format!("https://www.openrice.com{path}")
        },
    }
}

fn foodpanda_item(item: &serde_json::Value) -> FoodpandaRestaurant {
    let cuisines = item
        .get("cuisines")
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|cuisine| {
                    cuisine
                        .get("name")
                        .or_else(|| cuisine.get("title"))
                        .and_then(serde_json::Value::as_str)
                        .or_else(|| cuisine.as_str())
                })
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let code = text(item, "code");
    FoodpandaRestaurant {
        name: text(item, "name"),
        address: text(item, "address"),
        cuisines,
        rating: item
            .get("rating")
            .and_then(serde_json::Value::as_f64)
            .filter(|rating| *rating > 0.0),
        review_count: item
            .get("review_number")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
        budget: item.get("budget").and_then(serde_json::Value::as_u64),
        minimum_order: item
            .get("minimum_order_amount")
            .and_then(serde_json::Value::as_f64),
        delivery_fee: item
            .get("minimum_delivery_fee")
            .and_then(serde_json::Value::as_f64),
        delivery_minutes: item
            .get("minimum_delivery_time")
            .and_then(serde_json::Value::as_f64),
        distance_km: item.get("distance").and_then(serde_json::Value::as_f64),
        url: item
            .get("web_path")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("https://foodpanda.hk/restaurant/{code}")),
        code,
    }
}

fn text(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            b' ' => "+".to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
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
    fn parses_fehd_xml() {
        let xml = r#"<?xml version="1.0"?><DATA>
            <GENERATION_DATE>2026-11-21</GENERATION_DATE>
            <TYPE_CODE><CODE ID="RL">General Restaurant Licence</CODE></TYPE_CODE>
            <DIST_CODE><CODE ID="11">Eastern</CODE></DIST_CODE>
            <LPS><LP><TYPE>RL</TYPE><DIST>11</DIST><LICNO>123</LICNO>
            <SS>Test Kitchen</SS><ADR>1 Test Road</ADR><INFO></INFO>
            <EXPDATE>2027-01-01</EXPDATE></LP></LPS></DATA>"#;
        let doc: FehdDoc = quick_xml::de::from_str(xml).unwrap();
        assert_eq!(doc.generation_date, "2026-11-21");
        assert_eq!(doc.lps.records.len(), 1);
        assert_eq!(doc.lps.records[0].name, "Test Kitchen");
        assert_eq!(doc.type_codes.codes[0].text, "General Restaurant Licence");
    }

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

    #[test]
    fn encodes_queries() {
        assert_eq!(percent_encode("dim sum"), "dim+sum");
        assert_eq!(percent_encode("a/b"), "a%2Fb");
    }

    #[test]
    fn parses_openrice_item() {
        let value: serde_json::Value = serde_json::from_str(
            r#"{"name":"Sushi N","address":"1 Road","district":{"name":"Tsim Sha Tsui"},
                "categories":[{"name":"Japanese"}],"scoreOverall":4.5,"reviewCount":10,
                "priceUI":"$101-200","openNow":true,"urlUI":"/en/hongkong/r-sushi-n"}"#,
        )
        .unwrap();
        let item = openrice_item(&value);
        assert_eq!(item.name, "Sushi N");
        assert_eq!(item.district, "Tsim Sha Tsui");
        assert_eq!(item.categories, vec!["Japanese"]);
        assert!(item.url.starts_with("https://www.openrice.com/"));
    }
}
