use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::info;

use crate::{
    core::{
        color::{Color, ColorExt},
        constant::{BIN_PATH, INSTALL_TRACK_PATH, PACKAGES_PATH},
        util::{format_bytes, parse_size},
    },
    package::{parse_package_query, remove::Remover, ResolvedPackage},
};

use super::storage::PackageStorage;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstalledPackage {
    pub repo_name: String,
    pub collection: String,
    pub name: String,
    pub family: Option<String>,
    pub bin_name: String,
    pub version: String,
    pub checksum: String,
    pub size: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InstalledPackages {
    pub packages: Vec<InstalledPackage>,
}

impl InstalledPackages {
    pub async fn new() -> Result<Self> {
        let path = INSTALL_TRACK_PATH.join("latest");

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create installs directory to track installations.")?;
        }

        let packages = if path.exists() {
            let content = tokio::fs::read(&path)
                .await
                .context("Failed to read installed packages")?;

            let mut de = rmp_serde::Deserializer::new(&content[..]);

            InstalledPackages::deserialize(&mut de)?
        } else {
            InstalledPackages {
                packages: Vec::new(),
            }
        };

        Ok(packages)
    }

    pub fn is_installed(&self, package: &ResolvedPackage) -> bool {
        self.packages
            .iter()
            .any(|installed| installed.full_name('-') == package.package.full_name('-'))
    }

    fn find_package_mut(&mut self, package: &ResolvedPackage) -> Option<&mut InstalledPackage> {
        self.packages
            .iter_mut()
            .find(|installed| installed.full_name('-') == package.package.full_name('-'))
    }

    pub fn find_package(&self, package: &ResolvedPackage) -> Option<&InstalledPackage> {
        self.packages.iter().find(|installed| {
            installed.repo_name == package.repo_name
                && installed.collection == package.collection
                && installed.full_name('-') == package.package.full_name('-')
        })
    }

    pub async fn register_package(
        &mut self,
        resolved_package: &ResolvedPackage,
        checksum: &str,
    ) -> Result<()> {
        let package = resolved_package.package.to_owned();

        let new_installed = InstalledPackage {
            repo_name: resolved_package.repo_name.to_owned(),
            collection: resolved_package.collection.to_string().to_owned(),
            name: package.pkg,
            family: package.family,
            bin_name: package.pkg_name,
            version: package.version,
            checksum: checksum.to_owned(),
            size: parse_size(&package.size).unwrap_or_default(),
            timestamp: Utc::now(),
        };

        if let Some(installed) = self.find_package_mut(resolved_package) {
            *installed = new_installed;
        } else {
            self.packages.push(new_installed);
        }

        self.save().await?;

        Ok(())
    }

    pub async fn unregister_package(&mut self, installed_package: &InstalledPackage) -> Result<()> {
        self.packages
            .retain(|installed| installed.full_name('-') != installed_package.full_name('-'));

        self.save().await?;

        Ok(())
    }

    pub async fn remove(&mut self, installed_package: &InstalledPackage) -> Result<()> {
        let remover = Remover::new(installed_package).await?;
        remover.execute(self).await?;
        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        let path = INSTALL_TRACK_PATH.join("latest");

        let content = rmp_serde::to_vec(&self)
            .context("Failed to serialize installed packages to MessagePack")?;

        fs::write(&path, content).await.context(format!(
            "Failed to write to {}",
            path.to_string_lossy().color(Color::Red)
        ))?;

        Ok(())
    }

    pub async fn info(
        &self,
        packages: Option<&[String]>,
        package_store: &PackageStorage,
    ) -> Result<()> {
        let mut total: HashMap<String, (u32, u64)> = HashMap::new();

        let resolved_packages = packages
            .map(|pkgs| {
                pkgs.iter()
                    .flat_map(|package| {
                        let query = parse_package_query(package);
                        package_store
                            .get_packages(&query)
                            .unwrap_or_default()
                            .into_iter()
                            .filter_map(|package| self.find_package(&package).cloned())
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| self.packages.clone());

        if resolved_packages.is_empty() {
            return Err(anyhow::anyhow!("No installed packages"));
        }

        resolved_packages.iter().for_each(|package| {
            info!(
                "- [{}] {1}:{1}-{2} ({3}) ({4})",
                package.collection.clone().color(Color::BrightGreen),
                package.name.clone().color(Color::Blue),
                package.version.clone().color(Color::Green),
                package
                    .timestamp
                    .format("%Y-%m-%d %H:%M:%S")
                    .color(Color::Yellow),
                format_bytes(package.size).color(Color::Magenta)
            );

            let (count, size) = total.get(&package.collection).unwrap_or(&(0, 0));
            total.insert(
                package.collection.to_owned(),
                (count + 1, size + package.size),
            );
        });
        info!("{:<2} Installed:", "");

        for (collection, (count, size)) in total.iter() {
            info!(
                "{:<4} {}: {} ({})",
                "",
                collection,
                count.color(Color::BrightGreen),
                format_bytes(size.to_owned())
            );
        }

        let (count, value) = total
            .values()
            .fold((0, 0), |(count_acc, value_acc), &(count, value)| {
                (count_acc + count, value_acc + value)
            });

        info!(
            "{:<2} Total: {} ({})",
            "",
            count.color(Color::BrightYellow),
            format_bytes(value)
        );

        Ok(())
    }

    pub async fn use_package(&self, resolved_package: &ResolvedPackage) -> Result<()> {
        if let Some(installed) = self.find_package(resolved_package) {
            let install_path = resolved_package
                .package
                .get_install_path(&installed.checksum);
            let symlink_path = &BIN_PATH.join(&installed.bin_name);

            if symlink_path.exists() {
                if xattr::get_deref(symlink_path, "user.managed_by")?.as_deref() != Some(b"soar") {
                    return Err(anyhow::anyhow!(
                        "{} is not managed by soar",
                        symlink_path.to_string_lossy().color(Color::Blue)
                    ));
                }
                fs::remove_file(symlink_path).await?;
            }

            fs::symlink(&install_path, symlink_path)
                .await
                .context(format!(
                    "Failed to link {} to {}",
                    install_path.to_string_lossy().color(Color::Blue),
                    symlink_path.to_string_lossy().color(Color::Blue)
                ))?;
        } else {
            return Err(anyhow::anyhow!("NOT_INSTALLED"));
        }

        Ok(())
    }
}

impl InstalledPackage {
    pub fn full_name(&self, join_char: char) -> String {
        let family_prefix = self
            .family
            .to_owned()
            .map(|family| format!("{}{}", family, join_char))
            .unwrap_or_default();
        format!("{}{}", family_prefix, self.name)
    }

    pub fn get_install_dir(&self) -> PathBuf {
        PACKAGES_PATH.join(format!("{}-{}", &self.checksum[..8], self.full_name('-')))
    }

    pub fn get_install_path(&self) -> PathBuf {
        self.get_install_dir().join(&self.bin_name)
    }
}
