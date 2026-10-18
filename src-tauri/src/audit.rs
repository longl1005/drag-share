use serde_json::Value;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};
pub fn append_at(path: &Path, value: &Value, limit: u64) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if fs::metadata(path)
        .map(|m| m.len() >= limit)
        .unwrap_or(false)
    {
        let oldest = path.with_extension("jsonl.3");
        if oldest.exists() {
            fs::remove_file(oldest)?;
        }
        for i in (1..3).rev() {
            let src = path.with_extension(format!("jsonl.{i}"));
            if src.exists() {
                fs::rename(src, path.with_extension(format!("jsonl.{}", i + 1)))?;
            }
        }
        fs::rename(path, path.with_extension("jsonl.1"))?;
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    serde_json::to_writer(&mut file, value)?;
    file.write_all(b"\n")?;
    Ok(())
}
pub fn record(
    action: &str,
    target: Option<&str>,
    error: Option<&crate::error::AppError>,
    duration_ms: u128,
) {
    if cfg!(test) {
        return;
    }
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    #[cfg(target_os = "macos")]
    let base = std::path::PathBuf::from(home).join("Library/Logs/SMBShare");
    #[cfg(not(target_os = "macos"))]
    let base = std::path::PathBuf::from(home).join(".local/state/SMBShare");
    let result = match error {
        None => "success",
        Some(e) if e.canceled() => "canceled",
        _ => "error",
    };
    let value = serde_json::json!({"ts":chrono::Local::now().to_rfc3339(),"action":action,"target":target,"result":result,"error_code":error.map(|e|&e.code),"duration_ms":duration_ms});
    if let Err(e) = append_at(&base.join("ops.jsonl"), &value, 5 * 1024 * 1024) {
        eprintln!("Local audit unavailable: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotates_and_keeps_three_archives() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("ops.jsonl");
        for _ in 0..6 {
            append_at(&p, &serde_json::json!({"action":"copy_address"}), 1).unwrap();
        }
        assert!(p.exists());
        assert!(p.with_extension("jsonl.3").exists());
        assert!(!p.with_extension("jsonl.4").exists());
    }
}
