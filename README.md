# hkrestaurant

Hong Kong restaurant lookup as a Rust CLI and MCP server.

Three sources:

- **FEHD** — official licensed-restaurant list (name, address, district,
  licence type and expiry)
- **OpenRice** — editorial search with scores, reviews, price bands and open
  status
- **Foodpanda** — nearby delivery vendors with minimum order, fee and ETA

Keeta is **not** supported: it has no public consumer API, only a credentialed
merchant API. This project does not scrape the app.

## Install

```bash
npx skills add hongkong-ai-labs/hongkong-restaurant-skill
cargo install --path . --locked
sh scripts/install.sh
```

## CLI

```bash
hkrestaurant licences --query "jade" --limit 5
hkrestaurant licences --district "Yau Tsim" --kind General
hkrestaurant openrice sushi --limit 5
hkrestaurant foodpanda --lat 22.2793 --lng 114.1628
hkrestaurant foodpanda --lat 22.2793 --lng 114.1628 --query sushi
hkrestaurant sources
hkrestaurant mcp
```

## MCP

```json
{
  "mcpServers": {
    "hongkong-restaurant": { "command": "hkrestaurant", "args": ["mcp"] }
  }
}
```

Tools: `search_restaurant_licences`, `search_openrice`,
`list_foodpanda_vendors`, `list_restaurant_sources`.

## Limits

OpenRice and Foodpanda are undocumented public endpoints, not partner APIs.
They can change or rate-limit without notice and remain subject to their terms.
Foodpanda keyword filtering is client-side over the nearby list. FEHD licences
do not include hygiene grades, menus or opening hours.

## Licence

MIT. Data remains subject to FEHD, OpenRice and Foodpanda terms.
