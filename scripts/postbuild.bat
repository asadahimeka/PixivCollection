@echo off
mkdir "%~dp0\PixivCollection"
copy "%~dp0\..\src-tauri\target\x86_64-pc-windows-msvc\release\PixivCollection.exe" "%~dp0\PixivCollection\"
xcopy "%~dp0\..\pxder" "%~dp0\PixivCollection\pxder\" /s /y
del /Q "%~dp0\PixivCollection\pxder\readme.md"
del /Q "%~dp0\PixivCollection\pxder\data\*"
del /Q "%~dp0\PixivCollection\pxder\src\config\config.json"
move "%~dp0PixivCollection\pxder\src\config\config.init.json" "%~dp0PixivCollection\pxder\src\config\config.json"
for /R "%~dp0\PixivCollection\pxder\node_modules" %%f in (*.map) do del /Q "%%f"
for /R "%~dp0\PixivCollection\pxder\node_modules" %%f in (*.d.ts) do del /Q "%%f"
for /R "%~dp0\PixivCollection\pxder\node_modules" %%f in (*.d.cts) do del /Q "%%f"
for /R "%~dp0\PixivCollection\pxder\node_modules" %%f in (*.d.mts) do del /Q "%%f"
for /R "%~dp0\PixivCollection\pxder\node_modules" %%f in (*.md) do del /Q "%%f"
bz.exe c -l:9 -r "%~dp0\PixivCollection.7z" "%~dp0\PixivCollection"
rmdir /Q /S "%~dp0\PixivCollection"
