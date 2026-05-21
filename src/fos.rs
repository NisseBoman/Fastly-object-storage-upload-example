use crate::config::FosConfig;
use fastly::http::header::CONTENT_LENGTH;
use fastly::http::StatusCode;
use fastly::{Error, Request, Response};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use time::format_description;
use time::OffsetDateTime;

type HmacSha256 = Hmac<Sha256>;

fn hash_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn hmac_sha256(key: &[u8], data: &str) -> Result<Vec<u8>, Error> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|e| Error::msg(e.to_string()))?;
    mac.update(data.as_bytes());
    Ok(mac.finalize().into_bytes().to_vec())
}

fn signing_key(secret: &str, datestamp: &str, region: &str) -> Result<Vec<u8>, Error> {
    let k_date = hmac_sha256(format!("AWS4{}", secret).as_bytes(), datestamp)?;
    let k_region = hmac_sha256(&k_date, region)?;
    let k_service = hmac_sha256(&k_region, "s3")?;
    hmac_sha256(&k_service, "aws4_request")
}

fn hex_hmac(key: &[u8], data: &str) -> Result<String, Error> {
    Ok(hex::encode(hmac_sha256(key, data)?))
}

/// Resolve object key from `?key=` or from the URL path (after `/api/upload/`).
pub fn object_key_from_request(req: &Request) -> String {
    if let Some(key) = req.get_query_parameter("key") {
        if !key.is_empty() {
            return key.to_string();
        }
    }

    let path = req.get_path();
    let stripped = path
        .strip_prefix("/api/upload/")
        .or_else(|| path.strip_prefix("/api/upload"))
        .unwrap_or(path)
        .trim_start_matches('/');

    if !stripped.is_empty() {
        return stripped.to_string();
    }

    "upload.bin".to_string()
}

/// Stream the request body to FOS with AWS SigV4 and return JSON on success.
pub fn upload_to_fos(config: &FosConfig, mut req: Request) -> Result<Response, Error> {
    let object_key = object_key_from_request(&req);
    let body = req.take_body();
    let payload_hash = "UNSIGNED-PAYLOAD";

    let now = OffsetDateTime::now_utc();
    let amz_date_fmt = format_description::parse("[year][month][day]T[hour][minute][second]Z")
        .map_err(|e| Error::msg(e.to_string()))?;
    let datestamp_fmt =
        format_description::parse("[year][month][day]").map_err(|e| Error::msg(e.to_string()))?;
    let amz_date = now
        .format(&amz_date_fmt)
        .map_err(|e| Error::msg(e.to_string()))?;
    let datestamp = now
        .format(&datestamp_fmt)
        .map_err(|e| Error::msg(e.to_string()))?;

    let host = config.endpoint_host();
    let canonical_uri = format!("/{}/{}", config.bucket, object_key);
    let endpoint = format!("https://{}{}", host, canonical_uri);

    let canonical_headers = format!(
        "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
        host, payload_hash, amz_date
    );
    let signed_headers = "host;x-amz-content-sha256;x-amz-date";
    let canonical_request = format!(
        "PUT\n{}\n\n{}\n{}\n{}",
        canonical_uri, canonical_headers, signed_headers, payload_hash
    );
    let canonical_request_hash = hash_hex(canonical_request.as_bytes());

    let credential_scope = format!("{}/{}/s3/aws4_request", datestamp, config.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date, credential_scope, canonical_request_hash
    );
    let key = signing_key(config.secret_key, &datestamp, config.region)?;
    let signature = hex_hmac(&key, &string_to_sign)?;

    let auth_header = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        config.access_key, credential_scope, signed_headers, signature
    );

    let mut upstream = Request::put(endpoint)
        .with_header("host", &host)
        .with_header("x-amz-date", &amz_date)
        .with_header("x-amz-content-sha256", payload_hash)
        .with_header("authorization", auth_header)
        .with_body(body);

    if let Some(content_type) = req.get_header_str("content-type") {
        upstream = upstream.with_header("content-type", content_type);
    }

    let mut bytes_uploaded: Option<u64> = None;
    if let Ok(content_length) = u64::from_str(req.get_header_str(CONTENT_LENGTH).unwrap_or("")) {
        bytes_uploaded = Some(content_length);
        upstream = upstream.with_header(CONTENT_LENGTH, content_length.to_string());
    }

    let fos_resp = upstream.send(config.backend)?;
    if !fos_resp.get_status().is_success() {
        let status = fos_resp.get_status();
        let body = fos_resp.into_body_str();
        return Ok(Response::from_status(StatusCode::BAD_GATEWAY)
            .with_content_type(fastly::mime::TEXT_PLAIN_UTF_8)
            .with_body(format!("FOS upload failed: {} {}", status, body)));
    }

    let bytes_uploaded_json = bytes_uploaded
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());
    let json = format!(
        "{{\"ok\":true,\"path\":\"s3://{}/{}\",\"bytes_uploaded\":{}}}",
        config.bucket, object_key, bytes_uploaded_json
    );

    Ok(Response::from_status(StatusCode::OK)
        .with_content_type(fastly::mime::APPLICATION_JSON)
        .with_body(json))
}
