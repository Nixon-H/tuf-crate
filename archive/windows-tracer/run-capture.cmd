@echo off
REM tuf-crate one-click capture — run from an elevated cmd in this folder.
REM Only OBSERVES: debugger breakpoints on ntdll I/O APIs; never constructs requests.
REM Safe exit: Ctrl+C disarms all breakpoints before detaching.

if "%~1"=="" (
  echo Finding asus_framework.exe PID...
  tasklist /fi "imagename eq asus_framework.exe" /fo csv | findstr /i asus_framework
  echo.
  echo Usage: run-capture.cmd <PID-of-asus_framework.exe>
  echo Then perform the 7 Armoury Crate lighting operations and press Ctrl+C here.
  exit /b 1
)

tuf_trace.exe %~1 "%USERPROFILE%\Desktop\trace"
echo.
echo TRACE COMPLETE: %USERPROFILE%\Desktop\trace.csv + trace.buffers.bin
echo Copy both files to the Linux side, then reboot.
pause
