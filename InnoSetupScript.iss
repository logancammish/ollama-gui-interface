[Setup]
AppName=ollama-gui
AppVersion=0.5.0
; Install per-user so setup and the application never require elevation.
DefaultDirName={localappdata}\Programs\ollama-gui
DefaultGroupName=ollama-gui
PrivilegesRequired=lowest
UsedUserAreasWarning=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=dist
OutputBaseFilename=ollama-gui-win64-installer
Compression=lzma
SolidCompression=yes

[Files]
Source: "target\x86_64-pc-windows-gnu\release\ollama-gui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "assets\*"; DestDir: "{app}\assets"; Flags: recursesubdirs createallsubdirs
Source: "config\*"; DestDir: "{app}\config"; Flags: recursesubdirs createallsubdirs

[Icons]
Name: "{group}\ollama-gui"; Filename: "{app}\ollama-gui.exe"; WorkingDir: "{app}"
Name: "{userdesktop}\ollama-gui"; Filename: "{app}\ollama-gui.exe"; WorkingDir: "{app}"

[Run]
Filename: "{app}\ollama-gui.exe"; Description: "Launch Ollama GUI Interface"; WorkingDir: "{app}"; Flags: nowait postinstall skipifsilent
