@echo off
REM Build src-lib first
cargo build --release -p src-lib
IF %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

REM Build src-sidecar for Windows
cargo build --release --bin src-sidecar --target x86_64-pc-windows-msvc
IF %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

REM Rename/copy to what Tauri expects
copy /Y "target\x86_64-pc-windows-msvc\release\src-sidecar.exe" "target\release\src-sidecar-x86_64-pc-windows-msvc.exe"
