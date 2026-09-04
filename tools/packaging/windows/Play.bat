@echo off
rem Seal Cascade launcher.
rem
rem This is a .bat and not a .ps1 on purpose: double-clicking a .ps1 opens it in
rem Notepad, and Windows' default execution policy refuses to run one that was
rem downloaded. A .bat double-clicks and runs everywhere, so it is the door, and
rem PowerShell does the actual work behind it.
cd /d "%~dp0"

where powershell >nul 2>nul
if errorlevel 1 goto :nopowershell

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Play.ps1"
if errorlevel 1 pause
exit /b

:nopowershell
rem Fall back to launching directly. The checks are skipped, so if the pack is
rem incomplete the client fails on its own terms instead of explaining itself.
echo PowerShell was not found. Starting the game without the setup checks.
start "" korangar.exe
