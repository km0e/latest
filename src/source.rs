mod github;
use core::fmt;
use std::io::Write;

use anyhow::Result;
use futures::StreamExt;
pub use github::*;
mod gitlab;
pub use gitlab::*;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use reqwest::Response;
use tracing::instrument;

use crate::config::Config;

#[derive(Debug)]
pub struct Sources {
    github: Vec<Github>,
    gitlab: Vec<Gitlab>,
}

trait Source {
    async fn link(&self, client: &reqwest::Client) -> Result<String>;
    async fn sync(&self, client: &reqwest::Client) -> Result<(String, Response)>;
}

impl Sources {
    pub fn new(config: Config) -> (Self, Vec<anyhow::Error>) {
        let (gh, gh_err): (Vec<_>, Vec<_>) = config
            .github
            .clone()
            .unwrap_or_default()
            .iter()
            .cloned()
            .map(Github::new)
            .partition(Result::is_ok);
        let (gl, gl_err): (Vec<_>, Vec<_>) = config
            .gitlab
            .clone()
            .unwrap_or_default()
            .iter()
            .cloned()
            .map(Gitlab::new)
            .partition(Result::is_ok);
        let ghs: Vec<Github> = gh.into_iter().map(Result::unwrap).collect();
        let gls: Vec<Gitlab> = gl.into_iter().map(Result::unwrap).collect();
        let mut errors = gh_err
            .into_iter()
            .map(Result::unwrap_err)
            .collect::<Vec<_>>();
        errors.extend(gl_err.into_iter().map(Result::unwrap_err));
        (
            Self {
                github: ghs,
                gitlab: gls,
            },
            errors,
        )
    }
    #[instrument]
    pub async fn list(&self, client: &reqwest::Client) {
        let mut results = Vec::new();
        for github in &self.github {
            results.push(github.link(client).await);
        }
        for gitlab in &self.gitlab {
            results.push(gitlab.link(client).await);
        }
        for result in results {
            match result {
                Ok(link) => {
                    println!("{}", link);
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }
    #[instrument(skip(self, client))]
    pub async fn sync(&self, client: &reqwest::Client) {
        let mut results = Vec::new();
        for github in &self.github {
            results.push(github.sync(client).await);
        }
        for gitlab in &self.gitlab {
            results.push(gitlab.sync(client).await);
        }
        for hd in results.into_iter().filter_map(|result| {
            match result {
                Ok((name, resp)) => {
                    Some(tokio::spawn(async move {
                        println!("{}: {}", name, resp.status());
                        let pb = ProgressBar::new(resp.content_length().unwrap_or(0));
                        let tk = |state: &ProgressState, w: &mut dyn fmt::Write| {
                            write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
                        };
                        pb.set_style(
                        ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                        .unwrap().with_key("eta", tk)
                        .progress_chars("#>-"));
                        let mut stream = resp.bytes_stream();
                        let file = std::fs::File::create(&name).unwrap();
                        let mut writer = std::io::BufWriter::new(file);
                        let mut bytes = 0;
                        while let Some(chunk) = stream.next().await {
                            let chunk = chunk.unwrap();
                            bytes += chunk.len();
                            pb.set_position(bytes as u64);
                            writer.write_all(&chunk).unwrap();
                        }
                        pb.finish();
                    }))
                }
                Err(e) => {println!("Error: {}", e);None}
            }
        }){
            hd.await.unwrap();
        }
    }
}
