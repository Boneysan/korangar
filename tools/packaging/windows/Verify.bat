@echo off
rem Same reason Play.bat exists: a .ps1 double-clicks into Notepad and the
rem default execution policy refuses downloaded scripts.
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Verify.ps1"
pause
