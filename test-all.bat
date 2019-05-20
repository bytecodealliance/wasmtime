@echo off
setlocal

@rem This is the top-level test script.
@rem It is an adaption of the shell script "test-all.sh".
@rem - Check code formatting.
@rem - Make a debug build.
@rem - Make a release build.
@rem - Run unit tests for all Rust crates
@rem - Build API documentation.
@rem All tests run by this script should be passing at all times.

for /F "delims=" %%i in ("%%~f0") do set dirname=%%~dpi
cd %dirname%

call :banner Rust formatting
cmd /c "%dirname%format-all.bat --check"
if %errorlevel% neq 0 (
    echo Formatting diffs detected! Run "cargo fmt --all" to correct.
    goto error
)

call :banner Release build
cmd /c cargo build --release || goto error

call :banner Debug build
cmd /c cargo build || goto error

call :banner Rust unit tests
set RUST_BACKTRACE=1
cmd /c cargo test --all || goto error

call :banner Rust documentation: %dirname%target\doc\wasi-common\index.html
cmd /c cargo doc || goto error

call :banner OK

endlocal
exit /b %ERRORLEVEL%

:banner
echo ===== %* =====
exit /b 0

:error
exit /b 1