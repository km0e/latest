use super::source::GithubConfig;
use super::source::GitlabConfig;
use serde::{Deserialize, Serialize};
use xcfg::XCfg;

#[derive(Debug, XCfg, Serialize, Deserialize)]
pub struct Config {
    pub github: Option<Vec<GithubConfig>>,
    pub gitlab: Option<Vec<GitlabConfig>>,
}

#[cfg(test)]
mod tests {
    use scopeguard::defer;

    use super::*;
    use std::env::temp_dir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_config() {
        let temp_dir = temp_dir();
        let config_path = temp_dir.join("config.toml");
        let mut file = File::create(&config_path).unwrap();
        defer! {
            std::fs::remove_file(&config_path).unwrap();
        }
        file.write_all(
            br#"
[[gitlab]]
host = "gitlab.com"
id = "3472737"
            
[[github]]
repo = "github.com"
reg = "github"

"#,
        )
        .unwrap();

        let config = Config::load(&config_path).unwrap().into_inner();
        assert_eq!(
            config.github,
            Some(vec![GithubConfig {
                repo: "github.com".to_string(),
                reg: "github".to_string(),
            }])
        );
        assert_eq!(
            config.gitlab,
            Some(vec![GitlabConfig {
                host: "gitlab.com".to_string(),
                id: "3472737".to_string(),
                reg: None
            }])
        );

        let mut config = config;
        config.github = Some(vec![GithubConfig {
            repo: "gitlab.com".to_string(),
            reg: "gitlab".to_string(),
        }]);
        config.gitlab = Some(vec![GitlabConfig {
            host: "github.com".to_string(),
            id: "3472737".to_string(),
            reg: None,
        }]);
        config.save(&config_path).unwrap();

        let config = Config::load(&config_path).unwrap().into_inner();
        assert!(
            config.github
                == Some(vec![GithubConfig {
                    repo: "gitlab.com".to_string(),
                    reg: "gitlab".to_string(),
                }])
        );
        assert!(
            config.gitlab
                == Some(vec![GitlabConfig {
                    host: "github.com".to_string(),
                    id: "3472737".to_string(),
                    reg: None
                }])
        );
    }
}
