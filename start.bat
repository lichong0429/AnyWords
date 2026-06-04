@echo off
chcp 65001 >nul
title AnyWords - 本地文件全文搜索

set "ANYWORDS_DIR=%~dp0"
cd /d "%ANYWORDS_DIR%"

echo.
echo ╔══════════════════════════════════════════════╗
echo ║           AnyWords v0.1.0                     ║
echo ║     本地文件全文搜索引擎                        ║
echo ╚══════════════════════════════════════════════╝
echo.
echo 启动中...
echo.

:: Check if binary exists
if not exist "anywords.exe" (
    echo [错误] 找不到 anywords.exe，请确保文件在正确位置
    pause
    exit /b 1
)

:: Create data directory
if not exist "data" mkdir "data"

:: Check for config
if not exist "anywords.yml" (
    echo [信息] 未找到 anywords.yml，将使用默认配置
    echo [提示] 复制 anywords.sample.yml 为 anywords.yml 可自定义配置
)

:: Start the server
start "" "http://localhost:9921"
anywords.exe

pause
