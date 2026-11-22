//! Foodpanda nearby-vendor listing.

use anyhow::{Context, Result};

use crate::api::{Client, FoodpandaRestaurant};

const URL: &str = "https://disco.deliveryhero.io/listing/api/v1/pandora/vendors";

pub async fn nearby(
    client: &Client,
    latitude: f64,
    longitude: f64,
    query: Option<&str>,
    limit: usize,
) -> Result<Vec<FoodpandaRestaurant>> {
    let want = limit.clamp(1, 48);
    let url = format!(
        "{URL}?latitude={latitude}&longitude={longitude}&language_id=1&country=hk\
         &vertical=restaurants&customer_type=regular&configuration=Original&dynamic_pricing=0\
         &include=characteristics&limit={want}&offset=0"
    );
    let body = client
        .get_text_with(
            &url,
            &url,
            std::time::Duration::from_secs(120),
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
        .map(item_value)
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

pub(crate) fn item_value(item: &serde_json::Value) -> FoodpandaRestaurant {
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
