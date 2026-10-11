@echo off
setlocal
if exist "%~dp0dist\ZHKU-NetAutoAuth.exe" (
    "%~dp0dist\ZHKU-NetAutoAuth.exe" %*
    exit /b
)
where py >nul 2>nul
if not errorlevel 1 (
    py -3 -E "%~dp0src\netauth.py" %*
) else (
    where python >nul 2>nul
    if errorlevel 1 (
        echo Please install Python 3.10 or newer from https://www.python.org/downloads/
        pause
        exit /b 1
    )
    python -E "%~dp0src\netauth.py" %*
)
set "auth_exit=%errorlevel%"
if not "%auth_exit%"=="0" pause
exit /b %auth_exit%
