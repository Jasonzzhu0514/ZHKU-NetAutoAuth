param([string]$ReleaseTag = '')

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

    & cargo fmt --check
    if ($LASTEXITCODE -ne 0) { throw 'Formatting check failed.' }
    & cargo clippy --locked --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Clippy checks failed.' }
    & cargo test --locked
    if ($LASTEXITCODE -ne 0) { throw 'Tests failed.' }
    & cargo build --locked --release --target x86_64-pc-windows-msvc --bin zhku-netautoauth-gui
    if ($LASTEXITCODE -ne 0) { throw 'Executable build failed.' }

    $executable = Join-Path $metadata.target_directory 'x86_64-pc-windows-msvc\release\zhku-netautoauth-gui.exe'
    if (-not (Test-Path -LiteralPath $executable)) { throw 'Release executable is missing.' }
    # --version exits before GUI initialization, credential loading or network access.
    $versionOutput = (& $executable --version | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $versionOutput -ne "ZHKU-NetAutoAuth $appVersion") { throw 'Packaged executable version verification failed.' }
    New-Item -ItemType Directory -Path 'dist' -Force | Out-Null
    Copy-Item -LiteralPath $executable -Destination 'dist\ZHKU-NetAutoAuth.exe' -Force
    Write-Host "Built dist\ZHKU-NetAutoAuth.exe ($appVersion)"
} finally {
    Pop-Location
}
