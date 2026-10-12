$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    $buildPython = Join-Path $projectRoot '.venv\Scripts\python.exe'
    if (-not (Test-Path -LiteralPath $buildPython)) {
        & python -m venv .venv
        if ($LASTEXITCODE -ne 0) { throw 'Failed to create the build environment.' }
    }
    & $buildPython -m pip install -r scripts\requirements-build.txt
    if ($LASTEXITCODE -ne 0) { throw 'Failed to install build dependencies.' }
    & $buildPython -X utf8 -P -m unittest discover -s tests -v
    if ($LASTEXITCODE -ne 0) { throw 'Tests failed.' }
    & $buildPython -m PyInstaller --noconfirm --clean --onefile --console --specpath build --name ZHKU-NetAutoAuth src\netauth.py
    if ($LASTEXITCODE -ne 0) { throw 'Executable build failed.' }
    Write-Host 'Built dist\ZHKU-NetAutoAuth.exe'
} finally {
    Pop-Location
}
