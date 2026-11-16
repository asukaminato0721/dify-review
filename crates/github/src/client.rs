//! Minimal GitHub client: GraphQL for history, REST for one pull request.
//! Every request retries server errors and secondary rate limits.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use jiff::Timestamp;
use reqwest::Method;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

const API: &str = "https://api.github.com";
const MAX_ATTEMPTS: u32 = 6;
/// Below this many GraphQL points left, wait for the reset.
const LOW_WATERMARK: i64 = 100;

pub struct GitHub {
    http: reqwest::Client,
    token: String,
}

/// Every GraphQL query asks for `rateLimit { cost remaining resetAt }`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    pub cost: i64,
    pub remaining: i64,
    pub reset_at: Timestamp,
}

pub trait HasRateLimit {
    fn rate_limit(&self) -> Option<&RateLimit>;
}

#[derive(Deserialize)]
struct Envelope<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GqlError>,
}

#[derive(Debug, Deserialize)]
struct GqlError {
    message: String,
}

/// A successful response: its body and the `rel="next"` page, if any.
struct Page {
    text: String,
    next: Option<String>,
}

enum Outcome {
    Done(Page),
    Retry(Duration, String),
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
        let from_var = ["GITHUB_TOKEN", "GH_TOKEN"]
            .iter()
            .filter_map(|v| std::env::var(v).ok())
            .map(|t| t.trim().to_owned())
            .find(|t| !t.is_empty());
        match from_var {
            Some(t) => Self::new(t),
            None => {
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
        }
    }

    /// Run a GraphQL query, pausing when the point budget runs low.
    pub async fn query<T: DeserializeOwned + HasRateLimit>(
        &self,
        query: &str,
        variables: Value,
    ) -> Result<T> {
        let body = serde_json::json!({ "query": query, "variables": variables });
        let page = self
            .send(Method::POST, &format!("{API}/graphql"), Some(&body))
            .await?;
        let env: Envelope<T> = serde_json::from_str(&page.text)
            .with_context(|| format!("decoding GraphQL response: {}", snippet(&page.text)))?;
        if !env.errors.is_empty() {
            let msgs: Vec<_> = env.errors.iter().map(|e| e.message.as_str()).collect();
            bail!("GitHub GraphQL errors: {}", msgs.join("; "));
        }
        let data = env
            .data
            .context("GraphQL response had neither data nor errors")?;
        if let Some(rl) = data.rate_limit()
            && rl.remaining < LOW_WATERMARK
        {
            let secs = rl
                .reset_at
                .duration_since(Timestamp::now())
                .as_secs()
                .max(0) as u64
                + 5;
            tracing::warn!(
                remaining = rl.remaining,
                secs,
                "rate limit low, waiting for reset"
            );
            tokio::time::sleep(Duration::from_secs(secs)).await;
        }
        Ok(data)
    }

    /// GET one REST resource, `path` relative to the API root.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let page = self
            .send(Method::GET, &format!("{API}{path}"), None)
            .await?;
        serde_json::from_str(&page.text).with_context(|| format!("decoding GET {path}"))
    }

    /// GET every page of a REST list, following `Link: rel="next"`.
    pub async fn get_all<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>> {
        let sep = if path.contains('?') { '&' } else { '?' };
        let mut url = Some(format!("{API}{path}{sep}per_page=100"));
        let mut items = Vec::new();
        while let Some(u) = url {
            let page = self.send(Method::GET, &u, None).await?;
            let batch: Vec<T> =
                serde_json::from_str(&page.text).with_context(|| format!("decoding GET {path}"))?;
            items.extend(batch);
            url = page.next;
        }
        Ok(items)
    }

    /// POST a REST resource and decode the reply.
    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        let page = self
            .send(Method::POST, &format!("{API}{path}"), Some(body))
            .await?;
        serde_json::from_str(&page.text).with_context(|| format!("decoding POST {path}"))
    }

    /// Up to MAX_ATTEMPTS tries; the last one's retry reason becomes the error.
    async fn send(&self, method: Method, url: &str, body: Option<&Value>) -> Result<Page> {
        for attempt in 1..MAX_ATTEMPTS {
            match self.try_once(method.clone(), url, body).await? {
                Outcome::Done(page) => return Ok(page),
                Outcome::Retry(wait, why) => {
                    tracing::warn!(attempt, wait_secs = wait.as_secs(), %why, "retrying");
                    tokio::time::sleep(wait).await;
                }
            }
        }
        match self.try_once(method, url, body).await? {
            Outcome::Done(page) => Ok(page),
            Outcome::Retry(_, why) => {
                bail!("GitHub request failed after {MAX_ATTEMPTS} attempts: {why}")
            }
        }
    }

    async fn try_once(&self, method: Method, url: &str, body: Option<&Value>) -> Result<Outcome> {
        let backoff = Duration::from_secs(8);
        let req = self
            .http
            .request(method, url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        let req = match body {
            Some(b) => req.json(b),
            None => req,
        };
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) if e.is_timeout() || e.is_connect() => {
                return Ok(Outcome::Retry(backoff, e.to_string()));
            }
            Err(e) => return Err(e.into()),
        };
        let status = resp.status();
        let header = |name: &str| {
            resp.headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        let retry_after = header("retry-after")
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs);
        let next = header("link").and_then(|l| next_link(&l));
        let text = resp.text().await?;
        match status.as_u16() {
            200..=299 => Ok(Outcome::Done(Page { text, next })),
            401 => bail!("GitHub rejected the token (401)"),
            // Secondary rate limits come back as 403 or 429.
            403 | 429 => Ok(Outcome::Retry(
                retry_after.unwrap_or(Duration::from_secs(60)),
                format!("{status}: {}", snippet(&text)),
            )),
            500..=599 => Ok(Outcome::Retry(
                backoff,
                format!("{status}: {}", snippet(&text)),
            )),
            _ => bail!("GitHub {status}: {}", snippet(&text)),
        }
    }
}

/// The `rel="next"` URL of a `Link` header.
fn next_link(link: &str) -> Option<String> {
    link.split(',').find_map(|part| {
        let (url, rel) = part.split_once(';')?;
        rel.contains("rel=\"next\"").then(|| {
            url.trim()
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_owned()
        })
    })
}

fn snippet(s: &str) -> &str {
    let end = s.char_indices().nth(300).map_or(s.len(), |(i, _)| i);
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_next_link() {
        let l = r#"<https://api.github.com/x?page=2>; rel="next", <https://api.github.com/x?page=5>; rel="last""#;
        assert_eq!(
            next_link(l).as_deref(),
            Some("https://api.github.com/x?page=2")
        );
        assert_eq!(next_link(r#"<https://a/x?page=1>; rel="prev""#), None);
    }
}
