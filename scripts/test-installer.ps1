param(
    [Parameter(Mandatory)][string]$PayloadExe,
    [string]$NsisCompiler = 'makensis.exe'
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$payload = (Resolve-Path -LiteralPath $PayloadExe).Path
$template = Get-Content -LiteralPath (Join-Path $projectRoot 'installer/windows.nsi') -Raw
$sandbox = Join-Path ([IO.Path]::GetTempPath()) ('zhku uninstall test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $sandbox | Out-Null

function Run-Installer([string]$File, [string[]]$Arguments) {
    $process = Start-Process -FilePath $File -ArgumentList $Arguments -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit(30000)) { $process.Kill(); throw 'Installer test timed out.' }
    if ($process.ExitCode -ne 0) { throw "Installer test failed with exit code $($process.ExitCode)." }
}

foreach ($removeData in @($false, $true)) {
    $identity = 'ZHKU-UninstallTest-' + [guid]::NewGuid().ToString('N')
    $data = Join-Path ([IO.Path]::GetTempPath()) $identity
    $installation = Join-Path $sandbox $identity
    # Compile the real installer logic with isolated product, registry and data paths.
    $source = $template.Replace('$LOCALAPPDATA\ZHKU-NetAutoAuth', '$TEMP\' + $identity)
    $source = $source.Replace('ZHKU-NetAutoAuth', $identity)
    if ($removeData) {
        # Simulate an explicitly checked box while exercising the actual uninstall section.
        $source = $source.Replace('StrCpy $DeleteUserData ${BST_UNCHECKED}', 'StrCpy $DeleteUserData ${BST_CHECKED}')
    }
    $script = Join-Path $sandbox ($identity + '.nsi')
    $setup = Join-Path $sandbox ($identity + '-setup.exe')
    [IO.File]::WriteAllText($script, $source, [Text.UTF8Encoding]::new($false))
    & $NsisCompiler /V2 /INPUTCHARSET UTF8 /WX /DAPP_VERSION=0.0.0 "/DAPP_EXE=$payload" "/DOUT_FILE=$setup" $script
    if ($LASTEXITCODE -ne 0) { throw 'Isolated NSIS compilation failed.' }
    # NSIS consumes the remainder for /D= and _?=; keep these last and unquoted.
    Run-Installer $setup @('/S', "/D=$installation")
    New-Item -ItemType Directory -Path (Join-Path $data 'rust-gui') -Force | Out-Null
    $ownedFiles = @('credentials.dat', 'desktop.json', 'rust-gui\credentials.dat', 'rust-gui\desktop.json')
    foreach ($file in $ownedFiles) { [IO.File]::WriteAllText((Join-Path $data $file), 'synthetic-test-data') }
    [IO.File]::WriteAllText((Join-Path $data 'unrelated.txt'), 'keep')
    [IO.File]::WriteAllText((Join-Path $data 'rust-gui\unrelated.txt'), 'keep')
    Run-Installer (Join-Path $installation 'Uninstall.exe') @('/S', "_?=$installation")
    foreach ($file in $ownedFiles) {
        if ((Test-Path -LiteralPath (Join-Path $data $file)) -eq $removeData) {
            throw "Unexpected configuration retention result: $file"
        }
    }
    foreach ($file in @('unrelated.txt', 'rust-gui\unrelated.txt')) {
        if ((Get-Content -LiteralPath (Join-Path $data $file) -Raw) -ne 'keep') { throw 'Unrelated data was modified.' }
    }
    if (Test-Path -LiteralPath (Join-Path $installation ($identity + '.exe'))) { throw 'Application was not uninstalled.' }
    Write-Host "Uninstall test passed: delete data = $removeData; unrelated files preserved."
}
