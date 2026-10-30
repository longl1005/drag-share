$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
$version = (Get-Content package.json -Raw | ConvertFrom-Json).version
npm run tauri build -- --target x86_64-pc-windows-msvc --config src-tauri/tauri.windows.conf.json
if ($LASTEXITCODE -ne 0) { throw 'Windows build failed' }
$root = 'src-tauri/target/x86_64-pc-windows-msvc/release'
$installers = @(Get-ChildItem "$root/bundle/nsis/*_${version}_x64-setup.exe")
if ($installers.Count -ne 1) { throw 'Expected exactly one NSIS installer for this version' }
New-Item release -ItemType Directory -Force | Out-Null
Copy-Item $installers[0].FullName "release/drag-share-$version-windows-x64-setup.exe"
Compress-Archive -Path "$root/drag-share.exe" -DestinationPath "release/drag-share-$version-windows-x64.zip" -Force
node scripts/sha256sums.mjs release
if ($LASTEXITCODE -ne 0) { throw 'Checksum generation failed' }
