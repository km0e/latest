use std::env;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use reqwest::Client;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use xcfg::XCfg;
mod config;
mod source;

fn default_config() -> PathBuf {
    home::home_dir()
        .expect("can't find home directory")
        .join(".config/latest/config")
}

fn get_client() -> Result<Client> {
    let proxy = env::var("http_proxy").ok();
    let client = reqwest::Client::builder();
    let client = if let Some(proxy) = proxy {
        println!("using proxy {}", proxy);
        client.proxy(reqwest::Proxy::all(proxy)?)
    } else {
        println!("not using proxy");
        client
    };
    Ok(client.build()?)
}

#[derive(Parser, Debug)]
#[command(version = "0.1")]
struct Cli {
    #[command(subcommand)]
    command: Service,
    #[arg(short, long, default_value_os_t = default_config())]
    config: PathBuf,
}

#[derive(Subcommand, Debug)]
enum Service {
    #[command(visible_alias = "ls")]
    List,
    Sync,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::Registry::default()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
    let args = Cli::parse();
    let config = config::Config::load(&args.config)
        .with_context(|| {
            format!(
                "failed to load config from {}.[toml|json|yaml]",
                args.config.display()
            )
        })?
        .into_inner();
    let client = get_client()?;
    match args.command {
        Service::List => {
            let (sources, errors) = source::Sources::new(config);
            for error in errors {
                eprintln!("{}", error);
            }
            sources.list(&client).await;
        }
        Service::Sync => {
            let (sources, errors) = source::Sources::new(config);
            for error in errors {
                eprintln!("{}", error);
            }
            sources.sync(&client).await;
        }
    }
    Ok(())
}
