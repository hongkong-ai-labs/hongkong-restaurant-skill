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

**Agent skill** — installs `SKILL.md` and the binary bootstrap script:

```bash
npx skills add hongkong-ai-labs/hongkong-restaurant-skill
```

**Prebuilt binary** — macOS (Intel or Apple silicon) and Linux (x86_64 or
aarch64, static musl), checksum-verified, into `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/hongkong-ai-labs/hongkong-restaurant-skill/main/scripts/install.sh | sh
```

**From source** — Rust 1.89 or newer:

```bash
git clone https://github.com/hongkong-ai-labs/hongkong-restaurant-skill
cd hongkong-restaurant-skill
cargo install --path . --locked
# or: cargo build --release --locked   # target/release/hkrestaurant
```

Verify:

```console
$ hkrestaurant --version
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

## Run modes and deployment

| Mode | Start with | Use for |
| --- | --- | --- |
| CLI | `hkrestaurant <command>` | Interactive use and shell scripts |
| JSON | `<command>` with `--json` | `jq`, automation, other programs |
| MCP stdio | `hkrestaurant mcp` | Desktop and CLI MCP clients |
| Container | `docker run --rm -i hkrestaurant mcp` | No local Rust toolchain |

### MCP and authentication

This server is **stdio-only**: the client launches the process and speaks MCP
over stdin/stdout. There is no listening port, so there is no network
authentication layer and no API-key setting. Optional API keys in this project
family apply only to servers that expose an HTTP transport. Keep the process
local and let the MCP client manage it.

Typical client entry:

```json
{
  "mcpServers": {
    "hongkong-restaurant-skill": {
      "command": "hkrestaurant",
      "args": ["mcp"]
    }
  }
}
```

If the client does not inherit `PATH`, use the absolute installed path.

### Docker and Apple container

```bash
docker build -t hkrestaurant .
docker run --rm hkrestaurant --help
docker run --rm -i hkrestaurant mcp          # stdio MCP
docker compose run --rm stdio             # same, through Compose
```

Apple's `container` runtime uses the same OCI image:

```bash
container build -t hkrestaurant .
container run --rm hkrestaurant --help
container run --rm -i hkrestaurant mcp
```

The runtime image contains the release binary and CA certificates, runs as a
non-root user, and defaults to `--help` so a bare `docker run` is harmless.

### Cloudflare and WASM

There is no Worker or WASM deployment. The binary talks directly to upstream
HTTP APIs over TLS and uses a Tokio runtime; packaging it as a stdio server is
the supported integration. A Worker would need a separate HTTP/JavaScript
adapter and is out of scope for this repo.

### Releases

Pushing a `v*` tag runs the release workflow, which checks the tag against
`Cargo.toml` and `CHANGELOG.md`, builds four targets, smoke-tests native
binaries and publishes:

- `hkrestaurant-<tag>-<target>.tar.gz` for macOS and static Linux
- `SHA256SUMS` covering every tarball

`scripts/install.sh` downloads the latest release, verifies its checksum and
falls back to `cargo build --release --locked` when no release is available.
`just tag X.Y.Z` creates the annotated tag locally; `just release X.Y.Z` tags
and pushes to trigger the workflow.

## Development

```bash
just                 # fmt check, clippy with -D warnings, tests
just skill-check     # validate SKILL.md and shell syntax
just container       # build the image and print CLI help
cargo test --all --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

CI runs the same checks. Dependabot proposes weekly Cargo and GitHub Actions
updates, and the audit workflow checks the lockfile against the RustSec
advisory database.

## Licence

MIT. Data remains subject to FEHD, OpenRice and Foodpanda terms.
