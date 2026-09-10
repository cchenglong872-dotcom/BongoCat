use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

const MAX_AVATAR_SIZE: u64 = 20 * 1024 * 1024;
const AVATAR_DIR: &str = "chat/avatar";

const AVATAR_EXTENSIONS: [&str; 6] = ["png", "jpg", "webp", "gif", "bmp", "ico"];

fn image_extension(path: &Path) -> Result<&'static str, String> {
    let metadata = fs::metadata(path).map_err(|_| "avatarNotFound".to_string())?;

    if !metadata.is_file() {
        return Err("avatarNotFound".to_string());
    }

    if metadata.len() > MAX_AVATAR_SIZE {
        return Err("avatarTooLarge".to_string());
    }

    let bytes = fs::read(path).map_err(|_| "avatarReadFailed".to_string())?;

    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Ok("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Ok("jpg")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Ok("webp")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Ok("gif")
    } else if bytes.starts_with(b"BM") {
        Ok("bmp")
    } else if bytes.len() >= 4 && bytes[..4] == [0x00, 0x00, 0x01, 0x00] {
        Ok("ico")
    } else {
        Err("avatarInvalidType".to_string())
    }
}

fn avatar_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join(AVATAR_DIR))
        .map_err(|_| "avatarSaveFailed".to_string())
}

pub fn save<R: Runtime>(app: &AppHandle<R>, source_path: &str) -> Result<String, String> {
    let source = Path::new(source_path);
    let extension = image_extension(source)?;
    let dir = avatar_dir(app)?;

    fs::create_dir_all(&dir).map_err(|_| "avatarSaveFailed".to_string())?;

    let file_name = format!("user-avatar.{extension}");
    let target = dir.join(&file_name);
    let temporary = dir.join("user-avatar.tmp");

    fs::copy(source, &temporary).map_err(|_| "avatarSaveFailed".to_string())?;

    if target.exists() {
        fs::remove_file(&target).map_err(|_| "avatarSaveFailed".to_string())?;
    }

    fs::rename(&temporary, &target).map_err(|_| "avatarSaveFailed".to_string())?;

    for other_extension in AVATAR_EXTENSIONS {
        if other_extension != extension {
            let _ = fs::remove_file(dir.join(format!("user-avatar.{other_extension}")));
        }
    }

    Ok(format!("{AVATAR_DIR}/{file_name}"))
}

pub fn clear<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let dir = avatar_dir(app)?;

    for extension in AVATAR_EXTENSIONS {
        let path = dir.join(format!("user-avatar.{extension}"));

        if path.exists() {
            fs::remove_file(path).map_err(|_| "avatarRemoveFailed".to_string())?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_file(extension: &str, bytes: &[u8]) -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("bongo-avatar-{id}.{extension}"));
        fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn image_extension_should_detect_supported_headers() {
        let png = temporary_file("png", &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        let jpg = temporary_file("jpg", &[0xFF, 0xD8, 0xFF]);
        let webp = temporary_file("webp", b"RIFF0000WEBP");
        let gif = temporary_file("gif", b"GIF89a");
        let bmp = temporary_file("bmp", b"BM");
        let ico = temporary_file("ico", &[0x00, 0x00, 0x01, 0x00, 1, 0]);

        assert_eq!(image_extension(&png).unwrap(), "png");
        assert_eq!(image_extension(&jpg).unwrap(), "jpg");
        assert_eq!(image_extension(&webp).unwrap(), "webp");
        assert_eq!(image_extension(&gif).unwrap(), "gif");
        assert_eq!(image_extension(&bmp).unwrap(), "bmp");
        assert_eq!(image_extension(&ico).unwrap(), "ico");

        for path in [png, jpg, webp, gif, bmp, ico] {
            let _ = fs::remove_file(path);
        }
    }

    #[test]
    fn image_extension_should_reject_unknown_content() {
        let path = temporary_file("png", b"not an image");

        assert_eq!(image_extension(&path).unwrap_err(), "avatarInvalidType");

        let _ = fs::remove_file(path);
    }
}
