@echo off
del /Q "%~dp0\data\tmp\*"
"%~dp0\node.exe" "%~dp0\pxder" --debug -b -M && "%~dp0\node.exe" "%~dp0\scripts\rename.mjs" && "%~dp0\node.exe" "%~dp0\scripts\compress.mjs"
echo Update Bookmark End.
