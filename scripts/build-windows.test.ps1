# Packaging fixture: no Rust build, installer execution, SMB changes or UAC.
$ErrorActionPreference = 'Stop'
$original = Get-Location
$fixture = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString())
$repo = Join-Path $PSScriptRoot '..'
try {
    New-Item "$fixture/scripts" -ItemType Directory -Force | Out-Null
    Copy-Item "$repo/scripts/build-windows.ps1", "$repo/scripts/sha256sums.mjs" "$fixture/scripts"
    Copy-Item "$repo/package.json" "$fixture/package.json"
    $version = (Get-Content "$fixture/package.json" -Raw | ConvertFrom-Json).version
    $output = "$fixture/src-tauri/target/x86_64-pc-windows-msvc/release"
    New-Item "$output/bundle/nsis" -ItemType Directory -Force | Out-Null
    Set-Content "$output/drag-share.exe" 'executable fixture'
    Set-Content "$output/bundle/nsis/SMB 共享助手_${version}_x64-setup.exe" 'installer fixture'
    $global:dragShareMockExit = 0
    function npm { $global:LASTEXITCODE = $global:dragShareMockExit }
    & "$fixture/scripts/build-windows.ps1"
    $installer = "$fixture/release/drag-share-$version-windows-x64-setup.exe"
    $zip = "$fixture/release/drag-share-$version-windows-x64.zip"
    if (!(Test-Path $installer) -or !(Test-Path $zip)) { throw 'Missing named packages' }
    $sums = @(Get-Content "$fixture/release/SHA256SUMS.txt")
    if ($sums.Count -ne 2) { throw 'Expected checksums for both packages' }
    foreach ($file in @($installer, $zip)) {
        $expected = (Get-FileHash $file -Algorithm SHA256).Hash.ToLowerInvariant() + '  ' + [IO.Path]::GetFileName($file)
        if ($sums -notcontains $expected) { throw 'Checksum mismatch' }
    }
    $archive = [IO.Compression.ZipFile]::OpenRead($zip)
    try {
        if ($archive.Entries.Count -ne 1 -or $archive.Entries[0].Name -ne 'drag-share.exe') { throw 'Wrong ZIP contents' }
    } finally { $archive.Dispose() }
    $global:dragShareMockExit = 7
    $rejected = $false
    try { & "$fixture/scripts/build-windows.ps1" } catch { $rejected = $_.Exception.Message -eq 'Windows build failed' }
    if (!$rejected) { throw 'Build failure was not propagated' }
    Write-Output 'Windows packaging fixture passed'
} finally {
    Set-Location $original
    Remove-Item $fixture -Recurse -Force
}
# Actions' PowerShell wrapper propagates LASTEXITCODE; the rejected fixture is expected.
$global:LASTEXITCODE = 0
