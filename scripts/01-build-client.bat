@echo off
chcp 65001 >nul
setlocal EnableExtensions
set "CODE=0"
set "HERE=%~dp0"
call "%HERE%00-client-config.bat"

echo ============================================================
echo   benilla - сборка клиента (release)
echo ============================================================
echo.

if not exist "%VS_VCVARS%" call :find_vcvars
if not exist "%VS_VCVARS%" (
    echo [ОШИБКА] не найден vcvars64.bat.
    echo Укажи путь к нему в scripts\00-client-config.bat ^(VS_VCVARS^).
    set "CODE=1"
    goto :finish
)

rem --- Free space on the target drive ----------------------------------------
for %%d in ("%TARGET_DIR%") do set "TARGET_DRIVE=%%~dd"
set "FREE_GB="
for /f %%G in ('powershell -NoProfile -Command "[math]::Floor((Get-PSDrive -Name '%TARGET_DRIVE:~0,1%').Free/1GB)" 2^>nul') do set "FREE_GB=%%G"
if defined FREE_GB (
    echo Свободно на %TARGET_DRIVE% %FREE_GB% ГБ.
    if %FREE_GB% LSS 5 (
        echo [ОШИБКА] для release-сборки нужно минимум ~4-5 ГБ свободных.
        echo Освободи место на %TARGET_DRIVE% и запусти снова.
        set "CODE=1"
        goto :finish
    )
)

echo Целевая папка: %TARGET_DIR%
echo Инициализирую окружение MSVC...
call "%VS_VCVARS%" >nul
if errorlevel 1 (
    echo [ОШИБКА] vcvars64.bat завершился с ошибкой.
    set "CODE=1"
    goto :finish
)

cd /d "%ROOT%"
echo Собираю benilla ^(cargo build --release -p benilla^)...
echo.
cargo build --release -p benilla
if errorlevel 1 (
    echo.
    echo [ОШИБКА] сборка не удалась.
    set "CODE=1"
    goto :finish
)

if not exist "%BENILLA_EXE%" (
    echo [ОШИБКА] сборка прошла, но "%BENILLA_EXE%" не найден.
    set "CODE=1"
    goto :finish
)

echo.
echo Готово: "%BENILLA_EXE%"
echo Следующий шаг: 02-play-client.bat
goto :finish

rem ============================================================================
:find_vcvars
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" exit /b 0
for /f "delims=" %%p in ('"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2^>nul') do (
    if exist "%%p\VC\Auxiliary\Build\vcvars64.bat" set "VS_VCVARS=%%p\VC\Auxiliary\Build\vcvars64.bat"
)
exit /b 0

:finish
echo.
if not defined BENILLA_NO_PAUSE pause
exit /b %CODE%
