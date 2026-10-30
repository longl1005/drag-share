//! Windows PowerShell 5.1 backend. User values are literals; scripts travel as UTF-16 base64.
use super::SmbPlatform;
use crate::{
    error::{AppError, Result},
    model::{ServiceStatus, ShareItem},
    process,
};
use std::time::Duration;
pub struct Windows;
fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
fn encoded(script: &str) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        for i in 0..4 {
            out.push(if i > chunk.len() {
                '='
            } else {
                TABLE[((n >> (18 - i * 6)) & 63) as usize] as char
            });
        }
    }
    out
}
const PRELUDE: &str =
    "$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); ";
// Stable numeric errors survive localization and the elevated process boundary.
const CATCH: &str = " catch { $e=$_.Exception; while ($null -ne $e) { if ($e.NativeErrorCode -eq 1223 -or ($e.HResult -band 65535) -eq 1223) { exit 20 }; if ($e.NativeErrorCode -eq 5) { exit 21 }; $e=$e.InnerException }; if ($_.CategoryInfo.Category -eq 'PermissionDenied') { exit 21 }; exit 22 }";
fn wrapped(script: &str) -> String {
    format!("{PRELUDE}try {{ {script} }}{CATCH}")
}
fn elevation(script: &str) -> String {
    format!("$p=Start-Process -FilePath \"$PSHOME\\powershell.exe\" -Verb RunAs -WindowStyle Hidden -ArgumentList '-NoProfile -NonInteractive -EncodedCommand {}' -Wait -PassThru; exit $p.ExitCode", encoded(&wrapped(script)))
}
fn failure(code: i32) -> AppError {
    match code {
        20 => AppError::new("USER_CANCELED", "操作已取消"),
        21 => AppError::new(
            "PERMISSION_DENIED",
            "权限不足，请检查 Windows 管理员授权及共享权限",
        ),
        _ => AppError::new(
            "COMMAND_FAILED",
            "Windows 共享操作失败，请检查 Server 服务、SmbShare 模块及系统共享设置",
        ),
    }
}
async fn run(script: &str, elevated: bool) -> Result<String> {
    let script = if elevated {
        elevation(script)
    } else {
        script.into()
    };
    let code = encoded(&wrapped(&script));
    let out = process::run_with_timeout(
        "powershell.exe",
        &[
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &code,
        ],
        Duration::from_secs(if elevated { 120 } else { 15 }),
    )
    .await?;
    if out.success {
        Ok(out.stdout)
    } else {
        Err(failure(out.code.unwrap_or(22)))
    }
}
const LIST: &str = r#"$items=@(Get-SmbShare | Where-Object { -not $_.Special -and $_.Path } | ForEach-Object {
$s=$_; $acl=@(Get-SmbShareAccess -Name $s.Name); $allow=@($acl | Where-Object AccessControlType -eq Allow);
[pscustomobject]@{id=$s.Name; name=$s.Name; path=$s.Path; readOnly=(@($allow | Where-Object { $_.AccessRight -ne 'Read' }).Count -eq 0); guestAccess=(@($allow | Where-Object { try { $sid=([Security.Principal.NTAccount]::new($_.AccountName)).Translate([Security.Principal.SecurityIdentifier]).Value } catch { $sid='' }; $sid -eq 'S-1-5-7' -or $sid -match '-501$' }).Count -gt 0); exists=(Test-Path -LiteralPath $s.Path -PathType Container)}
}); ConvertTo-Json -InputObject $items -Compress"#;
// LanmanServer's CIM provider can be unavailable while stopped. Read persistent
// share records and security descriptors without starting the service or elevating.
const STOPPED_LIST: &str = r#"
$key=Get-Item 'HKLM:\SYSTEM\CurrentControlSet\Services\LanmanServer\Shares' -ErrorAction SilentlyContinue;
if (-not $key) { '[]'; return };
$security=Get-Item 'HKLM:\SYSTEM\CurrentControlSet\Services\LanmanServer\Shares\Security' -ErrorAction SilentlyContinue;
$items=@(foreach ($name in $key.GetValueNames()) {
    if ($name -match '^(IPC\$|ADMIN\$|[A-Z]\$)$') { continue }
    $path=@($key.GetValue($name) | Where-Object { $_ -like 'Path=*' } | Select-Object -First 1);
    if ($path.Count -eq 0) { continue }; $path=$path[0].Substring(5);
    $allow=@(); $descriptor=if ($security) { $security.GetValue($name) } else { $null };
    if ($descriptor) {
        $sd=[Security.AccessControl.RawSecurityDescriptor]::new([byte[]]$descriptor,0);
        $allow=@($sd.DiscretionaryAcl | Where-Object { $_.AceQualifier -eq 'AccessAllowed' });
    }
    [pscustomobject]@{id=$name; name=$name; path=$path; readOnly=(@($allow | Where-Object { ($_.AccessMask -band 2) -ne 0 }).Count -eq 0); guestAccess=(@($allow | Where-Object { $_.SecurityIdentifier.Value -eq 'S-1-5-7' -or $_.SecurityIdentifier.Value -match '-501$' }).Count -gt 0); exists=(Test-Path -LiteralPath $path -PathType Container)}
}); ConvertTo-Json -InputObject $items -Compress
"#;
fn parse_shares(raw: &str) -> Result<Vec<ShareItem>> {
    serde_json::from_str(raw.trim_start_matches('\u{feff}').trim())
        .map_err(|_| AppError::new("PARSE_ERROR", "无法解析 Windows 共享列表"))
}
fn add_script(path: &str, name: &str, account: &str) -> Result<String> {
    if name.is_empty()
        || name.encode_utf16().count() > 80
        || name
            .chars()
            .any(|c| c.is_control() || "\\/[]:|<>+=;,?*\"".contains(c))
        || matches!(name.to_uppercase().as_str(), "IPC$" | "ADMIN$")
        || (name.len() == 2 && name.as_bytes()[0].is_ascii_alphabetic() && name.ends_with('$'))
    {
        return Err(AppError::new(
            "INVALID_NAME",
            "目录名称不适合作为 Windows 共享名称，请重命名目录",
        ));
    }
    Ok(format!(
        "New-SmbShare -Name {} -Path {} -ChangeAccess {} | Out-Null",
        literal(name),
        literal(path),
        literal(account)
    ))
}
#[async_trait::async_trait]
impl SmbPlatform for Windows {
    async fn get_service_status(&self) -> Result<ServiceStatus> {
        match run("(Get-Service LanmanServer).Status.ToString()", false)
            .await?
            .trim()
        {
            "Running" => Ok(ServiceStatus::Enabled),
            "Stopped" => Ok(ServiceStatus::Disabled),
            _ => Ok(ServiceStatus::Unknown),
        }
    }
    async fn enable_service(&self) -> Result<()> {
        run("Set-Service LanmanServer -StartupType Automatic; Start-Service LanmanServer; (Get-Service LanmanServer).WaitForStatus('Running',[TimeSpan]::FromSeconds(15))", true).await?;
        Ok(())
    }
    async fn disable_service(&self) -> Result<()> {
        // Do not force-stop dependent services or delete persistent shares.
        run("Stop-Service LanmanServer; (Get-Service LanmanServer).WaitForStatus('Stopped',[TimeSpan]::FromSeconds(15)); Set-Service LanmanServer -StartupType Disabled", true).await?;
        Ok(())
    }
    async fn list_shares(&self) -> Result<Vec<ShareItem>> {
        let script = if self.get_service_status().await? == ServiceStatus::Disabled {
            STOPPED_LIST
        } else {
            LIST
        };
        match run(script, false).await {
            Ok(raw) => parse_shares(&raw),
            // Some Windows editions restrict the CIM provider to administrators.
            // Persistent local records remain readable without repeated UAC prompts.
            Err(_) if script == LIST => parse_shares(&run(STOPPED_LIST, false).await?),
            Err(e) => Err(e),
        }
    }
    async fn add_share(&self, path: &str, name: &str) -> Result<()> {
        // Capture caller identity BEFORE UAC (alternate administrator credentials may be used).
        let account = run(
            "[Security.Principal.WindowsIdentity]::GetCurrent().Name",
            false,
        )
        .await?;
        let script = add_script(path, name, account.trim())?;
        run(&script, true).await?;
        Ok(())
    }
    async fn remove_share(&self, name: &str) -> Result<()> {
        let n = literal(name);
        run(&format!("$s=Get-SmbShare -Name {n}; if ($s.Special) {{ throw 'Protected system share' }}; Remove-SmbShare -Name {n} -Force -Confirm:$false"), true).await?;
        Ok(())
    }
    async fn get_address(&self) -> Result<(String, bool)> {
        let raw = run("Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.AddressState -eq 'Preferred' -and $_.IPAddress -notmatch '^(127\\.|169\\.254\\.|0\\.)' } | Sort-Object InterfaceMetric | Select-Object -First 1 -ExpandProperty IPAddress", false).await?;
        if let Ok(ip) = raw.trim().parse::<std::net::Ipv4Addr>() {
            return Ok((format!("smb://{ip}"), false));
        }
        let host = run("[Environment]::MachineName", false).await?;
        if host.trim().is_empty() {
            return Err(AppError::new("NO_ADDRESS", "无法获取本机网络地址"));
        }
        Ok((format!("smb://{}", host.trim()), true))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf16_base64_vectors() {
        assert_eq!(encoded(""), "");
        assert_eq!(encoded("A"), "QQA=");
        assert_eq!(encoded("AB"), "QQBCAA==");
        assert_eq!(encoded("中文"), "LU6HZQ==");
    }
    #[test]
    fn literals_prevent_injection() {
        assert_eq!(literal("a';&$()\n中"), "'a'';&$()\n中'");
        let s = add_script("C:\\Photo's $x", "Photo's $x", "PC\\user").unwrap();
        assert!(s.contains("'C:\\Photo''s $x'"));
        assert!(s.contains("-ChangeAccess 'PC\\user'"));
        assert!(!s.contains("Everyone"));
    }
    #[test]
    fn names_reject_windows_reserved_characters() {
        for n in ["", "a/b", "a:b", "IPC$", "a\n"] {
            assert!(add_script("C:\\x", n, "u").is_err());
        }
    }
    #[test]
    fn fixtures_cover_empty_missing_unicode_and_acl_flags() {
        assert!(parse_shares("[]").unwrap().is_empty());
        let s=parse_shares(r#"[{"id":"中文","name":"中文","path":"C:\\照片","readOnly":true,"guestAccess":false,"exists":false}]"#).unwrap();
        assert_eq!(s[0].name, "中文");
        assert!(s[0].read_only);
        assert!(!s[0].exists);
        assert!(parse_shares("broken").is_err());
    }
    #[test]
    fn elevation_is_encoded_and_errors_distinct() {
        let s = elevation("throw 'x'");
        assert!(s.contains("-Verb RunAs"));
        assert!(!s.contains("throw 'x'"));
        assert_eq!(failure(20).code, "USER_CANCELED");
        assert_eq!(failure(21).code, "PERMISSION_DENIED");
        assert_eq!(failure(22).code, "COMMAND_FAILED");
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn powershell_wrapper_roundtrips_unicode_and_exit_codes() {
        assert_eq!(
            run("'照片 & 空格'", false).await.unwrap().trim(),
            "照片 & 空格"
        );
        assert_eq!(
            run("exit 20", false).await.unwrap_err().code,
            "USER_CANCELED"
        );
        assert_eq!(
            run("exit 21", false).await.unwrap_err().code,
            "PERMISSION_DENIED"
        );
        assert_eq!(
            run("throw 'fixture'", false).await.unwrap_err().code,
            "COMMAND_FAILED"
        );
        assert_eq!(run("throw [InvalidOperationException]::new('localized', [ComponentModel.Win32Exception]::new(1223))", false).await.unwrap_err().code, "USER_CANCELED");
        assert_eq!(
            run("throw [ComponentModel.Win32Exception]::new(5)", false)
                .await
                .unwrap_err()
                .code,
            "PERMISSION_DENIED"
        );
    }
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn powershell_list_uses_mock_cmdlets_without_touching_smb() {
        let mock = r#"
function Get-SmbShare { [pscustomobject]@{Name='照片'; Path='C:\missing'; Special=$false}; [pscustomobject]@{Name='IPC$';Path='';Special=$true} }
function Get-SmbShareAccess { param($Name) [pscustomobject]@{AccountName='unresolved-fixture'; AccessControlType='Allow'; AccessRight='Read'} }
function Test-Path { param($LiteralPath,$PathType) $false }
"#;
        let shares = parse_shares(&run(&format!("{mock}; {LIST}"), false).await.unwrap()).unwrap();
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].name, "照片");
        assert!(shares[0].read_only);
        assert!(!shares[0].exists);
        assert!(!shares[0].guest_access);
        let mock = "function Get-SmbShare {}";
        assert!(
            parse_shares(&run(&format!("{mock}; {LIST}"), false).await.unwrap())
                .unwrap()
                .is_empty()
        );
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn stopped_service_registry_fixture_keeps_missing_folder_records() {
        let mock = r#"
function Get-Item {
    param($Path,$ErrorAction)
    if ($Path.EndsWith('\Security')) { return $null }
    $key=[pscustomobject]@{};
    $key | Add-Member ScriptMethod GetValueNames { @('IPC$', '照片') };
    $key | Add-Member ScriptMethod GetValue { param($name) @('Path=C:\missing', 'Type=0') };
    $key
}
function Test-Path { param($LiteralPath,$PathType) $false }
"#;
        let shares = parse_shares(
            &run(&format!("{mock}; {STOPPED_LIST}"), false)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].name, "照片");
        assert!(!shares[0].exists);
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn mock_elevation_preserves_cancel_and_child_failure_without_uac() {
        for exit in [0, 20, 21, 22] {
            let mock = format!("function Start-Process {{ param($FilePath,$Verb,$WindowStyle,$ArgumentList,[switch]$Wait,[switch]$PassThru) [pscustomobject]@{{ExitCode={exit}}} }};");
            let result = run(
                &format!("{mock} {}", elevation("throw 'must not execute'")),
                false,
            )
            .await;
            if exit == 0 {
                assert!(result.is_ok());
            } else {
                assert_eq!(result.unwrap_err().code, failure(exit).code);
            }
        }
    }
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn mocked_new_share_receives_literal_unicode_path_and_original_account() {
        let mock = "function New-SmbShare { param($Name,$Path,$ChangeAccess) if ($Name -ne \"照片's `$x\" -or $Path -ne \"C:\\照片's `$x\" -or $ChangeAccess -ne 'PC\\user') { throw 'arguments changed' } };";
        let script = add_script("C:\\照片's $x", "照片's $x", "PC\\user").unwrap();
        run(&format!("{mock} {script}"), false).await.unwrap();
    }
}
