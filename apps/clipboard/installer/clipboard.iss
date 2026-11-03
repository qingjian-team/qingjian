; 青简剪贴板的独立安装脚本（Inno Setup）。随输入法内置的那份由 apps\windows\installer\qingjian.iss 装，这里给只要剪贴板的人。
;
; 按用户装到 %LOCALAPPDATA%\Programs\Qingjian Clipboard，不要管理员；只有一个 exe，界面文案、图标都编在里面。
; 开机启动由托盘菜单开关（写 HKCU 的 Run），卸载时一并删掉；历史与配置在 %LOCALAPPDATA%\Qingjian 与 %APPDATA%\Qingjian，卸载不动。
; 装和卸之前结束正在跑的剪贴板：exe 被占着覆盖不了。输入法里那份同名，也会被结束，输入法下次读配置时再拉起。
;
; 版本号由 build.ps1 用 /DAppVersion=... 传入。

#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
; VersionInfoVersion 只认 a.b.c.d 数字；带预发布后缀时由打包脚本传去掉后缀的数字版本。
#ifndef AppVersionNumeric
  #define AppVersionNumeric AppVersion
#endif
#define AppName "青简剪贴板"
#define Publisher "青简"
#define WebsiteUrl "https://github.com/qingjian-team/qingjian-clipboard"
; 脚本相对仓库根（ime/）：installer → clipboard → apps → ime
#define Repo "..\..\.."

[Setup]
AppId={{5C2E8A41-7D36-4B9F-A0E5-91F4C6D23B87}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#Publisher}
AppSupportURL={#WebsiteUrl}
VersionInfoVersion={#AppVersionNumeric}
DefaultDirName={localappdata}\Programs\Qingjian Clipboard
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
PrivilegesRequired=lowest
OutputDir={#Repo}\target\installer
OutputBaseFilename=qingjian-clipboard-{#AppVersion}-windows-x86_64-setup
SetupIconFile={#Repo}\apps\windows\tsf\resources\qingjian.ico
UninstallDisplayIcon={app}\qingjian-clipboard.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "chs"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"

[Files]
Source: "{#Repo}\target\release\qingjian-clipboard.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Repo}\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\qingjian-clipboard.exe"
Name: "{group}\卸载{#AppName}"; Filename: "{uninstallexe}"

[Registry]
; 不在安装时写：只让卸载程序删掉托盘菜单写的开机启动项。
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "QingjianClipboard"; ValueType: none; Flags: dontcreatekey uninsdeletevalue

[Run]
Filename: "{app}\qingjian-clipboard.exe"; Description: "启动{#AppName}"; Flags: postinstall nowait skipifsilent

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/im qingjian-clipboard.exe /f"; Flags: runhidden waituntilterminated; RunOnceId: "StopClipboard"

[Code]
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Code: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/im qingjian-clipboard.exe /f', '', SW_HIDE, ewWaitUntilTerminated, Code);
  Result := '';
end;
