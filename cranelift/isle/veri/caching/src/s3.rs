//! A minimal, blocking Amazon S3 client implementing [`ObjectStore`].
//!
//! This speaks just the four S3 operations the cache needs — `ListObjectsV2`,
//! `GetObject`, `PutObject` and `DeleteObject` — over HTTPS, signing requests
//! with AWS Signature Version 4 when credentials are available and sending
//! anonymous requests (for public-read buckets) otherwise.

use std::fmt::Write as _;
use std::io;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::store::ObjectStore;

/// AWS credentials for signing requests.
#[derive(Clone)]
pub struct Credentials {
    access_key_id: String,
    secret_access_key: String,
    /// Session token for temporary credentials (e.g. from an assumed role).
    session_token: Option<String>,
}

impl Credentials {
    /// Read credentials from the standard `AWS_ACCESS_KEY_ID`,
    /// `AWS_SECRET_ACCESS_KEY` and (optional) `AWS_SESSION_TOKEN` environment
    /// variables. Returns `None` if the key ID or secret is unset or empty.
    pub fn from_env() -> Option<Credentials> {
        let var = |name| std::env::var(name).ok().filter(|v: &String| !v.is_empty());
        Some(Credentials {
            access_key_id: var("AWS_ACCESS_KEY_ID")?,
            secret_access_key: var("AWS_SECRET_ACCESS_KEY")?,
            session_token: var("AWS_SESSION_TOKEN"),
        })
    }
}

/// An S3 bucket (and key prefix within it) used as an [`ObjectStore`].
pub struct S3Store {
    agent: ureq::Agent,
    bucket: String,
    region: String,
    /// Prefix prepended to every object name; empty or ending in `/`.
    prefix: String,
    /// Credentials for signing; `None` sends anonymous requests.
    credentials: Option<Credentials>,
}

/// Number of attempts for requests that fail transiently.
const ATTEMPTS: u32 = 4;

impl S3Store {
    /// Create a store for objects under `prefix` in `bucket`.
    ///
    /// The region is taken from `AWS_REGION` or `AWS_DEFAULT_REGION`,
    /// defaulting to `us-east-1`.
    pub fn new(bucket: &str, prefix: &str, credentials: Option<Credentials>) -> S3Store {
        let region = std::env::var("AWS_REGION")
            .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
            .ok()
            .filter(|r| !r.is_empty())
            .unwrap_or_else(|| "us-east-1".to_string());
        let mut prefix = prefix.trim_start_matches('/').to_string();
        if !prefix.is_empty() && !prefix.ends_with('/') {
            prefix.push('/');
        }
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(120)))
            .build();
        S3Store {
            agent: ureq::Agent::new_with_config(config),
            bucket: bucket.to_string(),
            region,
            prefix,
            credentials,
        }
    }

    /// Whether requests are signed (as opposed to anonymous).
    pub fn has_credentials(&self) -> bool {
        self.credentials.is_some()
    }

    fn host(&self) -> String {
        format!("{}.s3.{}.amazonaws.com", self.bucket, self.region)
    }

    /// Issue a request, retrying transient failures. `key` is the full object
    /// key (or `None` for bucket-level requests such as listing). Returns the
    /// status code and response body.
    fn request(
        &self,
        method: &str,
        key: Option<&str>,
        query: &[(&str, &str)],
        body: &[u8],
    ) -> io::Result<(u16, Vec<u8>)> {
        let mut last_err = None;
        for attempt in 0..ATTEMPTS {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(250 << attempt));
            }
            match self.request_once(method, key, query, body) {
                Ok((status, body)) if status >= 500 || status == 429 => {
                    log::warn!("S3 {method} {key:?}: HTTP {status}, retrying");
                    last_err = Some(status_error(method, key, status, &body));
                }
                Ok(result) => return Ok(result),
                Err(e) => {
                    log::warn!("S3 {method} {key:?}: {e}, retrying");
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap())
    }

    fn request_once(
        &self,
        method: &str,
        key: Option<&str>,
        query: &[(&str, &str)],
        body: &[u8],
    ) -> io::Result<(u16, Vec<u8>)> {
        let host = self.host();
        let canonical_uri = format!("/{}", uri_encode(key.unwrap_or(""), false));
        let mut query: Vec<(String, String)> = query
            .iter()
            .map(|(k, v)| (uri_encode(k, true), uri_encode(v, true)))
            .collect();
        query.sort();
        let canonical_query = query
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");

        let mut url = format!("https://{host}{canonical_uri}");
        if !canonical_query.is_empty() {
            url.push('?');
            url.push_str(&canonical_query);
        }

        let headers = match &self.credentials {
            Some(creds) => sign(
                creds,
                &self.region,
                method,
                &host,
                &canonical_uri,
                &canonical_query,
                body,
                SystemTime::now(),
            ),
            None => vec![],
        };

        macro_rules! with_headers {
            ($req:expr) => {{
                let mut req = $req;
                for (name, value) in &headers {
                    req = req.header(name.as_str(), value.as_str());
                }
                req
            }};
        }
        let response = match method {
            "GET" => with_headers!(self.agent.get(&url)).call(),
            "DELETE" => with_headers!(self.agent.delete(&url)).call(),
            "PUT" => with_headers!(self.agent.put(&url)).send(body),
            _ => unreachable!("unsupported method {method}"),
        };
        let mut response = response.map_err(io::Error::other)?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .with_config()
            .limit(1 << 30)
            .read_to_vec()
            .map_err(io::Error::other)?;
        Ok((status, body))
    }
}

impl ObjectStore for S3Store {
    fn list(&self, prefix: &str) -> io::Result<Vec<String>> {
        let full_prefix = format!("{}{prefix}", self.prefix);
        let mut names = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let mut query = vec![("list-type", "2"), ("prefix", full_prefix.as_str())];
            if let Some(token) = &token {
                query.push(("continuation-token", token.as_str()));
            }
            let (status, body) = self.request("GET", None, &query, &[])?;
            if status != 200 {
                return Err(status_error("LIST", Some(&full_prefix), status, &body));
            }
            let xml = String::from_utf8_lossy(&body);
            for key in xml_elements(&xml, "Key") {
                if let Some(name) = key.strip_prefix(&self.prefix) {
                    names.push(name.to_string());
                }
            }
            let truncated = xml_elements(&xml, "IsTruncated")
                .first()
                .map(String::as_str)
                == Some("true");
            token = xml_elements(&xml, "NextContinuationToken")
                .into_iter()
                .next();
            if !truncated || token.is_none() {
                return Ok(names);
            }
        }
    }

    fn get(&self, name: &str) -> io::Result<Option<Vec<u8>>> {
        let key = format!("{}{name}", self.prefix);
        match self.request("GET", Some(&key), &[], &[])? {
            (200, body) => Ok(Some(body)),
            (404, _) => Ok(None),
            (status, body) => Err(status_error("GET", Some(&key), status, &body)),
        }
    }

    fn put(&self, name: &str, body: &[u8]) -> io::Result<()> {
        let key = format!("{}{name}", self.prefix);
        match self.request("PUT", Some(&key), &[], body)? {
            (200, _) => Ok(()),
            (status, body) => Err(status_error("PUT", Some(&key), status, &body)),
        }
    }

    fn delete(&self, name: &str) -> io::Result<()> {
        let key = format!("{}{name}", self.prefix);
        match self.request("DELETE", Some(&key), &[], &[])? {
            (200 | 204 | 404, _) => Ok(()),
            (status, body) => Err(status_error("DELETE", Some(&key), status, &body)),
        }
    }

    fn describe(&self) -> String {
        format!(
            "s3://{}/{} ({}, {})",
            self.bucket,
            self.prefix,
            self.region,
            if self.has_credentials() {
                "signed"
            } else {
                "anonymous"
            }
        )
    }
}

fn status_error(op: &str, key: Option<&str>, status: u16, body: &[u8]) -> io::Error {
    let body = String::from_utf8_lossy(body);
    let code = xml_elements(&body, "Code").into_iter().next();
    let message = xml_elements(&body, "Message").into_iter().next();
    io::Error::other(format!(
        "S3 {op} {}: HTTP {status}{}{}",
        key.unwrap_or("(bucket)"),
        code.map(|c| format!(" {c}")).unwrap_or_default(),
        message.map(|m| format!(": {m}")).unwrap_or_default(),
    ))
}

/// Compute the SigV4 headers (`x-amz-date`, `x-amz-content-sha256`, optional
/// `x-amz-security-token`, and `authorization`) for a request.
///
/// See <https://docs.aws.amazon.com/AmazonS3/latest/API/sig-v4-header-based-auth.html>.
fn sign(
    creds: &Credentials,
    region: &str,
    method: &str,
    host: &str,
    canonical_uri: &str,
    canonical_query: &str,
    body: &[u8],
    now: SystemTime,
) -> Vec<(String, String)> {
    let (date, amz_date) = amz_dates(now);
    let payload_hash = hex(&Sha256::digest(body));

    // Canonical headers must be sorted by (lowercase) name.
    let mut signed: Vec<(&str, String)> = vec![
        ("host", host.to_string()),
        ("x-amz-content-sha256", payload_hash.clone()),
        ("x-amz-date", amz_date.clone()),
    ];
    if let Some(token) = &creds.session_token {
        signed.push(("x-amz-security-token", token.clone()));
    }
    let canonical_headers: String = signed.iter().map(|(k, v)| format!("{k}:{v}\n")).collect();
    let signed_headers = signed.iter().map(|(k, _)| *k).collect::<Vec<_>>().join(";");

    let canonical_request = format!(
        "{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );
    let scope = format!("{date}/{region}/s3/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        hex(&Sha256::digest(canonical_request.as_bytes()))
    );

    let k_date = hmac(
        format!("AWS4{}", creds.secret_access_key).as_bytes(),
        date.as_bytes(),
    );
    let k_region = hmac(&k_date, region.as_bytes());
    let k_service = hmac(&k_region, b"s3");
    let k_signing = hmac(&k_service, b"aws4_request");
    let signature = hex(&hmac(&k_signing, string_to_sign.as_bytes()));

    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
        creds.access_key_id
    );

    // `host` is set by the HTTP client from the URL.
    let mut headers: Vec<(String, String)> = signed
        .into_iter()
        .filter(|(k, _)| *k != "host")
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    headers.push(("authorization".to_string(), authorization));
    headers
}

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}

/// Format `now` as SigV4's `YYYYMMDD` date and `YYYYMMDDTHHMMSSZ` timestamp.
fn amz_dates(now: SystemTime) -> (String, String) {
    let secs = now.duration_since(UNIX_EPOCH).unwrap().as_secs();
    let (days, rem) = (secs / 86400, secs % 86400);
    // Civil-from-days, from Howard Hinnant's date algorithms.
    let z = days as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let date = format!("{year:04}{month:02}{day:02}");
    let time = format!("{:02}{:02}{:02}", rem / 3600, (rem % 3600) / 60, rem % 60);
    (date.clone(), format!("{date}T{time}Z"))
}

/// URI-encode per SigV4 rules: everything except unreserved characters is
/// percent-encoded, and `/` is left alone unless `encode_slash`.
fn uri_encode(s: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b'/' if !encode_slash => out.push('/'),
            _ => write!(out, "%{b:02X}").unwrap(),
        }
    }
    out
}

/// Extract the text of every `<tag>...</tag>` element in `xml`. This is not a
/// general XML parser, but S3's list and error responses are flat enough
/// that it suffices.
fn xml_elements(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find(&open) {
        rest = &rest[start + open.len()..];
        let Some(end) = rest.find(&close) else { break };
        out.push(
            rest[..end]
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&apos;", "'")
                .replace("&amp;", "&"),
        );
        rest = &rest[end + close.len()..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The worked `ListObjects` example from the SigV4 documentation:
    /// <https://docs.aws.amazon.com/AmazonS3/latest/API/sig-v4-header-based-auth.html>.
    #[test]
    fn test_sigv4_list_example() {
        let creds = Credentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            session_token: None,
        };
        // 20130524T000000Z
        let now = UNIX_EPOCH + Duration::from_secs(1369353600);
        let headers = sign(
            &creds,
            "us-east-1",
            "GET",
            "examplebucket.s3.amazonaws.com",
            "/",
            "max-keys=2&prefix=J",
            b"",
            now,
        );
        let auth = &headers
            .iter()
            .find(|(k, _)| k == "authorization")
            .unwrap()
            .1;
        assert_eq!(
            auth,
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
             SignedHeaders=host;x-amz-content-sha256;x-amz-date, \
             Signature=34b48302e7b5fa45bde8084f4b7868a86f0a534bc59db6670ed5711ef69dc6f7"
        );
    }

    #[test]
    fn test_amz_dates() {
        let now = UNIX_EPOCH + Duration::from_secs(1_793_276_096); // 2026-10-29T12:14:56Z
        assert_eq!(
            amz_dates(now),
            ("20261029".to_string(), "20261029T121456Z".to_string())
        );
    }

    #[test]
    fn test_xml_elements() {
        let xml = "<R><Key>a/b</Key><Key>c&amp;d</Key><IsTruncated>false</IsTruncated></R>";
        assert_eq!(xml_elements(xml, "Key"), vec!["a/b", "c&d"]);
        assert_eq!(xml_elements(xml, "IsTruncated"), vec!["false"]);
    }
}
