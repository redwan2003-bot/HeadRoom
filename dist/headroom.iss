; Inno Setup Script for Headroom v1.0
#define MyAppName "Headroom"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "Headroom Contributors"
#define MyAppURL "https://github.com/redwan2003-bot/HeadRoom"
#define MyAppExeName "headroom-agent.exe"

[Setup]
AppId={{E681C370-13D4-48E6-9DCB-8B4019E0228E}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={localappdata}\Programs\{#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=..\target\dist
OutputBaseFilename=Headroom-Setup-v1.0.0
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64compatible or arm64
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startupicon"; Description: "Start Headroom automatically at login"; GroupDescription: "Startup:"

[Files]
Source: "..\target\release\headroom-agent.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\headroom-dashboard.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\headroom-cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}\Headroom"; Filename: "{app}\{#MyAppExeName}"
Name: "{autoprograms}\{#MyAppName}\Diagnostic Dashboard"; Filename: "{app}\headroom-dashboard.exe"
Name: "{autoprograms}\{#MyAppName}\Uninstall Headroom"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon
Name: "{userstartup}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: startupicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
