@echo off
REM Build src-lib in debug mode
cargo build -p src-lib
IF %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

REM Build src-sidecar for Windows in debug mode
cargo build --bin src-sidecar --target x86_64-pc-windows-msvc
IF %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

REM Rename/copy to what Tauri expects for dev
copy /Y "target\x86_64-pc-windows-msvc\debug\src-sidecar.exe" "target\release\src-sidecar-x86_64-pc-windows-msvc.exe"

REM Run Tauri dev
bun tauri dev