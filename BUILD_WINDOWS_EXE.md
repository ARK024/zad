# 🚀 دليل بناء تطبيق زاد المسلم - Windows EXE

## 📋 المتطلبات الأساسية

### 1. **تثبيت Rust**

1. افتح PowerShell كمسؤول (Admin)
2. نفذ الأمر التالي:
```powershell
winget install Rustlang.Rust.MSVC
```

أو من الموقع الرسمي:
- رابط التحميل: https://rustup.rs/
- حمل `rustup-init.exe`
- شغله واختار "Default Installation"

3. تأكد من التثبيت:
```powershell
rustc --version
cargo --version
```

يجب أن يظهر:
```
rustc 1.77.0 (أو أحدث)
cargo 1.77.0 (أو أحدث)
```

---

### 2. **تثبيت WebView2**

موجود افتراضياً في Windows 10/11، لكن لو مش موجود:
- حمل من: https://developer.microsoft.com/en-us/microsoft-edge/webview2/
- اختار: **Evergreen Bootstrapper**

---

### 3. **تثبيت Visual Studio Build Tools**

1. حمل من: https://visualstudio.microsoft.com/downloads/
2. اختار: **Build Tools for Visual Studio**
3. في Components، اختار:
   - ✅ MSVC v143 - VS 2022 C++ x64/x86 build tools
   - ✅ Windows 10 SDK (أو Windows 11 SDK)
   - ✅ C++ CMake tools for Windows

---

## 🔨 خطوات البناء

### الطريقة 1: باستخدام Tauri CLI (موصى بها)

```powershell
# 1. تثبيت Tauri CLI
cargo install tauri-cli --version "^2"

# 2. الانتقال لمجلد المشروع
cd C:\path\to\zad-rs\src-tauri

# 3. بناء التطبيق
cargo tauri build

# 4. الـexe هيكون في:
# src-tauri\target\release\زاد المسلم.exe
```

---

### الطريقة 2: بناء مباشر (أسرع)

```powershell
# 1. الانتقال لمجلد المشروع
cd C:\path\to\zad-rs\src-tauri

# 2. بناء النسخة النهائية
cargo build --release

# 3. الـexe هيكون في:
# src-tauri\target\release\زاد المسلم.exe
```

---

### الطريقة 3: بناء مع اختبار

```powershell
# 1. اختبار الكود
cargo check

# 2. اختبار الوحدة
cargo test --lib

# 3. بناء النسخة النهائية
cargo build --release

# 4. التحقق من حجم الملف
ls target\release\زاد*.exe
```

---

## 📦 ملفات البناء

بعد ما البناء يخلص، الملفات هتكون في:

```
src-tauri/target/release/
├── زاد المسلم.exe          # التطبيق الرئيسي
├── زاد المسلم.pdb          # debug symbols
└── (ملفات DLL تابعة)
```

---

## 🎯 إنشاء Installer (اختياري)

### باستخدام NSIS:

```powershell
# 1. تثبيت NSIS
winget install NSIS.NSIS

# 2. بناء installer
cd src-tauri
cargo tauri build --bundles nsis
```

الinstaller هيكون في:
```
src-tauri/target/release/bundle/nsis/زاد المسلم_1.1.0_x64-setup.exe
```

---

## ⚠️ حل المشاكل الشائعة

### المشكلة 1: `cargo: not found`

**الحل:**
```powershell
# تأكد من أن Rust مثبت
rustup --version

# لو مش موجود، ثبته:
winget install Rustlang.Rust.MSVC

# أعد تشغيل PowerShell
```

---

### المشكلة 2: `link.exe` not found

**الحل:**
```powershell
# ثبّت Visual Studio Build Tools
winget install Microsoft.VisualStudio.2022.BuildTools

# اختار C++ build tools في التثبيت
```

---

### المشكلة 3: WebView2 runtime missing

**الحل:**
```powershell
# حمل WebView2
winget install Microsoft.WebView2
```

---

### المشكلة 4: أخطاء في الـdependencies

**الحل:**
```powershell
# امسح cache وأعد البناء
cd src-tauri
cargo clean
cargo build --release
```

---

## 📊 مواصفات الـexe النهائي

| البند | القيمة |
|-------|--------|
| **الحجم** | ~6-8 MB |
| **الذاكرة** | ~140 MB عند التشغيل |
| **الإقلاع** | ~1-2 ثانية |
| **المنصة** | Windows 10/11 (x64) |
| **المتطلبات** | WebView2 runtime |

---

## 🎯 خطوات ما بعد البناء

### 1. اختبار التطبيق:
```powershell
.\target\release\زاد المسلم.exe
```

### 2. التحقق من الملفات:
```powershell
# حجم الملف
(Get-Item .\target\release\زاد*.exe).Length / 1MB

# التوقيع الرقمي (اختياري)
# يحتاج شهادة توقيع
```

### 3. إنشاء اختصار:
```powershell
$WshShell = New-Object -comObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("$Home\Desktop\زاد المسلم.lnk")
$Shortcut.TargetPath = "$PWD\target\release\زاد المسلم.exe"
$Shortcut.Save()
```

---

## 📝 ملاحظات مهمة

1. **أول بناء هيخد وقت** (5-15 دقيقة) عشان تحميل الـdependencies
2. **البناء اللي بعده أسرع** (1-3 دقائق)
3. **تأكد من:**
   - اتصال الإنترنت موجود (عشان الـdependencies)
   - مساحة كافية على الهارد (2-3 GB)
   - PowerShell مفتوح كمسؤول (لو فيه مشاكل permissions)

---

## 🔗 روابط مفيدة

- **Rust:** https://rustup.rs/
- **Tauri Docs:** https://v2.tauri.app/
- **WebView2:** https://developer.microsoft.com/en-us/microsoft-edge/webview2/
- **Visual Studio Build Tools:** https://visualstudio.microsoft.com/downloads/

---

## ✅ التحقق النهائي

بعد ما البناء يخلص، تأكد من:

```powershell
# 1. الملف موجود
Test-Path .\target\release\زاد المسلم.exe

# 2. الحجم معقول
(Get-Item .\target\release\زاد*.exe).Length / 1MB  # يجب أن يكون ~6-8 MB

# 3. التطبيق يشتغل
.\target\release\زاد المسلم.exe
```

---

**آخر تحديث:** 2026-05-21  
**الإصدار:** v1.1.0  
**الحالة:** ✅ جاهز للبناء
