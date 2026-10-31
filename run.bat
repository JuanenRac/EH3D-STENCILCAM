@echo off
rem Runs eh3d-stencilcam with its sample. Extra arguments replace the sample: run.bat [ARGUMENTS]
cd /d "%~dp0"
set "ARGS=%*"
if "%ARGS%"=="" set "ARGS=check examples/paste.gtp"
cargo run --release --quiet -- %ARGS%
if not defined NOPAUSE pause
exit /b %ERRORLEVEL%
