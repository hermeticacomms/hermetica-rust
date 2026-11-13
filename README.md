# Hermetica Rust client

Generated from `docs/api/openapi.json` by `node scripts/gen-sdks.mjs` (2524 operations). Async (tokio), reqwest with rustls.

```toml
[dependencies]
hermeticacomms = "0.4.0"
```

```rust
use hermetica::{Client, Params};
use serde_json::json;

let hm = Client::new("hm_sk_...", "https://api.example.com");
let page = hm.get_v1_profiles(Params::new().query("limit", 25)).await?;
hm.post_v1_events(Params::new()
    .body(json!({ "name": "signed_up", "profile": { "email": "ada@example.com" } }))
    .idempotency_key("signup-ada-1")).await?;
let all = hm.list_all("GET", "/v1/profiles", Params::new()).await?; // follows nextCursor
```

- One method per operationId (snake_case). Results are `serde_json::Value`; errors are `hermetica::Error` with status, code, message, details
  (status 0 for a network error).
- Every non-GET call sends an `Idempotency-Key` (random unless you set one); reuse yours when you retry a call yourself.
- Network errors, 429 and 5xx are retried with exponential backoff and jitter, honouring a `Retry-After` in seconds. Every attempt sends
  the same `Idempotency-Key`, so a retried write never runs twice. `max_retries` is 3; set it to 0 to turn retries off.
- Consent and suppression are enforced by the API; the client cannot bypass them.
- Regenerate after the API changes: `node scripts/gen-sdks.mjs`.
