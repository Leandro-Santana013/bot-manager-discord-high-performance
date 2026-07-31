@echo off
cargo build --release
if %ERRORLEVEL% EQU 0 (
    echo Bot compilado com sucesso! Iniciando...
    .\target\release\bot-rust.exe
) else (
    echo Erro na compilacao!
    pause
)
