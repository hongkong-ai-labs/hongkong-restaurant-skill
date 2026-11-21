---
name: hongkong-restaurant-skill
description: Hong Kong restaurant lookup across the official FEHD licensed-restaurant list, OpenRice search and Foodpanda delivery listings. Use for restaurant licences, addresses, districts, cuisine search, ratings, opening status, delivery fees or nearby Foodpanda vendors. Keeta is not covered because it has no public consumer API.
license: MIT
---

# Hong Kong restaurants

`hkrestaurant` is a Rust CLI and MCP server that combines three restaurant
sources. It needs no API key. It is an independent, unofficial project.

| Source | What it is | Status |
| --- | --- | --- |
| **FEHD** | Official list of every licensed General, Light Refreshment and Marine restaurant | stable open data |
| **OpenRice** | Editorial search results from the OpenRice Hong Kong website | undocumented public endpoint |
| **Foodpanda** | Vendors delivering near a coordinate | undocumented public endpoint |

**Keeta is deliberately not supported.** Keeta has no public consumer search
API. Its documented OpenAPI is merchant-only and needs credentials. This skill
does not scrape the app or bypass that boundary. Say so if the user asks for
Keeta data, and use the Keeta app instead.

## Before first use

```console
$ bash scripts/ensure-binary.sh
/Users/you/.local/bin/hkrestaurant
```

```bash
BIN="$(bash scripts/ensure-binary.sh)"
```

It checks `$HKRESTAURANT_BIN`, `hkrestaurant` on `PATH`,
`scripts/install.sh`, then `cargo build --release --locked` in the repo root.

## CLI

```bash
# Official licences (best for "is this restaurant licensed?" and addresses)
$BIN licences --query "jade" --limit 5
$BIN licences --district "Yau Tsim" --kind "General"
$BIN licences --query "Causeway Bay" --json

# OpenRice editorial search (best for cuisine / dish discovery)
$BIN openrice sushi --limit 5
$BIN openrice "dim sum" --json

# Foodpanda nearby delivery (best for "what can deliver here?")
$BIN foodpanda --lat 22.2793 --lng 114.1628
$BIN foodpanda --lat 22.2793 --lng 114.1628 --query sushi
$BIN foodpanda --lat 22.2793 --lng 114.1628 --limit 48 --json

# What each source covers
$BIN sources
$BIN mcp
```

### Trade-offs

- **Licences** are authoritative for existence, name, address, district,
  licence type and expiry. They do not carry ratings, menus, opening hours or
  delivery.
- **OpenRice** gives scores, review counts, price bands, categories and a
  current open/closed flag. It returns 15 results per page; `--limit` fetches
  more pages up to 150.
- **Foodpanda** gives rating, minimum order, delivery fee, ETA and distance for
  vendors delivering to a coordinate. The public listing has no reliable
  keyword parameter, so `--query` filters the nearby list client-side. A query
  can therefore miss a restaurant that is further away.
- A restaurant can appear in one source and not another. Absence from OpenRice
  or Foodpanda is not evidence that it is closed.

## JSON fields

`licences` returns
`{generation_date, count, restaurants:[{licence_no,name,address,district,licence_type,licence_type_code,expiry_date,info}]}`.
`generation_date` is the day FEHD generated the list, not the time of your call.

`openrice` returns an array of
`{name,name_other_lang,address,address_other_lang,district,categories,score,review_count,price,open_now,url}`.
`score` is OpenRice's 0-5 overall score, not a hygiene rating. `open_now` is
the website's current flag.

`foodpanda` returns an array of
`{code,name,address,cuisines,rating,review_count,budget,minimum_order,delivery_fee,delivery_minutes,distance_km,url}`.
Fees and minimum orders are in HKD and can change at checkout.

## MCP

```json
{
  "mcpServers": {
    "hongkong-restaurant": {
      "command": "/absolute/path/to/hkrestaurant",
      "args": ["mcp"]
    }
  }
}
```

Tools:

| Tool | Arguments |
| --- | --- |
| `search_restaurant_licences` | `{ query?, district?, type?, limit? }` |
| `search_openrice` | `{ query, limit? }` |
| `list_foodpanda_vendors` | `{ lat, lng, query?, limit? }` |
| `list_restaurant_sources` | `{}` |

## Answering patterns

- "Is X licensed?" → `licences --query X`; quote the licence number, type and
  expiry, and say if there is no match.
- "Where is X?" → `licences --query X` for the official address, not a delivery
  platform's pin.
- "Good sushi in Causeway Bay?" → `openrice sushi`, then filter the district in
  the results. Mention scores and review counts, and that they are OpenRice's.
- "What can deliver to me?" → `foodpanda --lat ... --lng ...`; quote minimum
  order, fee and ETA together, since a cheap fee with a high minimum is not
  cheap.
- "What about Keeta?" → explain that Keeta has no public consumer API and this
  skill does not scrape it.

## Cautions

- OpenRice and Foodpanda endpoints are undocumented consumer endpoints. They
  can change, rate-limit, or require a browser session without notice. The
  skill sends a normal browser User-Agent and does not log in.
- Do not bulk-harvest either platform. Use small `--limit` values and keep the
  cache on (5 minutes for OpenRice, 2 minutes for Foodpanda, 6 hours for FEHD).
- Ratings are platform-specific and not comparable across OpenRice and
  Foodpanda.
- Prices, fees, availability and opening status are a snapshot, not a quote.
- FEHD licence data lists licensed premises, not a food-safety grade. A valid
  licence is not a hygiene rating.
- Do not present this as an official FEHD, OpenRice, Foodpanda or Keeta
  channel. Follow each platform's terms when redistributing its data.

## Sources

- FEHD: `https://www.fehd.gov.hk/english/licensing/license/text/LP_Restaurants_EN.XML`
- OpenRice: `https://www.openrice.com/api/pois?...`
- Foodpanda: `https://disco.deliveryhero.io/listing/api/v1/pandora/vendors?...`
  with `x-disco-client-id: web`
