use std::{
    collections::{HashMap, HashSet},
    env::{self, consts::ARCH},
    fs,
    path::PathBuf,
    sync::LazyLock,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::error;

use super::{
    constant::REGISTRY_PATH,
    util::{home_config_path, home_data_path},
};

/// Application's configuration
#[derive(Deserialize, Serialize)]
pub struct Config {
    /// Path to the directory where app data is stored.
    pub soar_root: String,

    /// Path to the directory where cache is stored.
    pub soar_cache: Option<String>,

    /// Path to the directory where binary symlinks is stored.
    pub soar_bin: Option<String>,

    /// A list of remote repositories to fetch packages from.
    pub repositories: Vec<Repository>,

    /// Indicates whether downloads should be performed in parallel.
    pub parallel: Option<bool>,

    /// Limit the number of parallel downloads
    pub parallel_limit: Option<u32>,

    /// Limit the number of search results to display
    pub search_limit: Option<usize>,
}

/// Struct representing a repository configuration.
#[derive(Deserialize, Serialize)]
pub struct Repository {
    /// Name of the repository.
    pub name: String,

    /// URL of the repository.
    pub url: String,

    /// Optional field specifying a custom metadata file for the repository. Default:
    /// `metadata.json`
    pub metadata: Option<String>,

    /// Download Sources for different collections
    pub sources: HashMap<String, String>,
}

impl Repository {
    pub fn get_path(&self) -> PathBuf {
        REGISTRY_PATH.join(&self.name)
    }
}

impl Config {
    /// Creates a new configuration by loading it from the configuration file.
    /// If the configuration file is not found, it uses the default configuration.
    pub fn new() -> Self {
        let home_config = home_config_path();
        let pkg_config = PathBuf::from(home_config).join("soar");
        let config_path = pkg_config.join("config.json");

        let mut config = match fs::read(&config_path) {
            Ok(content) => serde_json::from_slice(&content).unwrap_or_else(|e| {
                error!("Failed to parse config file: {}", e.to_string());
                std::process::exit(1);
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                error!("Error reading config file: {}", e.to_string());
                std::process::exit(1);
            }
        };

        config.soar_root = env::var("SOAR_ROOT").unwrap_or(config.soar_root);
        config.soar_bin = Some(env::var("SOAR_BIN").unwrap_or_else(|_| {
            config
                .soar_bin
                .unwrap_or_else(|| format!("{}/bin", config.soar_root))
        }));
        config.soar_cache = Some(env::var("SOAR_CACHE").unwrap_or_else(|_| {
            config
                .soar_cache
                .unwrap_or_else(|| format!("{}/cache", config.soar_root))
        }));

        let mut seen = HashSet::new();
        for repo in &config.repositories {
            if !seen.insert(&repo.name) {
                error!("Found duplicate repo '{}'. Please rename the repo to have unique name. Aborting..", repo.name);
                std::process::exit(1);
            }
        }

        config
    }
}

impl Default for Config {
    fn default() -> Self {
        let sources = HashMap::from([
            ("bin".to_owned(), format!("https://bin.pkgforge.dev/{ARCH}")),
            (
                "base".to_owned(),
                format!("https://bin.pkgforge.dev/{ARCH}/Baseutils"),
            ),
            ("pkg".to_owned(), format!("https://pkg.pkgforge.dev/{ARCH}")),
        ]);

        let soar_root =
            env::var("SOAR_ROOT").unwrap_or_else(|_| format!("{}/soar", home_data_path()));

        Self {
            soar_root: soar_root.clone(),
            soar_bin: Some(format!("{}/bin", soar_root)),
            soar_cache: Some(format!("{}/cache", soar_root)),
            repositories: vec![Repository {
                name: "pkgforge".to_owned(),
                url: format!("https://bin.pkgforge.dev/{ARCH}"),
                metadata: Some("METADATA.AIO.json".to_owned()),
                sources,
            }],
            parallel: Some(true),
            parallel_limit: Some(4),
            search_limit: Some(20),
        }
    }
}

/// Initializes the global configuration by forcing the static `CONFIG` to load.
pub fn init() {
    let _ = &*CONFIG;
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::new);

pub fn generate_default_config() -> Result<()> {
    let home_config = home_config_path();
    let config_path = PathBuf::from(home_config).join("soar").join("config.json");

    if config_path.exists() {
        error!("Default config already exists. Not overriding it.");
        std::process::exit(1);
    }

    fs::create_dir_all(config_path.parent().unwrap())?;

    let def_config = Config::default();
    let serialized = serde_json::to_vec_pretty(&def_config)?;
    fs::write(&config_path, &serialized)?;

    println!("Default config is saved at: {}", config_path.display());

    Ok(())
}
