@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ============================================================
REM SimpleVoiceCleaner - Windows Build Script
REM ============================================================
REM Requirements:
REM   1) Rust: https://rustup.rs/
REM   2) Visual Studio Build Tools with C++ build tools
REM   3) Git
REM
REM Usage:
REM   Double-click this file, or run it in PowerShell/CMD.
REM ============================================================

cd /d "%~dp0"

if not defined NO_PAUSE_ON_SUCCESS set "NO_PAUSE_ON_SUCCESS=0"


echo.
echo [SimpleVoiceCleaner] Build started...
echo Project folder: %CD%
echo.

where cargo >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Rust/Cargo was not found.
    echo Install Rust first: https://rustup.rs/
    echo.
    pause
    exit /b 1
)

where git >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Git was not found.
    echo NIH-plug is pulled from GitHub, so Git is required.
    echo Install Git: https://git-scm.com/download/win
    echo.
    pause
    exit /b 1
)

echo [1/4] Rust version:
rustc --version
cargo --version

echo.
echo [2/4] Checking Rust toolchain...
rustup default stable
if errorlevel 1 (
    echo [WARN] rustup default stable failed. Continuing with current toolchain.
)

echo.
echo [3/4] Building VST3/CLAP bundle...
echo Command: cargo run --package xtask -- bundle simple_voice_cleaner --release
cargo run --package xtask -- bundle simple_voice_cleaner --release
if errorlevel 1 (
    echo.
    echo [ERROR] Build failed.
    echo.
    echo Common fixes:
    echo - Install Visual Studio Build Tools with "Desktop development with C++"
    echo - Install Git
    echo - Make sure this folder path does not contain unusual special characters
    echo - If this is the first build, wait for Rust to download dependencies from GitHub/crates.io
    echo.
    pause
    exit /b 1
)

echo.
echo [4/4] Build complete.
echo.
echo Output folder:
echo %CD%\target\bundled

echo.
if exist "%CD%\target\bundled" (
    start "" "%CD%\target\bundled"
) else (
    echo [WARN] target\bundled folder was not found.
)

echo.
echo To install VST3 manually, copy:
echo   target\bundled\simple_voice_cleaner.vst3
echo to:
echo   C:\Program Files\Common Files\VST3
echo.

if "%NO_PAUSE_ON_SUCCESS%"=="0" pause
endlocal
