@echo off
chcp 65001 >nul
echo ====================================
echo   زاد المسلم - بناء Windows EXE
echo ====================================
echo.

REM Check if cargo is installed
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [خطأ] Rust/Cargo غير مثبت!
    echo.
    echo تحميل Rust من: https://rustup.rs/
    echo أو: winget install Rustlang.Rust.MSVC
    pause
    exit /b 1
)

echo [✓] Rust مثبت
echo [✓] Cargo: %cargo:version%
echo.

REM Check if we're in the right directory
if not exist "Cargo.toml" (
    echo [خطأ] ملف Cargo.toml غير موجود!
    echo تأكد إنك في مجلد src-tauri
    pause
    exit /b 1
)

echo [✓] Cargo.toml موجود
echo.

echo 🚀 جاري البناء...
echo.

cargo build --release

if %errorlevel% neq 0 (
    echo.
    echo [خطأ] البناء فشل!
    pause
    exit /b 1
)

echo.
echo ====================================
echo   ✅ البناء تم بنجاح!
echo ====================================
echo.
echo الـexe موجود هنا:
echo   target\release\زاد المسلم.exe
echo.
echo الحجم:
for %%I in ("target\release\زاد المسلم.exe") do echo   %%~zI bytes
echo.
pause
