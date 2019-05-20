@echo off
setlocal

@rem Format all sources using rustfmt.
@rem This script is an adaption of the shell version
@rem 'format-all.sh'.

for /F "delims=" %%i in ("%%~f0") do set dirname=%%~dpi
cd %dirname%

@REM Make sure we can find rustfmt
set PATH=%PATH%;%USERPROFILE%\.cargo\bin

cmd /C cargo +stable fmt --all -- %*

endlocal
exit /b %ERRORLEVEL%