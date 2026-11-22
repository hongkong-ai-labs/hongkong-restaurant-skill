//! OpenRice public search JSON.

use anyhow::{Context, Result};

use crate::api::{Client, OpenRiceRestaurant};

const URL: &str = "https://www.openrice.com/api/pois";

pub async fn search(client: &Client, query: &str, limit: usize) -> Result<Vec<OpenRiceRestaurant>> {
    let want = limit.clamp(1, 150);
    let mut out = Vec::new();
    let mut page = 1;
    while out.len() < want && page <= 10 {
        let url = format!(
            "{URL}?uiLang=en&uiCity=hongkong&page={page}&sortBy=Default&what={}",
            percent_encode(query)
        );
        let body = client
            .get_text(&url, &url, std::time::Duration::from_secs(300))
            .await?;
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
            out.push(item_value(item));
        }
        page += 1;
    }
    out.truncate(want);
    Ok(out)
}

fn item_value(item: &serde_json::Value) -> OpenRiceRestaurant {
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let item = item_value(&value);
        assert_eq!(item.name, "Sushi N");
        assert_eq!(item.district, "Tsim Sha Tsui");
        assert_eq!(item.categories, vec!["Japanese"]);
        assert!(item.url.starts_with("https://www.openrice.com/"));
    }
}
