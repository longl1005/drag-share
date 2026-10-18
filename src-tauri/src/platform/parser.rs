#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ServiceStatus;
    #[test]
    fn parses_smb_fields_separately_from_record_name() {
        let raw="List of Share Points\nname: Record One\npath: /tmp/My Folder\nsmb: {\n name: 我的 视频\n shared: 1\n guest access: 0\n read-only: 1\n}\nname: Disabled\npath: /tmp/off\nsmb: {\n name: Off\n shared: 0\n guest access: 1\n read-only: 0\n}\n";
        let shares = parse_shares(raw).unwrap();
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].id, "Record One");
        assert_eq!(shares[0].name, "我的 视频");
        assert_eq!(shares[0].path, "/tmp/My Folder");
        assert!(shares[0].read_only);
        assert!(!shares[0].guest_access);
    }
    #[test]
    fn empty_is_not_a_parse_error_but_garbage_is() {
        assert!(parse_shares("List of Share Points\n").unwrap().is_empty());
        assert!(parse_shares("Unable to query share point records.").is_err());
        assert!(parse_shares("name: incomplete\n").is_err());
    }
    #[test]
    fn conflicting_override_and_loaded_state_are_never_reported_as_safe() {
        assert!(parse_status("\"com.apple.smbd\" => disabled", true).is_err());
        assert!(parse_status("\"com.apple.smbd\" => enabled", false).is_err());
        assert_eq!(
            parse_status("\"com.apple.smbd\" => disabled", false).unwrap(),
            ServiceStatus::Disabled
        );
        assert_eq!(
            parse_status("\"com.apple.smbd\" => enabled", true).unwrap(),
            ServiceStatus::Enabled
        );
        assert_eq!(
            parse_status("disabled services = { }", true).unwrap(),
            ServiceStatus::Enabled
        );
        assert_eq!(
            parse_status("disabled services = { }", false).unwrap(),
            ServiceStatus::Disabled
        );
        assert!(parse_status("not launchctl output", false).is_err());
    }
}

use crate::{
    error::{AppError, Result},
    model::{ServiceStatus, ShareItem},
};
pub fn parse_status(raw: &str, loaded: bool) -> Result<ServiceStatus> {
    if let Some(line) = raw.lines().find(|l| l.contains("\"com.apple.smbd\"")) {
        let value = line
            .split("=>")
            .nth(1)
            .unwrap_or("")
            .trim()
            .trim_end_matches(';');
        return match value {
            "disabled" | "true" if !loaded => Ok(ServiceStatus::Disabled),
            "enabled" | "false" if loaded => Ok(ServiceStatus::Enabled),
            "disabled" | "true" | "enabled" | "false" => Err(AppError::new(
                "STATUS_CONFLICT",
                "系统共享开关与服务加载状态不一致，请检查系统共享设置",
            )),
            _ => Err(AppError::new("PARSE_ERROR", "无法识别系统共享服务状态")),
        };
    }
    if !raw.contains("disabled services") {
        return Err(AppError::new("PARSE_ERROR", "无法读取系统共享服务状态"));
    }
    Ok(if loaded {
        ServiceStatus::Enabled
    } else {
        ServiceStatus::Disabled
    })
}
pub fn parse_shares(raw: &str) -> Result<Vec<ShareItem>> {
    if !raw.contains("List of Share Points") {
        return Err(AppError::new(
            "PARSE_ERROR",
            "无法解析系统共享目录，请在系统共享设置中检查配置",
        ));
    }
    let mut shares = Vec::new();
    let mut record = String::new();
    let mut path = String::new();
    let mut smb = false;
    let mut name = String::new();
    let mut shared = false;
    let mut guest = false;
    let mut readonly = false;
    let finish = |shares: &mut Vec<ShareItem>,
                  record: &str,
                  path: &str,
                  name: &str,
                  shared: bool,
                  guest: bool,
                  readonly: bool|
     -> Result<()> {
        if record.is_empty() {
            return Ok(());
        }
        if path.is_empty() {
            return Err(AppError::new("PARSE_ERROR", "共享目录记录缺少路径"));
        }
        if shared {
            shares.push(ShareItem {
                id: record.into(),
                name: if name.is_empty() {
                    record.into()
                } else {
                    name.into()
                },
                path: path.into(),
                read_only: readonly,
                guest_access: guest,
                exists: std::path::Path::new(path).is_dir(),
            });
        }
        Ok(())
    };
    for line in raw.lines() {
        let line = line.trim();
        if line == "smb:" || line.starts_with("smb:") {
            smb = true;
            continue;
        }
        if line == "}" {
            smb = false;
            continue;
        }
        let Some((key, val)) = line.split_once(':') else {
            continue;
        };
        let val = val.trim();
        match (smb, key.trim()) {
            (false, "name") => {
                finish(&mut shares, &record, &path, &name, shared, guest, readonly)?;
                record = val.into();
                path.clear();
                name.clear();
                shared = false;
                guest = false;
                readonly = false;
            }
            (false, "path") => path = val.into(),
            (true, "name") => name = val.into(),
            (true, "shared") => shared = val == "1",
            (true, "guest access") => guest = val == "1",
            (true, "read-only") => readonly = val == "1",
            _ => {}
        }
    }
    finish(&mut shares, &record, &path, &name, shared, guest, readonly)?;
    Ok(shares)
}

#[cfg(test)]
mod json_tests {
    use super::*;
    #[test]
    fn json_preserves_special_names_and_rejects_missing_permission_data() {
        let raw = r#"{"record's":{"path":"/tmp/ a\nb ","smb_name":"影片 \"2026\"\n ","smb_shared":1,"smb_guest_access":0,"smb_read_only":1},"afp":{"path":"/old","smb_shared":0}}"#;
        let shares = parse_json_shares(raw).unwrap();
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].name, "影片 \"2026\"\n ");
        assert_eq!(shares[0].path, "/tmp/ a\nb ");
        assert!(shares[0].read_only);
        assert!(parse_json_shares(r#"{"broken":{"path":"/tmp","smb_shared":1}}"#).is_err());
        assert!(parse_json_shares("{}").unwrap().is_empty());
    }
}

pub fn parse_json_shares(raw: &str) -> Result<Vec<ShareItem>> {
    let bad = || AppError::new("PARSE_ERROR", "无法解析系统共享目录，请检查系统共享设置");
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|_| bad())?;
    let records = value.as_object().ok_or_else(bad)?;
    let mut shares = vec![];
    for (id, data) in records {
        if data.get("smb_shared").and_then(|v| v.as_i64()) == Some(0) {
            continue;
        }
        if data.get("smb_shared").and_then(|v| v.as_i64()) != Some(1) {
            return Err(bad());
        }
        let path = data
            .get("path")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(bad)?;
        let name = data
            .get("smb_name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(bad)?;
        let flag = |field: &str| -> Result<bool> {
            match data.get(field).and_then(|v| v.as_i64()) {
                Some(0) => Ok(false),
                Some(1) => Ok(true),
                _ => Err(bad()),
            }
        };
        shares.push(ShareItem {
            id: id.clone(),
            name: name.into(),
            path: path.into(),
            read_only: flag("smb_read_only")?,
            guest_access: flag("smb_guest_access")?,
            exists: std::path::Path::new(path).is_dir(),
        });
    }
    shares.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(shares)
}
