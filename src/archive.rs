use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),

    #[error("Unsupported archive format: {0}")]
    UnsupportedFormat(String),
}

pub fn extract_zip(data: &[u8], dest: &Path) -> Result<(), ArchiveError> {
    let reader = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let Some(enclosed_name) = file.enclosed_name() else {
            continue;
        };
        let out_path = dest.join(enclosed_name);

        if file.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out_file = fs::File::create(&out_path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            out_file.write_all(&buf)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&out_path, fs::Permissions::from_mode(mode))?;
                }
            }
        }
    }

    Ok(())
}

pub fn extract_tar_gz(data: &[u8], dest: &Path) -> Result<(), ArchiveError> {
    let decoder = GzDecoder::new(Cursor::new(data));
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(dest)?;
    Ok(())
}

pub fn extract(data: &[u8], dest: &Path, filename: &str) -> Result<(), ArchiveError> {
    if filename.ends_with(".zip") {
        extract_zip(data, dest)
    } else if filename.ends_with(".tar.gz") {
        extract_tar_gz(data, dest)
    } else {
        Err(ArchiveError::UnsupportedFormat(filename.to_string()))
    }
}

pub fn find_nvim_binary(extracted_dir: &Path) -> Option<PathBuf> {
    find_nvim_binary_recursive(extracted_dir)
}

fn find_nvim_binary_recursive(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name == "nvim" || name == "nvim.exe" {
                    return Some(path);
                }
            }
        } else if path.is_dir() {
            subdirs.push(path);
        }
    }

    for subdir in subdirs {
        if let Some(found) = find_nvim_binary_recursive(&subdir) {
            return Some(found);
        }
    }

    None
}

/// Copies the entire directory tree from `src` into `dst`, creating directories as needed.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), ArchiveError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_extract_unsupported_format() {
        let tmp = std::env::temp_dir().join("pnm_test_unsupported_fmt");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let result = extract(b"", &tmp, "file.rar");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, ArchiveError::UnsupportedFormat(_)),
            "Expected UnsupportedFormat, got: {err:?}"
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_find_nvim_binary_empty_dir() {
        let tmp = std::env::temp_dir().join("pnm_test_find_empty");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let result = find_nvim_binary(&tmp);
        assert!(result.is_none());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_find_nvim_binary_finds_exe() {
        let tmp = std::env::temp_dir().join("pnm_test_find_exe");
        let _ = fs::remove_dir_all(&tmp);

        let bin_dir = tmp.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        let binary_name = if cfg!(windows) { "nvim.exe" } else { "nvim" };
        let binary_path = bin_dir.join(binary_name);
        fs::write(&binary_path, b"fake nvim").unwrap();

        let found = find_nvim_binary(&tmp);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), binary_path);

        let _ = fs::remove_dir_all(&tmp);
    }
}

pub fn install_nvim_binary(
    extracted_dir: &Path,
    instance_bin_dir: &Path,
) -> Result<PathBuf, ArchiveError> {
    let nvim_binary = find_nvim_binary(extracted_dir).ok_or_else(|| {
        ArchiveError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "nvim binary not found in extracted archive",
        ))
    })?;

    // Find the top-level extracted directory (e.g. "nvim-win64" or "nvim-linux-x86_64").
    // This is the first directory entry inside extracted_dir.
    let top_level_dir = fs::read_dir(extracted_dir)?
        .filter_map(|e| e.ok())
        .find(|e| e.path().is_dir())
        .map(|e| e.path())
        .unwrap_or_else(|| extracted_dir.to_path_buf());

    copy_dir_recursive(&top_level_dir, instance_bin_dir)?;

    // Compute the relative path from the top-level dir to the binary, then map it
    // into instance_bin_dir so we return the correct final location.
    let relative = nvim_binary
        .strip_prefix(&top_level_dir)
        .unwrap_or(nvim_binary.as_path());
    let installed_binary = instance_bin_dir.join(relative);

    Ok(installed_binary)
}
