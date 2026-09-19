@echo off
setlocal
rem Windows-native test APK; never installs JavaScript packages through WSL.
cd /d "%~dp0.."
set "ROOT=%CD%"
if not defined JAVA_HOME set "JAVA_HOME=C:\Program Files\Android\openjdk\jdk-21.0.8"
if not defined ANDROID_HOME set "ANDROID_HOME=%LOCALAPPDATA%\Android\Sdk"
if not defined NDK_HOME set "NDK_HOME=%ANDROID_HOME%\ndk\29.0.14206865"
set "PATH=%USERPROFILE%\.bun\bin;%USERPROFILE%\.cargo\bin;%JAVA_HOME%\bin;%ANDROID_HOME%\platform-tools;%PATH%"
for /f %%V in ('bun.exe --version') do if not "%%V"=="1.4.0" exit /b 1
if not exist "%NDK_HOME%\source.properties" exit /b 1
if not exist "%ROOT%\src-tauri\gen\android\gradlew.bat" (
  bun.exe run tauri android init --ci || exit /b 1
)
set "OUT=%~1"
if not defined OUT set "OUT=%ROOT%\artifacts\android"
if not exist "%OUT%" mkdir "%OUT%" || exit /b 1
set "LOG=%OUT%\android-build.log"
bun.exe run tauri android build --target aarch64 --apk --ci > "%LOG%" 2>&1
if errorlevel 1 (
  rem The CLI has compiled Rust successfully before attempting this symlink.
  rem Fall back only for the known Windows privilege error, never other failures.
  findstr /c:"Creation symbolic link is not allowed for this system." "%LOG%" >nul || exit /b 1
  copy /y "%ROOT%\target\aarch64-linux-android\release\libdeepwork_lib.so" "%ROOT%\src-tauri\gen\android\app\src\main\jniLibs\arm64-v8a\libdeepwork_lib.so" || exit /b 1
  pushd "%ROOT%\src-tauri\gen\android"
  call gradlew.bat :app:assembleArm64Release -x :app:rustBuildArm64Release -Pkotlin.incremental=false >> "%LOG%" 2>&1
  if errorlevel 1 exit /b 1
  popd
)
bun.exe "%ROOT%\scripts\check-elf-alignment.ts" "%ROOT%\src-tauri\gen\android\app\src\main\jniLibs\arm64-v8a\libdeepwork_lib.so" || exit /b 1
set "TOOLS=%ANDROID_HOME%\build-tools\35.0.0"
set "SOURCE=%ROOT%\src-tauri\gen\android\app\build\outputs\apk\arm64\release\app-arm64-release-unsigned.apk"
set "OUTPUT=%OUT%\Deepwork-shaft-first-arm64-test.apk"
if not exist "%USERPROFILE%\.android\debug.keystore" (
  echo Android debug keystore missing. Create a local test keystore before signing.
  exit /b 1
)
"%TOOLS%\zipalign.exe" -P 16 -f 4 "%SOURCE%" "%OUTPUT%" || exit /b 1
call "%TOOLS%\apksigner.bat" sign --ks "%USERPROFILE%\.android\debug.keystore" --ks-key-alias androiddebugkey --ks-pass pass:android --key-pass pass:android "%OUTPUT%" || exit /b 1
call "%TOOLS%\apksigner.bat" verify --verbose "%OUTPUT%" || exit /b 1
"%TOOLS%\zipalign.exe" -c -P 16 4 "%OUTPUT%" || exit /b 1
certutil -hashfile "%OUTPUT%" SHA256
echo APK: %OUTPUT%
exit /b 0
