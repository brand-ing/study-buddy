; focus — Inno Setup installer script
;
; Before compiling:
;   cargo build --release
;
; Then either:
;   - Open this file in the Inno Setup IDE and press Compile (F9)
;   - Or from the command line: iscc focus.iss
;
; Output lands in installer\output\focus-setup-0.1.0.exe
;
; Inno Setup 6: https://jrsoftware.org/isinfo.php

#define AppName      "focus"
#define AppVersion   "0.0.4"
#define AppPublisher "brand-ing"
#define AppExeName   "focus.exe"
#define SourceExe    "target\release\" + AppExeName

[Setup]
; AppId uniquely identifies this app for updates and uninstall.
; Regenerate this GUID if you fork or redistribute under a different name.
AppId={{CA7F3B28-D594-4E1A-B062-8A3F50C2E917}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
; No admin rights needed — installs per-user under %LOCALAPPDATA%\Programs
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=installer\output
OutputBaseFilename=focus-setup-{#AppVersion}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#AppExeName}
UninstallDisplayName={#AppName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\{#AppName}";  Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
