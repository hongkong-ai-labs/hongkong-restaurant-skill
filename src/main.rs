//! `hkrestaurant` — Hong Kong restaurant licences, OpenRice and Foodpanda.

mod api;
mod mcp;
mod render;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use serde_json::json;

use api::Client;

#[derive(Parser)]
#[command(
    name = "hkrestaurant",
    version,
    about = "Hong Kong restaurant licences, OpenRice search and Foodpanda listings — CLI and MCP server",
    arg_required_else_help = true
)]
struct Cli {
    /// Print machine-readable JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Search the official FEHD licensed-restaurant list
    #[command(alias = "licence")]
    Licences {
        /// Restaurant name or address substring
        #[arg(long, short)]
        query: Option<String>,
        /// District substring, e.g. "Central", "Yau Tsim", "Islands"
        #[arg(long, short)]
        district: Option<String>,
        /// Licence type substring or code: General, Light Refreshment, Marine, RL, RR, MR
        #[arg(long, short, value_name = "TYPE")]
        kind: Option<String>,
        /// Maximum rows (default 50; 0 means all)
        #[arg(long, short, default_value_t = 50)]
        limit: usize,
    },
    /// Search OpenRice Hong Kong
    #[command(alias = "or")]
    Openrice {
        /// What to search for: cuisine, dish or restaurant name
        query: String,
        /// Maximum rows (default 15, maximum 150)
        #[arg(long, short, default_value_t = 15)]
        limit: usize,
    },
    /// List Foodpanda vendors delivering near a coordinate
    #[command(alias = "fp")]
    Foodpanda {
        /// Latitude, e.g. 22.2793
        #[arg(long)]
        lat: f64,
        /// Longitude, e.g. 114.1628
        #[arg(long)]
        lng: f64,
        /// Filter the nearby list by restaurant name or cuisine
        #[arg(long, short)]
        query: Option<String>,
        /// Maximum rows (default 24, maximum 48)
        #[arg(long, short, default_value_t = 24)]
        limit: usize,
    },
    /// Describe source coverage and known limitations
    Sources,
    /// Run the MCP server over stdio
    Mcp,
}

fn source_report() -> serde_json::Value {
    json!({
        "sources": [
            {
                "id": "fehd",
                "name": "FEHD restaurant licences",
                "kind": "official open data",
                "coverage": "Every licensed General, Light Refreshment and Marine restaurant in Hong Kong",
                "fields": ["name", "address", "district", "licence type", "licence number", "expiry date"],
                "auth": "none"
            },
            {
                "id": "openrice",
                "name": "OpenRice search",
                "kind": "undocumented public website endpoint",
                "coverage": "Editorial search results, 15 per page",
                "fields": ["name", "address", "district", "categories", "score", "reviews", "price", "open now", "url"],
                "auth": "none",
                "caution": "Unofficial endpoint; can change or rate-limit without notice. Subject to OpenRice terms."
            },
            {
                "id": "foodpanda",
                "name": "Foodpanda nearby vendors",
                "kind": "undocumented Delivery Hero listing endpoint",
                "coverage": "Vendors delivering near a coordinate; keyword filtering is client-side",
                "fields": ["name", "address", "cuisines", "rating", "reviews", "minimum order", "delivery fee", "ETA", "distance", "url"],
                "auth": "none",
                "caution": "Unofficial endpoint; availability depends on delivery coverage. Subject to Foodpanda terms."
            },
            {
                "id": "keeta",
                "name": "Keeta",
                "kind": "not supported",
                "coverage": "No public consumer search API; the OpenAPI is merchant-only and requires credentials",
                "auth": "merchant credentials required",
                "caution": "Not scraped by this skill. Use the Keeta app for consumer search."
            }
        ]
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new()?;

    if let Command::Mcp = cli.command {
        return mcp::serve(client).await;
    }

    match cli.command {
        Command::Licences {
            query,
            district,
            kind,
            limit,
        } => {
            let mut report = client.licences().await?;
            api::filter_licences(
                &mut report,
                query.as_deref(),
                district.as_deref(),
                kind.as_deref(),
            );
            let total = report.restaurants.len();
            if limit > 0 {
                report.restaurants.truncate(limit);
                report.count = report.restaurants.len();
            }
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", render::licences(&report.restaurants));
                println!(
                    "\n{} of {} licensed restaurants; FEHD list generated {}",
                    report.count, total, report.generation_date
                );
            }
        }
        Command::Openrice { query, limit } => {
            if query.trim().is_empty() {
                bail!("query must not be empty");
            }
            let restaurants = client.openrice(&query, limit).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&restaurants)?);
            } else {
                println!("{}", render::openrice(&restaurants));
            }
        }
        Command::Foodpanda {
            lat,
            lng,
            query,
            limit,
        } => {
            if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
                bail!("lat must be -90..90 and lng must be -180..180");
            }
            let restaurants = client.foodpanda(lat, lng, query.as_deref(), limit).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&restaurants)?);
            } else {
                println!("{}", render::foodpanda(&restaurants));
            }
        }
        Command::Sources => {
            let report = source_report();
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                for source in report["sources"].as_array().into_iter().flatten() {
                    println!(
                        "{} — {}\n  {}\n  caution: {}\n",
                        source["id"].as_str().unwrap_or_default(),
                        source["name"].as_str().unwrap_or_default(),
                        source["coverage"].as_str().unwrap_or_default(),
                        source["caution"].as_str().unwrap_or_default()
                    );
                }
            }
        }
        Command::Mcp => unreachable!(),
    }
    Ok(())
}
