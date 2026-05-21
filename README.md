# Fastly Object Storage uploader — Compute demo

A small **Fastly Compute** starter you can run locally or deploy as a demo. It shows how to accept file uploads at the edge and write them to **Fastly Object Storage (FOS)** using the S3-compatible API and **AWS Signature Version 4**.

Use it to:

- Demo uploads in a browser
- Copy working `curl` examples (hostname updates automatically)
- Fork the code as a base for your own upload feature

---

## What you get

| Feature | URL / method | Description |
|--------|----------------|-------------|
| **Demo website** | `GET /` | Upload form + live `curl` examples for the current host |
| **Upload API** | `POST` or `PUT /api/upload?key=<path>` | Raw body = object bytes; JSON response on success |

**Success response** (`200`):

```json
{
  "ok": true,
  "path": "s3://your-bucket/uploads/example.bin",
  "bytes_uploaded": 12345
}
```

**Flow (high level):**

```
Client (browser or curl)
    → Fastly Compute (this app)
        → signs request (AWS SigV4)
        → PUT to FOS: https://<region>.object.fastlystorage.app/<bucket>/<key>
```

---

## Before you start

You need:

1. A **Fastly account** and the [Fastly CLI](https://www.fastly.com/documentation/reference/tools/cli/) (`fastly`)
2. An **Object Storage bucket** and **access key** (read/write) — [create in the control panel](https://manage.fastly.com/resources/object-storage) or see [Working with Object Storage](https://www.fastly.com/documentation/guides/platform/object-storage/working-with-object-storage/)
3. **Rust via rustup** with the `wasm32-wasip1` target (required for Compute)

```bash
rustup target add wasm32-wasip1
```

If `fastly compute build` fails with *can't find crate for `core`*, Homebrew `rustc` is probably first on your `PATH`. Put rustup first:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

This repo pins Rust **1.90** in `rust-toolchain.toml`.

---

## Configure the demo (2 places)

### 1. `src/config.rs` — bucket, region, keys

```rust
pub const FOS: FosConfig = FosConfig {
    bucket: "your-bucket",
    region: "eu-central",           // FOS region name (not AWS)
    access_key: "YOUR_ACCESS_KEY",
    secret_key: "YOUR_SECRET_KEY",
    backend: "storage",             // must match fastly.toml backend name
};
```

### 2. `fastly.toml` — local backend URL

Under `[local_server.backends.storage]`, set the regional **object** hostname (bucket goes in the path, not the host):

```toml
[local_server.backends.storage]
  url = "https://eu-central.object.fastlystorage.app"
```

| Region (examples) | Host |
|-------------------|------|
| `eu-central` | `eu-central.object.fastlystorage.app` |
| `us-east` | `us-east.object.fastlystorage.app` |
| `us-west` | `us-west.object.fastlystorage.app` |

Full list: [FOS S3-compatible API](https://www.fastly.com/documentation/guides/platform/object-storage/working-with-object-storage/#using-the-s3-compatible-api).

`region` in `config.rs` and the host in `fastly.toml` must match.

---

## Run locally

```bash
fastly compute build
fastly compute serve
```

Then:

### Option A — Browser (easiest)

Open **[http://127.0.0.1:7676/](http://127.0.0.1:7676/)**

- Choose a file and optional key prefix (e.g. `uploads/`)
- Click **Upload**
- Scroll to **API examples (curl)** — commands use `http://127.0.0.1:7676` automatically

### Option B — curl

```bash
echo "hello demo" > /tmp/example.bin

curl -i -X POST "http://127.0.0.1:7676/api/upload?key=uploads/example.bin" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @/tmp/example.bin
```

`PUT` works the same; only change `-X PUT`.

### Verify in FOS (optional)

```bash
aws s3 ls s3://your-bucket/uploads/ \
  --endpoint-url https://eu-central.object.fastlystorage.app \
  --region eu-central
```

(Configure AWS CLI for FOS first if needed: [AWS CLI for Fastly Object Storage](https://www.fastly.com/documentation/guides/platform/object-storage/aws-cli-for-fastly-object-storage/).)

---

## Deploy as a demo

1. Link or create a Compute service: `fastly compute deploy` (follow CLI prompts).
2. Attach a **domain** to the service in the Fastly control panel.
3. Open `https://your-domain/` — the demo page and `curl` examples will use **your domain** instead of `127.0.0.1:7676`.

Hostname logic: `Host` header + `https` for real domains, `http` for `localhost` / `127.0.0.1`.

---

## Project layout (where to change things)

| File | Change when you want to… |
|------|---------------------------|
| `src/config.rs` | Bucket, region, credentials, backend name |
| `fastly.toml` | Local FOS backend URL, build command |
| `src/fos.rs` | SigV4 signing and upload to FOS |
| `src/html.rs` | Demo page look, copy, example paths |
| `src/main.rs` | Routes (`/` vs `/api/upload`) |

---

## API reference (demo)

**Upload**

- **Methods:** `POST`, `PUT`
- **Path:** `/api/upload`
- **Query:** `key` — object key inside the bucket (e.g. `uploads/photo.jpg`)
- **Body:** raw file bytes
- **Headers:** `Content-Type` optional; `Content-Length` recommended

**Errors**

| Status | Meaning |
|--------|---------|
| `405` | Wrong HTTP method (e.g. `GET` on `/api/upload`) |
| `404` | Unknown `GET` path |
| `502` | FOS rejected the signed `PUT` (check keys, region, bucket, endpoint) |

---

## Customize for your own demo

1. Fork or clone this repo.
2. Update `src/config.rs` and `fastly.toml`.
3. Tweak `src/html.rs` (title, colors, default prefix).
4. Add auth on `/api/upload` before going public.
5. Move secrets to [Fastly Secret Store](https://www.fastly.com/documentation/guides/compute/secrets/) — do not ship real keys in source.

---

## Production checklist

- [ ] Credentials in Secret Store, not in `config.rs`
- [ ] Keys rotated if they were ever committed or shared
- [ ] Auth (token, API key, or CDN ACL) on `/api/upload`
- [ ] Region/host in config matches your bucket’s region
- [ ] Domain + TLS configured on the Compute service

---

## Troubleshooting

| Problem | What to try |
|---------|-------------|
| `can't find crate for core` / `wasm32-wasip1` | `rustup target add wasm32-wasip1` and `export PATH="$HOME/.cargo/bin:$PATH"` |
| `502` from upload | Wrong endpoint host (`*.object.fastlystorage.app`), wrong region, or invalid keys |
| `InvalidRequest` from AWS CLI | Region must be FOS region name (e.g. `eu-central`), not `eu-central-1` |
| Browser works, curl fails | Use full URL with same host as the page; check `key=` query param |

---

## Learn more

- [Working with Object Storage](https://www.fastly.com/documentation/guides/platform/object-storage/working-with-object-storage/)
- [Compute Rust SDK](https://docs.rs/fastly/latest/fastly/)
- [fastly.toml reference](https://www.fastly.com/documentation/reference/compute/fastly-toml)

## Security

See [SECURITY.md](SECURITY.md) for reporting security issues.
