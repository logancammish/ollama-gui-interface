[Setup]
AppName=ollama-gui
AppVersion=0.3.4
DefaultDirName={pf}\ollama-gui
DefaultGroupName=ollama-gui
OutputDir=.
OutputBaseFilename=ollama-gui-win64-installer
Compression=lzma
SolidCompression=yes

[Files]
Source: "C:\Users\L.J.Cammish\OneDrive - Saint Kentigern\Documents\ollama-gui\target\release\ollama-gui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\Users\L.J.Cammish\OneDrive - Saint Kentigern\Documents\ollama-gui\assets\*"; DestDir: "{app}\assets"; Flags: recursesubdirs createallsubdirs
Source: "C:\Users\L.J.Cammish\OneDrive - Saint Kentigern\Documents\ollama-gui\config\*"; DestDir: "{app}\config"; Flags: recursesubdirs createallsubdirs
Source: "C:\Users\L.J.Cammish\OneDrive - Saint Kentigern\Documents\ollama-gui\output\*"; DestDir: "{app}\output"; Flags: recursesubdirs createallsubdirs

[Icons]
Name: "{group}\ollama-gui"; Filename: "{app}\ollama-gui.exe"
Name: "{commondesktop}\ollama-gui"; Filename: "{app}\ollama-gui.exe"

[Run]
Filename: "{app}\ollama-gui.exe"; Description: "Launch Ollama GUI Interface"; Flags: nowait postinstall skipifsilent