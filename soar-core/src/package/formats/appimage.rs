use std::{
    fs,
    path::{Path, PathBuf},
};

use squishy::{appimage::AppImage, EntryKind};

use crate::{
    constants::PNG_MAGIC_BYTES, database::models::PackageExt, error::ErrorContext,
    utils::calc_magic_bytes, SoarResult,
};

pub async fn integrate_appimage<P: AsRef<Path>, T: PackageExt>(
    install_dir: P,
    file_path: P,
    package: &T,
    icon: &mut Option<PathBuf>,
    desktop: &mut Option<PathBuf>,
) -> SoarResult<()> {
    if icon.is_some() && desktop.is_some() {
        return Ok(());
    }

    let install_dir = install_dir.as_ref();
    let pkg_name = package.pkg_name();
    let appimage = AppImage::new(None, &file_path, None)?;
    let squashfs = &appimage.squashfs;

    if icon.is_none() {
        if let Some(entry) = appimage.find_icon() {
            if let EntryKind::File(basic_file) = entry.kind {
                let dest = format!("{}/{}.DirIcon", install_dir.display(), pkg_name);
                let _ = squashfs.write_file(basic_file, &dest);

                let magic_bytes = calc_magic_bytes(&dest, 8)?;
                let ext = if magic_bytes == PNG_MAGIC_BYTES {
                    "png"
                } else {
                    "svg"
                };
                let final_path = format!("{}/{}.{ext}", install_dir.display(), pkg_name);
                fs::rename(&dest, &final_path)
                    .with_context(|| format!("renaming from {} to {}", dest, final_path))?;

                *icon = Some(PathBuf::from(&final_path));
            }
        }
    }

    if desktop.is_none() {
        if let Some(entry) = appimage.find_desktop() {
            if let EntryKind::File(basic_file) = entry.kind {
                let dest = format!("{}/{}.desktop", install_dir.display(), pkg_name);
                let _ = squashfs.write_file(basic_file, &dest);
                *desktop = Some(PathBuf::from(&dest));
            }
        }
    }

    if let Some(entry) = appimage.find_appstream() {
        if let EntryKind::File(basic_file) = entry.kind {
            let file_name = if entry
                .path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains("appdata")
            {
                "appdata"
            } else {
                "metainfo"
            };
            let dest = format!("{}/{}.{file_name}.xml", install_dir.display(), pkg_name);
            let _ = squashfs.write_file(basic_file, &dest);
        }
    }
    Ok(())
}
