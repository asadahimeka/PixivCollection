@echo off
@REM set http_proxy=http://127.0.0.1:7890
@REM set https_proxy=http://127.0.0.1:7890
"%~dp0\node.exe" "%~dp0\pxder" --debug -b -M && "%~dp0\node.exe" "%~dp0\scripts\rename.mjs" && "%~dp0\node.exe" "%~dp0\scripts\compress.mjs"
del /Q "%~dp0\data\tmp\*"
echo Update Bookmark End.
