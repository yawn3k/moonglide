@echo off
REM Build Moonglide and zip a Windows release archive.
REM Requires VS Build Tools + CMake (see docs/getting-started.md).
REM Usage: release-windows.bat   (run from the repo root)

cargo build --release || goto :err
set NAME=moonglide-windows.zip
del /Q %NAME% 2>nul
powershell -command "Compress-Archive -Path target\release\moonglide.exe,LICENSE -DestinationPath %NAME% -Force"
echo done: %NAME%
echo upload with: gh release create vX.Y.Z %NAME% --generate-notes
goto :eof

:err
echo build failed
exit /b 1
