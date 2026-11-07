@echo off
rem ============================================================================
rem  benilla local client - shared settings
rem
rem  Edit this file once. Every other script reads the values from here.
rem  Do not run this file directly.
rem ============================================================================

rem The scripts live in <root>\scripts; ROOT is the repository root.
for %%I in ("%~dp0..") do set "ROOT=%%~fI"

rem --- WoW 1.12.1 client folder (the folder that contains wow.exe and Data) ---
rem benilla reads the game data straight from <CLIENT_DIR>\Data; nothing is
rem copied. client-dir.txt at the repository root, when present, overrides
rem this value (02-play-client.bat writes it after asking once).
set "CLIENT_DIR=E:\games\wow_local"
if exist "%ROOT%\client-dir.txt" for /f "usebackq delims=" %%c in ("%ROOT%\client-dir.txt") do set "CLIENT_DIR=%%c"

rem --- Local server ----------------------------------------------------------
set "WOW_HOST=127.0.0.1"
set "AUTH_PORT=3724"

rem --- Build output, kept off the system disk --------------------------------
rem A release build needs about 4 GB free here. `cargo play` (incremental)
rem needs 8-10 GB, because it also keeps the incremental artifacts.
set "TARGET_DIR=E:\work\wow\benilla\target"

rem --- Visual Studio build tools ---------------------------------------------
rem vcvars64.bat initializes the MSVC environment Rust needs on Windows.
rem 01-build-client.bat falls back to vswhere when this path does not exist.
set "VS_VCVARS=C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

rem --- Internals -------------------------------------------------------------
set "BENILLA_EXE=%TARGET_DIR%\release\benilla.exe"
set "CARGO_TARGET_DIR=%TARGET_DIR%"
if defined CLIENT_DIR set "WOW_DATA=%CLIENT_DIR%\Data"

exit /b 0
