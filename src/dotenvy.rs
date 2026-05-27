use std::fs;
use std::env;
use std::path::{Path, PathBuf};

/// Loads environment variables from a `.env` file in the current directory.
/// Returns the path to the loaded file on success.
pub fn dotenv() -> Result<PathBuf, std::io::Error> {
    let path = Path::new(".env");
    if !path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File .env tidak ditemukan",
        ));
    }

    let content = fs::read_to_string(path)?;
    for line in content.lines() {
        let trimmed = line.trim();
        // Skip empty lines or comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(pos) = trimmed.find('=') {
            let key = trimmed[..pos].trim();
            let val = trimmed[pos + 1..].trim();

            // Strip optional quotes around value
            let clean_val = if (val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\''))
            {
                if val.len() >= 2 {
                    &val[1..val.len() - 1]
                } else {
                    ""
                }
            } else {
                val
            };

            unsafe {
                env::set_var(key, clean_val);
            }
        }
    }

    Ok(path.to_path_buf())
}
