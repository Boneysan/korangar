@echo off
rem Seal Cascade client update.
rem
rem Same door as Play.bat / Setup.bat: double-clicking a .ps1 opens Notepad,
rem and the default execution policy refuses a downloaded one. This .bat
rem double-clicks; PowerShell does the work.
cd /d "%~dp0"

where powershell >nul 2>nul
if errorlevel 1 goto :nopowershell

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Update.ps1"

echo.
pause
exit /b

:nopowershell
echo.
echo   Update needs Windows PowerShell, and it is not on this PC.
echo.
echo   That is unusual -- it ships with Windows. Tell the host.
echo.
pause
