@echo off
setlocal

set "ROOT=%~dp0"
set "RUST_APP=%ROOT%rust-app"

echo Cleaning FindBT generated files...
echo.

if exist "%RUST_APP%\Cargo.toml" (
    where cargo >nul 2>nul
    if errorlevel 1 (
        echo Cargo was not found on PATH. Removing Rust target directory directly.
        if exist "%RUST_APP%\target" rmdir /s /q "%RUST_APP%\target"
    ) else (
        echo Running cargo clean...
        pushd "%RUST_APP%" >nul
        cargo clean
        popd >nul
    )
) else (
    echo rust-app\Cargo.toml was not found. Skipping cargo clean.
)

echo Removing generated package and test outputs...
if exist "%RUST_APP%\dist" rmdir /s /q "%RUST_APP%\dist"
if exist "%ROOT%FindBT-Release" rmdir /s /q "%ROOT%FindBT-Release"
if exist "%ROOT%artifacts" rmdir /s /q "%ROOT%artifacts"
if exist "%ROOT%publish" rmdir /s /q "%ROOT%publish"
if exist "%ROOT%TestResults" rmdir /s /q "%ROOT%TestResults"

del /q "%ROOT%FindBT-report-*.html" >nul 2>nul
del /q "%ROOT%defender-scan.txt" >nul 2>nul
del /q "%ROOT%findbt-crash.log" >nul 2>nul

echo.
echo Clean complete.

endlocal
