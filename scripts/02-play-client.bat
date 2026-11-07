@echo off
chcp 65001 >nul
setlocal EnableExtensions
set "CODE=0"
set "HERE=%~dp0"
call "%HERE%00-client-config.bat"

echo ============================================================
echo   benilla - запуск клиента
echo ============================================================
echo.

:resolve_client
if not defined CLIENT_DIR goto ask_client
if not exist "%CLIENT_DIR%\Data" goto ask_client
goto client_ready

:ask_client
echo Укажи папку с WoW 1.12.1 ^(в ней должны лежать wow.exe и Data^).
set "CLIENT_DIR="
set /p "CLIENT_DIR=Путь: "
if not defined CLIENT_DIR (
    echo Отменено.
    set "CODE=1"
    goto :finish
)
set "CLIENT_DIR=%CLIENT_DIR:"=%"
if not exist "%CLIENT_DIR%\Data" (
    echo [ОШИБКА] в "%CLIENT_DIR%" нет папки Data.
    set "CODE=1"
    goto :finish
)
> "%ROOT%\client-dir.txt" echo %CLIENT_DIR%
echo Путь сохранён в client-dir.txt.
set "WOW_DATA=%CLIENT_DIR%\Data"

:client_ready
if not exist "%BENILLA_EXE%" (
    echo [ОШИБКА] не найден "%BENILLA_EXE%".
    choice /C YN /N /M "Собрать клиент сейчас? [Y/N] "
    if errorlevel 2 (
        set "CODE=1"
        goto :finish
    )
    set "BENILLA_NO_PAUSE=1"
    call "%HERE%01-build-client.bat"
    set "BENILLA_NO_PAUSE="
    if errorlevel 1 (
        set "CODE=1"
        goto :finish
    )
)

call :port_listening %AUTH_PORT%
if "%LISTENING%"=="0" (
    echo.
    echo [ВНИМАНИЕ] сервер не слушает порт %AUTH_PORT%.
    echo Запусти его в core: scripts\04-start-server.bat
    echo.
)

echo Данные клиента: "%WOW_DATA%"
echo Сервер: %WOW_HOST%:%AUTH_PORT%
echo Запускаю benilla...
start "" /D "%ROOT%" "%BENILLA_EXE%"
goto :finish

rem ============================================================================
:port_listening
set "LISTENING=0"
netstat -ano | findstr /C:":%~1 " | findstr /C:"LISTENING" >nul 2>&1
if not errorlevel 1 set "LISTENING=1"
exit /b 0

:finish
echo.
if not defined BENILLA_NO_PAUSE pause
exit /b %CODE%
