mod appimage;
pub mod image;
mod install;
pub mod remove;
pub mod run;
pub mod update;

use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use indicatif::MultiProgress;
use install::Installer;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{core::constant::PACKAGES_PATH, registry::installed::InstalledPackages};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Package {
    pub name: String,
    pub bin_name: String,
    pub description: String,
    pub note: String,
    pub version: String,
    pub download_url: String,
    pub size: String,
    pub bsum: String,
    pub build_date: String,
    pub src_url: String,
    pub web_url: String,
    pub build_script: String,
    pub build_log: String,
    pub category: String,
    pub extra_bins: String,
    pub icon: String,
    pub desktop: Option<String>,
    pub bin_id: Option<String>,
    pub family: Option<String>,
}

#[derive(Default, Debug, Clone)]
pub struct ResolvedPackage {
    pub repo_name: String,
    pub collection: String,
    pub package: Package,
}

impl ResolvedPackage {
    pub async fn install(
        &self,
        idx: usize,
        total: usize,
        installed_packages: Arc<Mutex<InstalledPackages>>,
        portable: Option<String>,
        portable_home: Option<String>,
        portable_config: Option<String>,
        multi_progress: Arc<MultiProgress>,
        yes: bool,
    ) -> Result<()> {
        let install_path = self.package.get_install_path(&self.package.bsum[..8]);
        let mut installer = Installer::new(self, install_path);
        installer
            .execute(
                idx,
                total,
                installed_packages,
                portable,
                portable_home,
                portable_config,
                multi_progress,
                yes,
            )
            .await?;
        Ok(())
    }
}

impl Package {
    pub fn get_install_dir(&self, checksum: &str) -> PathBuf {
        PACKAGES_PATH.join(format!("{}-{}", &checksum[..8], self.full_name('-')))
    }

    pub fn get_install_path(&self, checksum: &str) -> PathBuf {
        self.get_install_dir(checksum).join(&self.bin_name)
    }

    pub fn full_name(&self, join_char: char) -> String {
        let family_prefix = self
            .family
            .to_owned()
            .map(|family| format!("{}{}", family, join_char))
            .unwrap_or_default();
        format!("{}{}", family_prefix, self.name)
    }
}

#[derive(Debug)]
pub struct PackageQuery {
    pub name: String,
    pub family: Option<String>,
    pub collection: Option<String>,
}

pub fn parse_package_query(query: &str) -> PackageQuery {
    let (base_query, collection) = query
        .rsplit_once('#')
        .map(|(n, r)| (n.to_owned(), (!r.is_empty()).then(|| r.to_lowercase())))
        .unwrap_or((query.to_owned(), None));

    let (name, family) = base_query
        .split_once('/')
        .map(|(v, n)| (n.to_owned(), Some(v.to_owned())))
        .unwrap_or((base_query, None));

    PackageQuery {
        name,
        family,
        collection,
    }
}
