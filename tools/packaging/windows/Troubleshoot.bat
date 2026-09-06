@echo off
rem Seal Cascade troubleshooter -- for a white screen, or a window that opens
rem and shows nothing.
rem
rem A .bat and not a .ps1 for the same reason as Play.bat: double-clicking a
rem .ps1 opens Notepad, and the default execution policy refuses a downloaded
rem one. This file uses no PowerShell at all, so it works even where Play does
rem not.
rem
rem Each option runs the game in the SAME window, so whatever the game prints
rem stays on screen instead of vanishing with it.
setlocal
cd /d "%~dp0"
set "RUST_BACKTRACE=full"
set "KORANGAR_ANIMATION_LOG=1"

if not exist "korangar.exe" (
    echo.
    echo   korangar.exe is not in this folder.
    echo   Put Troubleshoot.bat next to the game and run it again.
    echo.
    pause
    exit /b 1
)

:menu
set "WGPU_BACKEND="
set "WGPU_ADAPTER_NAME="
cls
echo.
echo   Seal Cascade -- white screen troubleshooter
echo.
echo   Try these IN ORDER. After each one, close the game window and
echo   come back to this menu. Tell the host which number worked.
echo.
echo     1   Reset the graphics settings, then start   ^<-- try this first
echo     2   Start using Vulkan
echo     3   Start using DirectX 12
echo     4   Reset the settings AND use Vulkan
echo     5   Force the Radeon card (if this PC also has built-in graphics)
echo     6   Start normally, changing nothing
echo.
echo     7   Collect a diagnostics report to send to the host
echo.
echo     Q   Quit
echo.
set "pick="
set /p "pick=  Type a number and press Enter: "

if /i "%pick%"=="Q" exit /b 0
if "%pick%"=="1" goto opt1
if "%pick%"=="2" goto opt2
if "%pick%"=="3" goto opt3
if "%pick%"=="4" goto opt4
if "%pick%"=="5" goto opt5
if "%pick%"=="6" goto run
if "%pick%"=="7" goto diag
goto menu

:opt1
call :reset
goto run

:opt2
set "WGPU_BACKEND=vulkan"
echo   Using Vulkan.
goto run

:opt3
set "WGPU_BACKEND=dx12"
echo   Using DirectX 12.
goto run

:opt4
call :reset
set "WGPU_BACKEND=vulkan"
echo   Using Vulkan.
goto run

:opt5
set "WGPU_ADAPTER_NAME=Radeon"
echo   Forcing the Radeon card.
goto run

:reset
rem The game saves your graphics choices and re-reads them next time, so a bad
rem setting survives a restart and looks like the game broke by itself.
rem Keep the old settings so the host can inspect them or restore them.
if exist "client\graphics_settings.ron" (
    move /y "client\graphics_settings.ron" "client\graphics_settings.ron.previous" >nul
    echo   Graphics settings reset to defaults.
) else (
    echo   No saved graphics settings found, so there was nothing to reset.
)
exit /b 0

:diag
rem Everything here answers a question the game itself cannot, because when it
rem is killed by a native fault it never gets to write anything: no Rust panic,
rem no log line, nothing. The exit code and the Windows event entries are
rem recorded by the OS instead, and they name the faulting module.
rem
rem NOTE the redirect-first form (">> file echo text") used throughout. Writing
rem it the natural way round breaks silently: "echo code: %code%>> file" ends in
rem a DIGIT immediately before ">>", and cmd reads that digit as a file handle
rem rather than as text. An exit code of 0 would redirect stdin instead of
rem writing the line.
set "REPORT=%~dp0diagnostics.txt"
set "RAW=%TEMP%\korangar_run_output.txt"
cls
echo.
echo   Collecting diagnostics.
echo.
echo   The game will be started FOUR times, once per graphics mode.
echo   Each attempt either fails on its own (nothing to do) or opens the
echo   game -- if a game window opens, close it to continue.
echo.
pause

> "%REPORT%" echo Seal Cascade diagnostics
>> "%REPORT%" echo Generated %DATE% %TIME%
>> "%REPORT%" echo Folder: %~dp0
>> "%REPORT%" echo == Logs before diagnostics ==
if exist "korangar.log" type "korangar.log" >> "%REPORT%"
if exist "korangar.log.previous" type "korangar.log.previous" >> "%REPORT%"
>> "%REPORT%" echo.

>> "%REPORT%" echo == System ==
powershell -NoProfile -Command "Get-CimInstance Win32_Processor | Select-Object -ExpandProperty Name" >> "%REPORT%" 2>&1
powershell -NoProfile -Command "(Get-CimInstance Win32_OperatingSystem).Caption + ' build ' + (Get-CimInstance Win32_OperatingSystem).BuildNumber" >> "%REPORT%" 2>&1
>> "%REPORT%" echo.

>> "%REPORT%" echo == Graphics cards and drivers ==
powershell -NoProfile -Command "Get-CimInstance Win32_VideoController | ForEach-Object { $_.Name + '  driver ' + $_.DriverVersion + '  (' + $_.DriverDate + ')' }" >> "%REPORT%" 2>&1
>> "%REPORT%" echo.

>> "%REPORT%" echo == Graphics runtimes present ==
if exist "%SystemRoot%\System32\vulkan-1.dll" (>> "%REPORT%" echo vulkan-1.dll: present) else (>> "%REPORT%" echo vulkan-1.dll: MISSING - no Vulkan driver installed)
if exist "%SystemRoot%\System32\d3d12.dll" (>> "%REPORT%" echo d3d12.dll: present) else (>> "%REPORT%" echo d3d12.dll: MISSING)
if exist "%SystemRoot%\System32\dxgi.dll" (>> "%REPORT%" echo dxgi.dll: present) else (>> "%REPORT%" echo dxgi.dll: MISSING)
>> "%REPORT%" echo.

rem A backtrace costs nothing and pays off if this turns out to be a panic.
set "RUST_BACKTRACE=full"

>> "%REPORT%" echo == Runs ==
call :trybackend default
call :trybackend vulkan
call :trybackend dx12
call :trybackend gl
set "WGPU_BACKEND="

>> "%REPORT%" echo == korangar.log (this run) ==
if exist "korangar.log" (type "korangar.log" >> "%REPORT%") else (>> "%REPORT%" echo no korangar.log found)
>> "%REPORT%" echo.

>> "%REPORT%" echo == korangar.log.previous ==
if exist "korangar.log.previous" (type "korangar.log.previous" >> "%REPORT%") else (>> "%REPORT%" echo none)
>> "%REPORT%" echo.

rem Windows writes the crash entry a few seconds after the fault, so asking
rem immediately can report "none found" for a crash that did happen.
echo   Waiting for Windows to finish writing its crash records...
timeout /t 12 /nobreak >nul 2>&1

>> "%REPORT%" echo == Windows crash records naming korangar ==
powershell -NoProfile -Command "$names=@('Application Error','Windows Error Reporting','Application Hang'); $found=$false; foreach($n in $names){ try { $ev = Get-WinEvent -FilterHashtable @{LogName='Application'; ProviderName=$n} -MaxEvents 40 -ErrorAction Stop | Where-Object { $_.Message -like '*korangar*' } | Select-Object -First 3; foreach($e in $ev){ $found=$true; '### ' + $n + ' at ' + $e.TimeCreated.ToString(); $e.Message; '---' } } catch { } }; if(-not $found){ 'none found -- no crash record was written' }" >> "%REPORT%" 2>&1

echo.
echo   ------------------------------------------------------------
echo   Report written to:
echo     %REPORT%
echo.
echo   Send that file to the host.
echo   ------------------------------------------------------------
echo.
pause
goto menu

:trybackend
rem Run once under one backend and record how it ended. Running ALL of them
rem matters: "every backend dies the same way" and "only DX12 dies" are
rem different diagnoses, and only a full sweep can tell them apart.
set "BK=%~1"
if /i "%BK%"=="default" (set "WGPU_BACKEND=") else (set "WGPU_BACKEND=%BK%")
echo   trying %BK% ...
korangar.exe > "%RAW%" 2>&1
set "code=%ERRORLEVEL%"
>> "%REPORT%" echo --- backend: %BK%
>> "%REPORT%" echo     exit code: %code%
call :explain "%code%"
>> "%REPORT%" echo     output:
if exist "%RAW%" type "%RAW%" >> "%REPORT%"
>> "%REPORT%" echo     file log for this backend:
if exist "korangar.log" type "korangar.log" >> "%REPORT%"
if exist "%TEMP%\korangar\korangar.log" type "%TEMP%\korangar\korangar.log" >> "%REPORT%"
>> "%REPORT%" echo.
exit /b 0

:explain
rem Common Windows fault codes, so the host does not have to look them up.
set "meaning="
if "%~1"=="0" set "meaning=exited cleanly"
if "%~1"=="-1073741819" set "meaning=ACCESS VIOLATION (0xC0000005) - a crash inside a driver or DLL"
if "%~1"=="-1073741795" set "meaning=ILLEGAL INSTRUCTION (0xC000001D) - CPU lacks an instruction this build needs"
if "%~1"=="-1073741515" set "meaning=DLL NOT FOUND (0xC0000135) - a required runtime is missing"
if "%~1"=="-1073740791" set "meaning=STACK BUFFER OVERRUN (0xC0000409)"
if "%~1"=="-1073741571" set "meaning=STACK OVERFLOW (0xC00000FD)"
if "%~1"=="-1073741674" set "meaning=INTEGER DIVIDE BY ZERO (0xC0000094)"
if not defined meaning set "meaning=not a code this script knows - look it up as an NTSTATUS"
>> "%REPORT%" echo     meaning: %meaning%
exit /b 0

:run
echo.
echo   Starting the game. Leave this window open.
echo   ------------------------------------------------------------
rem Stamp a SEPARATE launcher log before running the executable. Missing DLLs
rem and unsupported CPU instructions can kill it before Rust main executes.
set "LAUNCHLOG=%~dp0troubleshoot-launch.log"
>> "%LAUNCHLOG%" echo Starting %DATE% %TIME% backend=%WGPU_BACKEND% adapter=%WGPU_ADAPTER_NAME%
korangar.exe >> "%LAUNCHLOG%" 2>&1
set "code=%ERRORLEVEL%"
>> "%LAUNCHLOG%" echo Finished %DATE% %TIME% exit code: %code%
echo   Exit code: %code%
echo   Output saved in troubleshoot-launch.log beside the game.
echo   ------------------------------------------------------------
echo.
echo   The game has closed. If the screen was still white, come back
echo   and try the next number.
echo.
pause
goto menu
