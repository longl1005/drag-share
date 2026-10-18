use super::{parser::{parse_shares,parse_status},SmbPlatform};
use crate::{error::{AppError,Result},model::{ShareItem,ServiceStatus},process,security::shell_quote};
pub struct MacOs;
const JOB:&str="system/com.apple.smbd";
async fn checked(program:&str,args:&[&str])->Result<String>{let out=process::read(program,args).await?;if out.success{Ok(out.stdout)}else{Err(process::classify_failure(&out.stderr))}}
async fn listening()->bool {
    tokio::task::spawn_blocking(||std::net::TcpStream::connect_timeout(&"127.0.0.1:445".parse().unwrap(),std::time::Duration::from_millis(350)).is_ok()).await.unwrap_or(false)
}
#[async_trait::async_trait]
impl SmbPlatform for MacOs {
    async fn get_service_status(&self)->Result<ServiceStatus>{
        let raw=checked("/bin/launchctl",&["print-disabled","system"]).await?;
        let loaded=process::read("/bin/launchctl",&["print",JOB]).await?.success;
        parse_status(&raw,loaded)
    }
    async fn enable_service(&self)->Result<()> {
        // The system can have the job loaded already. Test before bootstrap instead of swallowing failures.
        process::elevated("/bin/launchctl enable system/com.apple.smbd && { /bin/launchctl print system/com.apple.smbd >/dev/null 2>&1 || /bin/launchctl bootstrap system /System/Library/LaunchDaemons/com.apple.smbd.plist; } && /bin/launchctl kickstart -k system/com.apple.smbd").await?;
        for _ in 0..5 {if listening().await{return Ok(());}tokio::time::sleep(std::time::Duration::from_millis(150)).await;}
        process::elevated("/bin/launchctl kickstart -k system/com.apple.smbd").await?;
        for _ in 0..5 {if listening().await{return Ok(());}tokio::time::sleep(std::time::Duration::from_millis(150)).await;}
        Err(AppError::new("NOT_LISTENING","共享服务未开始接受连接，请前往系统共享设置检查 SMB 选项"))
    }
    async fn disable_service(&self)->Result<()> {
        process::elevated("/bin/launchctl disable system/com.apple.smbd && { if /bin/launchctl print system/com.apple.smbd >/dev/null 2>&1; then /bin/launchctl bootout system/com.apple.smbd; fi; }").await?;
        if self.get_service_status().await?!=ServiceStatus::Disabled || listening().await {return Err(AppError::new("VERIFY_FAILED","无法确认共享已关闭，请检查系统共享设置"));}Ok(())
    }
    async fn list_shares(&self)->Result<Vec<ShareItem>> {parse_shares(&checked("/usr/sbin/sharing",&["-l"]).await?)}
    async fn add_share(&self,path:&str,name:&str)->Result<()> {
        process::elevated(&format!("/usr/sbin/sharing -a {} -n {} -S {} -s 001",shell_quote(path),shell_quote(name),shell_quote(name))).await
    }
    async fn remove_share(&self,record_name:&str)->Result<()> {process::elevated(&format!("/usr/sbin/sharing -r {}",shell_quote(record_name))).await}
    async fn get_address(&self)->Result<(String,bool)> {
        // en0/en1 first per PRD, then other active interfaces (USB Ethernet, docking stations).
        let mut interfaces=vec!["en0".to_string(),"en1".to_string()];
        if let Ok(out)=process::read("/sbin/ifconfig",&["-l"]).await {for interface in out.stdout.split_whitespace().filter(|s|s.starts_with("en")){if !interfaces.iter().any(|s|s==interface){interfaces.push(interface.into());}}}
        for interface in interfaces {
            if let Ok(out)=process::read("/usr/sbin/ipconfig",&["getifaddr",&interface]).await {if out.success {if let Ok(ip)=out.stdout.trim().parse::<std::net::Ipv4Addr>() {if !ip.is_loopback() && !ip.is_unspecified() && !ip.is_link_local(){return Ok((format!("smb://{ip}"),false));}}}}
        }
        let host=checked("/usr/sbin/scutil",&["--get","LocalHostName"]).await?;
        let host=host.trim();if host.is_empty(){return Err(AppError::new("NO_ADDRESS","无法获取本机网络地址，请检查网络连接"));}
        Ok((format!("smb://{host}.local"),true))
    }
}
