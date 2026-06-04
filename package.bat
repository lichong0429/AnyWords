@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion

set "RELEASE_DIR=anywords-release"
set "PROJECT_DIR=%~dp0"

echo ====================================
echo   AnyWords Release Packager
echo ====================================
echo.

:: Create release directory
if exist "%RELEASE_DIR%" rmdir /s /q "%RELEASE_DIR%"
mkdir "%RELEASE_DIR%"

:: Copy binary
if exist "target\release\anywords.exe" (
    echo [OK] Copying anywords.exe...
    copy /y "target\release\anywords.exe" "%RELEASE_DIR%\" >nul
) else (
    echo [ERROR] target\release\anywords.exe not found!
    echo Run: cargo build --release
    pause
    exit /b 1
)

:: Copy frontend
if exist "frontend\dist" (
    echo [OK] Copying frontend...
    xcopy /e /i /q "frontend\dist" "%RELEASE_DIR%\frontend\dist" >nul
) else (
    echo [ERROR] frontend\dist not found! Run: cd frontend ^&^& npm run build
    pause
    exit /b 1
)

:: Copy config template
echo [OK] Copying config template...
copy /y "anywords.sample.yml" "%RELEASE_DIR%\" >nul

:: Copy start script
echo [OK] Copying start script...
copy /y "start.bat" "%RELEASE_DIR%\" >nul

:: Copy README
echo [OK] Copying README...
copy /y "README.md" "%RELEASE_DIR%\" >nul
copy /y "README_zh.md" "%RELEASE_DIR%\" >nul

:: Create data directory
mkdir "%RELEASE_DIR%\data" 2>nul

echo.
echo ====================================
echo   Release package created!
echo   Location: %RELEASE_DIR%
echo ====================================
echo.
echo Contents:
dir /b "%RELEASE_DIR%"
echo.
echo To run: double-click start.bat inside the release folder

endlocal
