@echo off
setlocal

REM ----------------------------------------
REM Win7 x86 target build (nightly + build-std)
REM ----------------------------------------

REM Force Win7 subsystem baseline (6.01)
set RUSTFLAGS=-C link-arg=/SUBSYSTEM:CONSOLE,6.01

REM Optional: sanity echo (helps logs / CI)
echo RUSTFLAGS=%RUSTFLAGS%
echo.

cargo +nightly build -r --workspace ^
  -Z build-std=std,panic_abort ^
  --target i686-win7-windows-msvc

endlocal
