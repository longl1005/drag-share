#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn shell_quotes_untrusted_values() {
        assert_eq!(shell_quote("Photo's $(touch /tmp/no)\n&"), "'Photo'\\''s $(touch /tmp/no)\n&'");
    }
    #[test] fn applescript_quotes_shell_script() {
        assert_eq!(apple_script("echo \"a\\b\"\n"), "do shell script \"echo \\\"a\\\\b\\\"\\n\" with administrator privileges");
    }
    #[test] fn links_encode_share_segment_not_path() {
        assert_eq!(share_url("smb://192.168.1.2", "我的 电影&视频's/#?"), "smb://192.168.1.2/%E6%88%91%E7%9A%84%20%E7%94%B5%E5%BD%B1%26%E8%A7%86%E9%A2%91's%2F%23%3F");
    }
    #[test] fn canonical_directory_rejects_files_and_duplicate_symlinks() {
        let dir=tempfile::tempdir().unwrap();
        let file=dir.path().join("file"); std::fs::write(&file,"x").unwrap();
        assert!(canonical_directory(file.to_str().unwrap()).is_err());
        assert!(canonical_directory(dir.path().to_str().unwrap()).is_ok());
        assert!(canonical_directory("\0bad").is_err());
    }
}

use crate::error::{AppError,Result};
pub fn shell_quote(input: &str) -> String { format!("'{}'",input.replace('\'',"'\\''")) }
pub fn apple_script(script: &str) -> String {
    let escaped=script.replace('\\',"\\\\").replace('"',"\\\"").replace('\n',"\\n").replace('\r',"\\r");
    format!("do shell script \"{escaped}\" with administrator privileges")
}
pub fn canonical_directory(path: &str) -> Result<String> {
    if path.contains('\0') || !std::path::Path::new(path).is_absolute() { return Err(AppError::new("INVALID_PATH","请选择有效的绝对目录路径")); }
    let canonical=std::fs::canonicalize(path).map_err(|_| AppError::new("INVALID_PATH","目录不存在或无法访问，请检查文件夹权限"))?;
    if !canonical.is_dir() { return Err(AppError::new("NOT_DIRECTORY","仅支持添加文件夹目录")); }
    canonical.to_str().map(str::to_owned).ok_or_else(|| AppError::new("INVALID_PATH","目录名称无法编码"))
}
pub fn share_url(address: &str, name: &str) -> String {
    // Match encodeURIComponent, including its five unescaped punctuation characters.
    let mut encoded=String::new();
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) { encoded.push(b as char); }
        else { encoded.push_str(&format!("%{b:02X}")); }
    }
    format!("{}/{encoded}",address.trim_end_matches('/'))
}
