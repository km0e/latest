use anyhow::{Context, Result};
use regex::Regex;
use reqwest::{Response, Url};
use serde::{Deserialize, Serialize};
use tracing::trace;

use super::Source;

pub type Root = Vec<Root2>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Root2 {
    pub assets: Assets,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assets {
    pub links: Vec<Link>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub direct_asset_url: String,
    pub link_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitlabConfig {
    pub host: String,
    pub id: String,
    pub reg: Option<String>,
}

#[derive(Debug)]
pub struct Gitlab {
    api: Url,
    reg: Option<Regex>,
}

impl Gitlab {
    pub fn new(config: GitlabConfig) -> Result<Self> {
        let api = Url::parse(&format!(
            "https://{}/api/v4/projects/{}/releases",
            &config.host, &config.id
        ))?;
        let reg = match &config.reg {
            Some(reg) => Some(Regex::new(reg)?),
            None => None,
        };
        Ok(Self { api, reg })
    }
}

impl Source for Gitlab {
    async fn link(&self, client: &reqwest::Client) -> Result<String> {
        let resp = client
            .get(self.api.clone())
            .send()
            .await
            .with_context(|| format!("failed to get {}", self.api))?;
        let link = resp
            .json::<Root>()
            .await?
            .into_iter()
            .next()
            .ok_or(anyhow::anyhow!("no releases"))?
            .assets
            .links
            .into_iter()
            .find_map(|x| match &self.reg {
                Some(reg) if !reg.is_match(&x.direct_asset_url) => None,
                _ => Some(x.direct_asset_url),
            });
        link.ok_or(anyhow::anyhow!("no assets"))
    }

    async fn sync(&self, client: &reqwest::Client) -> Result<(String, Response)> {
        let link = self.link(client).await?;
        let url = Url::parse(&link)?;
        let name = url
            .path_segments()
            .ok_or_else(|| {
                trace!("no path segments");
                anyhow::anyhow!("no path segments")
            })?
            .last()
            .ok_or_else(|| {
                trace!("no last segment");
                anyhow::anyhow!("no last segment")
            })?
            .to_string();
        let resp = client.get(url).send().await?;
        Ok((name, resp))
    }
}
