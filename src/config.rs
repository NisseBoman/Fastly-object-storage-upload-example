//! Edit these values when using this project as a starter template.

/// Fastly Object Storage settings for the demo uploader.
pub struct FosConfig {
    /// Bucket name (included in the signed request path).
    pub bucket: &'static str,
    /// FOS region name used in SigV4 (e.g. `eu-central`).
    pub region: &'static str,
    /// S3-compatible access key ID.
    pub access_key: &'static str,
    /// S3-compatible secret key.
    pub secret_key: &'static str,
    /// Compute backend name in `fastly.toml` (`[local_server.backends]`).
    pub backend: &'static str,
}

/// Default demo configuration — replace before production use.
pub const FOS: FosConfig = FosConfig {
    bucket: "<bucketname>",
    region: "eu-central",
    access_key: "<access_key>",
    secret_key: "<secret_key>",
    backend: "storage",
};

impl FosConfig {
    pub fn endpoint_host(&self) -> String {
        format!("{}.object.fastlystorage.app", self.region)
    }
}
