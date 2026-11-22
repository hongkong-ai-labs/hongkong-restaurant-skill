//! FEHD restaurant-licence XML.

use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::api::{Client, Licence, LicenceReport};

const URL: &str = "https://www.fehd.gov.hk/english/licensing/license/text/LP_Restaurants_EN.XML";

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

pub async fn licences(client: &Client) -> Result<LicenceReport> {
    let body = client
        .get_text("fehd", URL, std::time::Duration::from_secs(6 * 3600))
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
}
