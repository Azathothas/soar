use anyhow::{Context, Result};
use tokio::fs;

use crate::{
    core::{
        color::{Color, ColorExt},
        config::Repository,
    },
    warn,
};

use super::fetcher::RegistryFetcher;

pub struct RegistryLoader;

impl RegistryLoader {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, repo: &Repository, fetcher: &RegistryFetcher) -> Result<Vec<u8>> {
        let checksum = fetcher.checksum(repo).await;

        if let Ok(checksum) = checksum {
            let checksum_path = repo
                .get_path()
                .with_file_name(format!("{}.remote.bsum", repo.name));
            let local_checksum = fs::read(&checksum_path).await.unwrap_or_default();
            if checksum != local_checksum {
                warn!("Local registry is outdated. Refetching...");
                let content = fetcher.execute(repo).await;
                fs::write(checksum_path, &checksum).await?;
                return content;
            }
        }

        let path = repo.get_path();
        let content = fs::read(path)
            .await
            .context("Failed to load registry path.")?;
        Ok(content)
    }
}
