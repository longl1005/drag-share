#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ServiceStatus;
    #[test] fn parses_smb_fields_separately_from_record_name() {
        let raw="List of Share Points\nname: Record One\npath: /tmp/My Folder\nsmb: {\n name: 我的 视频\n shared: 1\n guest access: 0\n read-only: 1\n}\nname: Disabled\npath: /tmp/off\nsmb: {\n name: Off\n shared: 0\n guest access: 1\n read-only: 0\n}\n";
        let shares=parse_shares(raw).unwrap(); assert_eq!(shares.len(),1);
        assert_eq!(shares[0].id,"Record One"); assert_eq!(shares[0].name,"我的 视频");
        assert_eq!(shares[0].path,"/tmp/My Folder"); assert!(shares[0].read_only); assert!(!shares[0].guest_access);
    }
    #[test] fn empty_is_not_a_parse_error_but_garbage_is() {
        assert!(parse_shares("List of Share Points\n").unwrap().is_empty());
        assert!(parse_shares("Unable to query share point records.").is_err());
        assert!(parse_shares("name: incomplete\n").is_err());
    }
    #[test] fn disabled_override_wins_over_loaded_state() {
        assert_eq!(parse_status("\"com.apple.smbd\" => disabled",true).unwrap(),ServiceStatus::Disabled);
        assert_eq!(parse_status("\"com.apple.smbd\" => enabled",false).unwrap(),ServiceStatus::Enabled);
        assert_eq!(parse_status("disabled services = { }",true).unwrap(),ServiceStatus::Enabled);
        assert_eq!(parse_status("disabled services = { }",false).unwrap(),ServiceStatus::Disabled);
        assert!(parse_status("not launchctl output",false).is_err());
    }
}

use crate::{error::{AppError,Result}, model::{ShareItem,ServiceStatus}};
pub fn parse_status(raw: &str, loaded: bool) -> Result<ServiceStatus> {
    for line in raw.lines().filter(|l| l.contains("\"com.apple.smbd\"")) {
        let value=line.split("=>").nth(1).unwrap_or("").trim().trim_end_matches(';');
        return match value { "disabled"|"true"=>Ok(ServiceStatus::Disabled),"enabled"|"false"=>Ok(ServiceStatus::Enabled),_=>Err(AppError::new("PARSE_ERROR","无法识别系统共享服务状态")) };
    }
    if !raw.contains("disabled services") { return Err(AppError::new("PARSE_ERROR","无法读取系统共享服务状态")); }
    Ok(if loaded {ServiceStatus::Enabled} else {ServiceStatus::Disabled})
}
pub fn parse_shares(raw: &str) -> Result<Vec<ShareItem>> {
    if !raw.contains("List of Share Points") { return Err(AppError::new("PARSE_ERROR","无法解析系统共享目录，请在系统共享设置中检查配置")); }
    let mut shares=Vec::new(); let mut record=String::new(); let mut path=String::new();
    let mut smb=false; let mut name=String::new(); let mut shared=false; let mut guest=false; let mut readonly=false;
    let finish=|shares:&mut Vec<ShareItem>,record:&str,path:&str,name:&str,shared:bool,guest:bool,readonly:bool|->Result<()> {
        if record.is_empty() {return Ok(());}
        if path.is_empty() { return Err(AppError::new("PARSE_ERROR","共享目录记录缺少路径")); }
        if shared { shares.push(ShareItem{id:record.into(),name:if name.is_empty(){record.into()}else{name.into()},path:path.into(),read_only:readonly,guest_access:guest,exists:std::path::Path::new(path).is_dir()}); }
        Ok(())
    };
    for line in raw.lines() {
        let line=line.trim();
        if line=="smb:" || line.starts_with("smb:") { smb=true; continue; }
        if line=="}" {smb=false;continue;}
        let Some((key,val))=line.split_once(':') else {continue;}; let val=val.trim();
        match (smb,key.trim()) {
            (false,"name")=>{ finish(&mut shares,&record,&path,&name,shared,guest,readonly)?; record=val.into();path.clear();name.clear();shared=false;guest=false;readonly=false; },
            (false,"path")=>path=val.into(),
            (true,"name")=>name=val.into(),(true,"shared")=>shared=val=="1",(true,"guest access")=>guest=val=="1",(true,"read-only")=>readonly=val=="1",_=>{}
        }
    }
    finish(&mut shares,&record,&path,&name,shared,guest,readonly)?;
    Ok(shares)
}
