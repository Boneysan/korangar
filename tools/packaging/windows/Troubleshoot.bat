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
rem Deleting the file puts everything back to defaults; it is written again
rem when you next close the game.
if exist "client\graphics_settings.ron" (
    del /q "client\graphics_settings.ron"
    echo   Graphics settings reset to defaults.
) else (
    echo   No saved graphics settings found, so there was nothing to reset.
)
exit /b 0

:run
echo.
echo   Starting the game. Leave this window open.
echo   ------------------------------------------------------------
korangar.exe
echo   ------------------------------------------------------------
echo.
echo   The game has closed. If the screen was still white, come back
echo   and try the next number.
echo.
pause
goto menu
