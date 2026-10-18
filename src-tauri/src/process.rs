#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn canceled_is_distinct_from_permission_denied() {
        assert_eq!(classify_failure("execution error: User canceled. (-128)").code,"USER_CANCELED");
        assert_eq!(classify_failure("Not authorized (-1743)").code,"PERMISSION_DENIED");
        assert_eq!(classify_failure("Unexpected failure").code,"COMMAND_FAILED");
    }
    #[tokio::test] async fn timeout_kills_subprocess() {
        let err=run_with_timeout("/bin/sleep", &["2"],std::time::Duration::from_millis(20)).await.unwrap_err();
        assert_eq!(err.code,"TIMEOUT");
    }
}

use crate::{error::{AppError,Result},security::apple_script};
use std::{process::Stdio,time::Duration};
use tokio::io::AsyncReadExt;
#[derive(Debug)]
pub struct CommandOutput { pub success: bool, pub stdout: String, pub stderr: String }
pub fn classify_failure(stderr:&str)->AppError {
    if stderr.contains("(-128)") {AppError::new("USER_CANCELED","操作已取消")}
    else if stderr.to_lowercase().contains("not authorized") || stderr.to_lowercase().contains("permission denied") || stderr.contains("(-1743)") {AppError::new("PERMISSION_DENIED","权限不足，请在系统共享设置中检查授权")}
    else {AppError::new("COMMAND_FAILED","系统共享操作未完成，请重试或检查系统共享设置")}
}
pub async fn run_with_timeout(program:&str,args:&[&str],duration:Duration)->Result<CommandOutput> {
    let mut child=tokio::process::Command::new(program).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true).spawn().map_err(AppError::io)?;
    let mut stdout=child.stdout.take().unwrap();let mut stderr=child.stderr.take().unwrap();
    // Drain both pipes concurrently, bounded so unexpected tool output cannot exhaust memory.
    let out=tokio::spawn(async move {let mut b=Vec::new();let _=(&mut stdout).take(1024*1024).read_to_end(&mut b).await;b});
    let err=tokio::spawn(async move {let mut b=Vec::new();let _=(&mut stderr).take(1024*1024).read_to_end(&mut b).await;b});
    let status=match tokio::time::timeout(duration,child.wait()).await {
        Ok(s)=>s.map_err(AppError::io)?,
        Err(_)=>{let _=child.kill().await;out.abort();err.abort();return Err(AppError::new("TIMEOUT","操作超时，请重试"));}
    };
    Ok(CommandOutput{success:status.success(),stdout:String::from_utf8_lossy(&out.await.unwrap_or_default()).into(),stderr:String::from_utf8_lossy(&err.await.unwrap_or_default()).into()})
}
pub async fn read(program:&str,args:&[&str])->Result<CommandOutput> {run_with_timeout(program,args,Duration::from_secs(10)).await}
pub async fn elevated(script:&str)->Result<()> {
    let script=apple_script(script);
    let out=run_with_timeout("/usr/bin/osascript",&["-e",&script],Duration::from_secs(30)).await?;
    if out.success {Ok(())}else{Err(classify_failure(&out.stderr))}
}
