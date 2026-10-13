param([string]$ReleaseTag = '', [string]$NsisCompiler = 'makensis.exe')

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    $metadataJson = & cargo metadata --locked --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read application metadata.' }
    $metadata = $metadataJson | ConvertFrom-Json
    $application = $metadata.packages | Where-Object { $_.name -eq 'zhku-netautoauth-gui' -and $null -eq $_.source }
    if (@($application).Count -ne 1) { throw 'Cannot identify the desktop application package.' }
    $appVersion = $application.version
    if ($ReleaseTag -and $ReleaseTag -ne "v$appVersion") { throw 'Tag must match the package version in Cargo.toml.' }
    $desktopMetadataJson = & cargo metadata --manifest-path src-tauri/Cargo.toml --locked --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read Tauri metadata.' }
    $desktopMetadata = $desktopMetadataJson | ConvertFrom-Json
    $desktop = $desktopMetadata.packages | Where-Object { $_.name -eq 'zhku-netautoauth-desktop' -and $null -eq $_.source }
    $frontend = Get-Content package.json -Raw | ConvertFrom-Json
    $tauriConfig = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
    if (@($desktop).Count -ne 1 -or $desktop.version -ne $appVersion -or $frontend.version -ne $appVersion -or $tauriConfig.version -ne $appVersion) {
        throw 'Core, Tauri, frontend and window versions must match.'
    }
    & npm.cmd ci
    if ($LASTEXITCODE -ne 0) { throw 'Frontend dependency installation failed.' }
    & npm.cmd run test:ui
    if ($LASTEXITCODE -ne 0) { throw 'Frontend build or tests failed.' }

    & cargo fmt --check
    if ($LASTEXITCODE -ne 0) { throw 'Formatting check failed.' }
    & cargo clippy --locked --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Clippy checks failed.' }
    & cargo test --locked
    if ($LASTEXITCODE -ne 0) { throw 'Tests failed.' }
    & cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
    if ($LASTEXITCODE -ne 0) { throw 'Tauri formatting check failed.' }
    & cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Tauri checks failed.' }
    & cargo build --manifest-path src-tauri/Cargo.toml --locked --release --target x86_64-pc-windows-msvc --features custom-protocol
    if ($LASTEXITCODE -ne 0) { throw 'Executable build failed.' }

    $executable = Join-Path $desktopMetadata.target_directory 'x86_64-pc-windows-msvc\release\zhku-netautoauth-desktop.exe'
    if (-not (Test-Path -LiteralPath $executable)) { throw 'Release executable is missing.' }
    # --version exits before GUI initialization, credential loading or network access.
    $versionOutput = (& $executable --version | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $versionOutput -ne "ZHKU-NetAutoAuth $appVersion") { throw 'Packaged executable version verification failed.' }
    New-Item -ItemType Directory -Path 'dist' -Force | Out-Null
    Copy-Item -LiteralPath $executable -Destination 'dist\ZHKU-NetAutoAuth.exe' -Force
    Write-Host "Built dist\ZHKU-NetAutoAuth.exe ($appVersion)"
    $installerOutput = Join-Path $projectRoot 'dist\ZHKU-NetAutoAuth-Setup.exe'
    & $NsisCompiler /INPUTCHARSET UTF8 /WX "/DAPP_VERSION=$appVersion" "/DAPP_EXE=$executable" "/DOUT_FILE=$installerOutput" installer/windows.nsi
    if ($LASTEXITCODE -ne 0) { throw 'Installer build failed.' }
    if (-not (Test-Path -LiteralPath $installerOutput)) { throw 'Installer is missing.' }
    Write-Host "Built dist\ZHKU-NetAutoAuth-Setup.exe ($appVersion)"
} finally {
    Pop-Location
}
