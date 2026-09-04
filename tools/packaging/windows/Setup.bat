@echo off
rem Seal Cascade first-run setup.
rem
rem This is a .bat and not a .ps1 on purpose: double-clicking a .ps1 opens it in
rem Notepad, and Windows' default execution policy refuses to run one that was
rem downloaded. A .bat double-clicks and runs everywhere, so it is the door, and
rem PowerShell does the actual work behind it.
cd /d "%~dp0"

where powershell >nul 2>nul
if errorlevel 1 goto :nopowershell

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Setup.ps1"

rem Always pause. Setup is the one script whose OUTPUT is the product -- a
rem friend who is stuck needs to read the sentence explaining why, and a
rem console that closes on its own takes that away.
echo.
pause
exit /b

:nopowershell
echo.
echo   Setup needs Windows PowerShell, and it is not on this PC.
echo.
echo   That is unusual -- it ships with Windows. Tell the host.
echo.
pause
