use anyhow::{Context, Result};
use regex::Regex;
use reqwest::{header::USER_AGENT, Response, Url};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, trace};

use super::Source;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Root {
    pub assets: Vec<Asset>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub browser_download_url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GithubConfig {
    pub repo: String,
    pub reg: String,
}

#[derive(Debug)]
pub struct Github {
    api: Url,
    reg: Regex,
}

impl Github {
    pub fn new(config: GithubConfig) -> Result<Self> {
        let api = Url::parse(&format!(
            "https://api.github.com/repos/{}/releases/latest",
            &config.repo
        ))?;
        let reg = Regex::new(&config.reg)?;
        info!(%api, %reg);
        Ok(Self { api, reg })
    }
}

impl Source for Github {
    #[instrument(skip(self, client))]
    async fn link(&self, client: &reqwest::Client) -> Result<String> {
        let resp = client
            .get(self.api.clone())
            .header(USER_AGENT, "reqwest")
            .send()
            .await
            .inspect_err(|e| trace!(%e))
            .with_context(|| format!("failed to get {}", self.api))?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("failed to get {}", self.api));
        }

        let link = resp
            .json::<Root>()
            .await?
            .assets
            .into_iter()
            .find_map(|x| {
                self.reg
                    .is_match(&x.browser_download_url)
                    .then_some(x.browser_download_url.clone())
            })
            .ok_or_else(|| {
                trace!("no assets found");
                anyhow::anyhow!("no assets found")
            })?;
        Ok(link)
    }

    #[instrument(skip(self, client))]
    async fn sync(&self, client: &reqwest::Client) -> Result<(String, Response)> {
        let link = self.link(client).await?;
        info!(%link);
        let url = Url::parse(&link)?;
        let name = url
            .path_segments()
            .ok_or_else(|| {
                trace!("no path segments found");
                anyhow::anyhow!("no path segments found")
            })?
            .last()
            .ok_or_else(|| {
                trace!("no last path segment found");
                anyhow::anyhow!("no last path segment found")
            })?
            .to_string();
        let resp = client.get(url).send().await?;
        Ok((name, resp))
    }
}
