@echo off
rem Seal Cascade script repair launcher.
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Repair.ps1" %*
pause
