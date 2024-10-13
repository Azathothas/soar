use std::{
    collections::HashSet,
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use backhand::{kind::Kind, FilesystemReader, InnerNode, Node, SquashfsFileReader};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use tokio::fs;

use crate::core::{
    constant::{BIN_PATH, PACKAGES_PATH},
    util::home_data_path,
};

const SUPPORTED_DIMENSIONS: &[(u32, u32)] = &[
    (16, 16),
    (24, 24),
    (32, 32),
    (48, 48),
    (64, 64),
    (72, 72),
    (80, 80),
    (96, 96),
    (128, 128),
    (192, 192),
    (256, 256),
    (512, 512),
];

async fn find_offset(file: &mut BufReader<File>) -> Result<u64> {
    let mut magic = [0_u8; 4];
    // Little-Endian v4.0
    let kind = Kind::from_target("le_v4_0").unwrap();
    while file.read_exact(&mut magic).is_ok() {
        if magic == kind.magic() {
            let found = file.stream_position()? - magic.len() as u64;
            file.rewind()?;
            return Ok(found);
        }
    }
    file.rewind()?;
    Ok(0)
}

fn find_nearest_supported_dimension(width: u32, height: u32) -> (u32, u32) {
    SUPPORTED_DIMENSIONS
        .iter()
        .min_by_key(|&&(w, h)| {
            let width_diff = (w as i32 - width as i32).abs();
            let height_diff = (h as i32 - height as i32).abs();
            width_diff + height_diff
        })
        .cloned()
        .unwrap_or((width, height))
}

fn normalize_image(image: DynamicImage) -> DynamicImage {
    let (width, height) = image.dimensions();
    let (new_width, new_height) = find_nearest_supported_dimension(width, height);

    if (width, height) != (new_width, new_height) {
        println!(
            "Resizing image from {}x{} to {}x{}",
            width, height, new_width, new_height
        );
        image.resize(new_width, new_height, FilterType::Lanczos3)
    } else {
        image
    }
}

fn is_appimage(file: &mut BufReader<File>) -> bool {
    let mut magic_bytes = [0_u8; 16];
    let appimage_bytes = [
        0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00, 0x41, 0x49, 0x02, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];
    if file.read_exact(&mut magic_bytes).is_ok() {
        return appimage_bytes == magic_bytes;
    }
    false
}

async fn create_symlink(from: &Path, to: &Path) -> Result<()> {
    if to.exists() {
        if to.read_link().is_ok() && !to.read_link()?.starts_with(&*PACKAGES_PATH) {
            eprintln!("{} is not managed by soar", to.to_string_lossy());
            return Ok(());
        }
        fs::remove_file(to).await?;
    }
    fs::symlink(from, to).await?;

    Ok(())
}

async fn remove_link(path: &Path) -> Result<()> {
    if path.exists() {
        if path.read_link().is_ok() && !path.read_link()?.starts_with(&*PACKAGES_PATH) {
            eprintln!("{} is not managed by soar", path.to_string_lossy());
            return Ok(());
        }
        fs::remove_file(path).await?;
    }
    Ok(())
}

pub async fn remove_applinks(name: &str, file_path: &Path) -> Result<()> {
    let home_data = home_data_path();
    let data_path = Path::new(&home_data);

    let original_icon_path = file_path.with_extension("png");
    let (w, h) = image::image_dimensions(&original_icon_path)?;
    let icon_path = data_path
        .join("icons")
        .join("hicolor")
        .join(format!("{}x{}", w, h))
        .join("apps")
        .join(name)
        .with_extension("png");
    let desktop_path = data_path
        .join("applications")
        .join(format!("soar-{name}"))
        .with_extension("desktop");

    remove_link(&desktop_path).await?;
    remove_link(&icon_path).await?;

    Ok(())
}

pub async fn extract_appimage(name: &str, file_path: &Path) -> Result<()> {
    let mut file = BufReader::new(File::open(file_path)?);

    if !is_appimage(&mut file) {
        return Err(anyhow::anyhow!("NOT_APPIMAGE"));
    }

    let offset = find_offset(&mut file).await?;
    let squashfs = FilesystemReader::from_reader_with_offset(file, offset)?;

    let home_data = home_data_path();
    let data_path = Path::new(&home_data);

    for node in squashfs.files() {
        let node_path = node.fullpath.to_string_lossy();
        if !node_path.trim_start_matches("/").contains("/")
            && (node_path.ends_with(".DirIcon") || node_path.ends_with(".desktop"))
        {
            let extension = if node_path.ends_with(".DirIcon") {
                "png"
            } else {
                "desktop"
            };
            let output_path = file_path.with_extension(extension);
            match resolve_and_extract(&squashfs, node, &output_path, &mut HashSet::new()) {
                Ok(()) => {
                    if extension == "png" {
                        process_icon(&output_path, name, data_path).await?;
                    } else {
                        process_desktop(&output_path, name, data_path).await?;
                    }
                }
                Err(e) => eprintln!("Failed to extract {}: {}", node_path, e),
            }
        }
    }

    Ok(())
}

fn resolve_and_extract(
    squashfs: &FilesystemReader,
    node: &Node<SquashfsFileReader>,
    output_path: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<()> {
    match &node.inner {
        InnerNode::File(file) => extract_file(squashfs, file, output_path),
        InnerNode::Symlink(sym) => {
            let target_path = sym.link.clone();
            if !visited.insert(target_path.clone()) {
                return Err(anyhow::anyhow!(
                    "Uh oh. Bad symlink.. Infinite recursion detected..."
                ));
            }
            if let Some(target_node) = squashfs
                .files()
                .find(|n| n.fullpath.strip_prefix("/").unwrap() == target_path)
            {
                resolve_and_extract(squashfs, target_node, output_path, visited)
            } else {
                Err(anyhow::anyhow!("Symlink target not found"))
            }
        }
        _ => Err(anyhow::anyhow!("Unexpected node type")),
    }
}

fn extract_file(
    squashfs: &FilesystemReader,
    file: &SquashfsFileReader,
    output_path: &Path,
) -> Result<()> {
    let mut reader = squashfs.file(&file.basic).reader().bytes();
    let output_file = File::create(output_path)?;
    let mut buf_writer = BufWriter::new(output_file);
    while let Some(Ok(byte)) = reader.next() {
        buf_writer.write_all(&[byte])?;
    }
    Ok(())
}

async fn process_icon(output_path: &Path, name: &str, data_path: &Path) -> Result<()> {
    let image = image::open(output_path)?;
    let (orig_w, orig_h) = image.dimensions();

    let normalized_image = normalize_image(image);
    let (w, h) = normalized_image.dimensions();

    if (w, h) != (orig_w, orig_h) {
        normalized_image.save(output_path)?;
    }
    let final_path = data_path
        .join("icons")
        .join("hicolor")
        .join(format!("{}x{}", w, h))
        .join("apps")
        .join(name)
        .with_extension("png");

    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent).await.context(anyhow::anyhow!(
            "Failed to create icon directory at {}",
            parent.to_string_lossy()
        ))?;
    }
    create_symlink(output_path, &final_path).await?;
    Ok(())
}

async fn process_desktop(output_path: &Path, name: &str, data_path: &Path) -> Result<()> {
    let mut content = String::new();
    File::open(output_path)?.read_to_string(&mut content)?;

    let processed_content = content
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            if line.starts_with("Icon=") {
                format!("Icon={}", name)
            } else if line.starts_with("Exec=") {
                format!("Exec={}/{}", &*BIN_PATH.to_string_lossy(), name)
            } else if line.starts_with("TryExec=") {
                format!("TryExec={}/{}", &*BIN_PATH.to_string_lossy(), name)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n");

    let mut writer = BufWriter::new(File::create(output_path)?);
    writer.write_all(processed_content.as_bytes())?;

    let final_path = data_path
        .join("applications")
        .join(format!("soar-{name}"))
        .with_extension("desktop");

    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent).await.context(anyhow::anyhow!(
            "Failed to create desktop files directory at {}",
            parent.to_string_lossy()
        ))?;
    }

    create_symlink(output_path, &final_path).await?;
    Ok(())
}
