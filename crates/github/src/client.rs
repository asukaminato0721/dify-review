//! Minimal GitHub GraphQL client with retries and rate-limit waits.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

const ENDPOINT: &str = "https://api.github.com/graphql";
const MAX_ATTEMPTS: u32 = 6;
/// Below this many points left, wait for the reset instead of failing later.
const LOW_WATERMARK: i64 = 100;

pub struct GitHub {
    http: reqwest::Client,
    token: String,
}

/// Every query asks for `rateLimit { cost remaining resetAt }`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    pub cost: i64,
    pub remaining: i64,
    pub reset_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct Envelope<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GqlError>,
}

#[derive(Debug, Deserialize)]
struct GqlError {
    #[serde(rename = "type")]
    kind: Option<String>,
    message: String,
}

impl GitHub {
    pub fn new(token: String) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent("dify-review")
            .timeout(Duration::from_secs(90))
            .build()?;
        Ok(Self { http, token })
    }

    /// Token from `GITHUB_TOKEN` / `GH_TOKEN`, else `gh auth token`.
    pub async fn from_env() -> Result<Self> {
        for var in ["GITHUB_TOKEN", "GH_TOKEN"] {
            if let Ok(t) = std::env::var(var)
                && !t.trim().is_empty()
            {
                return Self::new(t.trim().to_owned());
            }
        }
        let out = tokio::process::Command::new("gh")
            .args(["auth", "token"])
            .output()
            .await
            .context("no GITHUB_TOKEN/GH_TOKEN set and `gh` is not runnable")?;
        if !out.status.success() {
            bail!("no GITHUB_TOKEN/GH_TOKEN set and `gh auth token` failed");
        }
        Self::new(String::from_utf8(out.stdout)?.trim().to_owned())
    }

    /// Run a query. `T` must include the `rateLimit` field as `rate_limit`
    /// via [`HasRateLimit`] so the client can pace itself.
    pub async fn query<T: DeserializeOwned + HasRateLimit>(
        &self,
        query: &str,
        variables: Value,
    ) -> Result<T> {
        let body = serde_json::json!({ "query": query, "variables": variables });
        let data = self.with_retries::<T>(&body).await?;
        if let Some(rl) = data.rate_limit()
            && rl.remaining < LOW_WATERMARK
        {
            let secs = (rl.reset_at - Utc::now()).num_seconds().max(0) as u64 + 5;
            tracing::warn!(
                remaining = rl.remaining,
                secs,
                "rate limit low, waiting for reset"
            );
            tokio::time::sleep(Duration::from_secs(secs)).await;
        }
        Ok(data)
    }

    /// Up to MAX_ATTEMPTS tries; the last one's Retry becomes the error.
    async fn with_retries<T: DeserializeOwned>(&self, body: &Value) -> Result<T> {
        for attempt in 1..MAX_ATTEMPTS {
            match self.try_once::<T>(body).await? {
                Outcome::Ok(data) => return Ok(data),
                Outcome::Retry(wait, why) => {
                    tracing::warn!(attempt, wait_secs = wait.as_secs(), %why, "retrying");
                    tokio::time::sleep(wait).await;
                }
            }
        }
        match self.try_once::<T>(body).await? {
            Outcome::Ok(data) => Ok(data),
            Outcome::Retry(_, why) => {
                bail!("GitHub GraphQL failed after {MAX_ATTEMPTS} attempts: {why}")
            }
        }
    }

    async fn try_once<T: DeserializeOwned>(&self, body: &Value) -> Result<Outcome<T>> {
        let backoff = Duration::from_secs(2u64.pow(3));
        let resp = match self
            .http
            .post(ENDPOINT)
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) if e.is_timeout() || e.is_connect() => {
                return Ok(Outcome::Retry(backoff, e.to_string()));
            }
            Err(e) => return Err(e.into()),
        };
        let status = resp.status();
        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs);
        let text = resp.text().await?;
        if status == 401 {
            bail!("GitHub rejected the token (401)");
        }
        // Secondary rate limits come back as 403 or 429.
        if status == 403 || status == 429 {
            return Ok(Outcome::Retry(
                retry_after.unwrap_or(Duration::from_secs(60)),
                format!("{status}: {}", snippet(&text)),
            ));
        }
        if status.is_server_error() {
            return Ok(Outcome::Retry(
                backoff,
                format!("{status}: {}", snippet(&text)),
            ));
        }
        if !status.is_success() {
            bail!("GitHub GraphQL {status}: {}", snippet(&text));
        }
        let env: Envelope<T> = serde_json::from_str(&text)
            .with_context(|| format!("decoding GraphQL response: {}", snippet(&text)))?;
        if let Some(e) = env.errors.first() {
            if e.kind.as_deref() == Some("RATE_LIMITED") {
                return Ok(Outcome::Retry(Duration::from_secs(60), e.message.clone()));
            }
            let msgs: Vec<_> = env.errors.iter().map(|e| e.message.as_str()).collect();
            bail!("GitHub GraphQL errors: {}", msgs.join("; "));
        }
        env.data
            .map(Outcome::Ok)
            .context("GraphQL response had neither data nor errors")
    }
}

enum Outcome<T> {
    Ok(T),
    Retry(Duration, String),
}

pub trait HasRateLimit {
    fn rate_limit(&self) -> Option<&RateLimit>;
}

fn snippet(s: &str) -> &str {
    let end = s.char_indices().nth(300).map_or(s.len(), |(i, _)| i);
    &s[..end]
}
