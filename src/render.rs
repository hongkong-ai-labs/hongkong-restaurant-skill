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

#[cfg(test)]
mod tests {
    use super::*;

    fn licence() -> Licence {
        Licence {
            licence_no: "1".to_string(),
            name: "Jade Garden".to_string(),
            address: "1 Queensway".to_string(),
            district: "Central/Western".to_string(),
            licence_type: "General Restaurant Licence".to_string(),
            licence_type_code: "RL".to_string(),
            expiry_date: "2027-01-01".to_string(),
            info: String::new(),
        }
    }

    fn openrice_item() -> OpenRiceRestaurant {
        OpenRiceRestaurant {
            name: "Sushi N".to_string(),
            name_other_lang: String::new(),
            address: "1 Road".to_string(),
            address_other_lang: String::new(),
            district: "Tsim Sha Tsui".to_string(),
            categories: vec!["Japanese".to_string()],
            score: Some(4.5),
            review_count: 10,
            price: "$101-200".to_string(),
            open_now: true,
            url: String::new(),
        }
    }

    fn foodpanda_item() -> FoodpandaRestaurant {
        FoodpandaRestaurant {
            code: "abc".to_string(),
            name: "Noodle House".to_string(),
            address: "3 Test Street".to_string(),
            cuisines: vec!["Thai".to_string()],
            rating: Some(4.2),
            review_count: 7,
            budget: Some(100),
            minimum_order: Some(50.0),
            delivery_fee: Some(5.5),
            delivery_minutes: Some(25.0),
            distance_km: Some(1.234),
            url: String::new(),
        }
    }

    #[test]
    fn licence_table_reports_empty_results() {
        assert_eq!(licences(&[]), "No matching licensed restaurants.");
    }

    #[test]
    fn licence_table_truncates_long_cells() {
        let long = Licence {
            name: "A Restaurant With A Very Long Name That Exceeds The Column".to_string(),
            address: "A Very Long Address That Also Exceeds The Address Column Width By Far"
                .to_string(),
            ..licence()
        };
        let out = licences(&[long]);
        assert!(out.contains('…'));
        assert!(out.contains("Central/Western"));
        assert!(out.contains("2027-01-01"));
    }

    #[test]
    fn openrice_table_reports_empty_results() {
        assert_eq!(openrice(&[]), "No OpenRice matches.");
    }

    #[test]
    fn openrice_table_formats_scores_and_open_state() {
        let closed = OpenRiceRestaurant {
            score: None,
            open_now: false,
            ..openrice_item()
        };
        let out = openrice(&[openrice_item(), closed]);
        assert!(out.contains("4.5"));
        assert!(out.contains("open"));
        assert!(out.contains("closed"));
        assert!(out.contains("$101-200"));
        assert!(out.contains("Tsim Sha Tsui"));
    }

    #[test]
    fn foodpanda_table_reports_empty_results() {
        assert_eq!(foodpanda(&[]), "No Foodpanda matches.");
    }

    #[test]
    fn foodpanda_table_formats_fees_eta_and_distance() {
        let out = foodpanda(&[foodpanda_item()]);
        assert!(out.contains("4.2"));
        assert!(out.contains("$50"));
        assert!(out.contains("$6"));
        assert!(out.contains("25m"));
        assert!(out.contains("1.23km"));
    }

    #[test]
    fn foodpanda_table_marks_missing_metrics_with_dashes() {
        let sparse = FoodpandaRestaurant {
            rating: None,
            minimum_order: None,
            delivery_fee: None,
            delivery_minutes: None,
            distance_km: None,
            ..foodpanda_item()
        };
        let out = foodpanda(&[sparse]);
        let row = out
            .lines()
            .find(|line| line.starts_with("Noodle House"))
            .expect("vendor row");
        assert!(row.contains(" - "), "{row}");
    }
}
