//! Restaurant tables.

use crate::api::{FoodpandaRestaurant, Licence, OpenRiceRestaurant};

fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if index < widths.len() {
                widths[index] = widths[index].max(cell.chars().count());
            }
        }
    }
    let mut out = String::new();
    out.push_str(
        headers
            .iter()
            .enumerate()
            .map(|(index, header)| format!("{header:<width$}", width = widths[index]))
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end(),
    );
    out.push('\n');
    out.push_str(
        &widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<_>>()
            .join("  "),
    );
    out.push('\n');
    for row in rows {
        out.push_str(
            row.iter()
                .enumerate()
                .map(|(index, cell)| format!("{cell:<width$}", width = widths[index]))
                .collect::<Vec<_>>()
                .join("  ")
                .trim_end(),
        );
        out.push('\n');
    }
    out
}

fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    let mut out: String = value.chars().take(limit.saturating_sub(1)).collect();
    out.push('…');
    out
}

pub fn licences(restaurants: &[Licence]) -> String {
    if restaurants.is_empty() {
        return "No matching licensed restaurants.".to_string();
    }
    let rows: Vec<Vec<String>> = restaurants
        .iter()
        .map(|restaurant| {
            vec![
                truncate(&restaurant.name, 40),
                restaurant.district.clone(),
                truncate(&restaurant.licence_type, 30),
                truncate(&restaurant.address, 56),
                restaurant.expiry_date.clone(),
            ]
        })
        .collect();
    table(
        &["NAME", "DISTRICT", "LICENCE", "ADDRESS", "EXPIRES"],
        &rows,
    )
    .trim_end()
    .to_string()
}

pub fn openrice(restaurants: &[OpenRiceRestaurant]) -> String {
    if restaurants.is_empty() {
        return "No OpenRice matches.".to_string();
    }
    let rows: Vec<Vec<String>> = restaurants
        .iter()
        .map(|restaurant| {
            vec![
                truncate(&restaurant.name, 34),
                restaurant
                    .score
                    .map(|score| format!("{score:.1}"))
                    .unwrap_or_else(|| "-".to_string()),
                restaurant.review_count.to_string(),
                restaurant.price.clone(),
                restaurant.district.clone(),
                if restaurant.open_now {
                    "open"
                } else {
                    "closed"
                }
                .to_string(),
                truncate(&restaurant.address, 56),
            ]
        })
        .collect();
    table(
        &[
            "NAME", "SCORE", "REVIEWS", "PRICE", "DISTRICT", "NOW", "ADDRESS",
        ],
        &rows,
    )
    .trim_end()
    .to_string()
}

pub fn foodpanda(restaurants: &[FoodpandaRestaurant]) -> String {
    if restaurants.is_empty() {
        return "No Foodpanda matches.".to_string();
    }
    let rows: Vec<Vec<String>> = restaurants
        .iter()
        .map(|restaurant| {
            vec![
                truncate(&restaurant.name, 34),
                restaurant
                    .rating
                    .map(|rating| format!("{rating:.1}"))
                    .unwrap_or_else(|| "-".to_string()),
                restaurant
                    .minimum_order
                    .map(|value| format!("${value:.0}"))
                    .unwrap_or_else(|| "-".to_string()),
                restaurant
                    .delivery_fee
                    .map(|value| format!("${value:.0}"))
                    .unwrap_or_else(|| "-".to_string()),
                restaurant
                    .delivery_minutes
                    .map(|value| format!("{value:.0}m"))
                    .unwrap_or_else(|| "-".to_string()),
                restaurant
                    .distance_km
                    .map(|value| format!("{value:.2}km"))
                    .unwrap_or_else(|| "-".to_string()),
                truncate(&restaurant.address, 56),
            ]
        })
        .collect();
    table(
        &["NAME", "RATING", "MIN", "FEE", "ETA", "DIST", "ADDRESS"],
        &rows,
    )
    .trim_end()
    .to_string()
}
