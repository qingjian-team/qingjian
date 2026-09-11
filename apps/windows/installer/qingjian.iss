; 青简 Windows 输入法安装脚本（Inno Setup）。
;
; 装到 Program Files\Qingjian（64 位），把 TSF DLL、Server、设置程序与随包数据装在一起，
; 然后：① 给安装目录加 ALL APPLICATION PACKAGES 读+执行权限（UWP/AppContainer 应用——任务栏搜索、
; 设置——才能加载 DLL）；② regsvr32 注册文本服务（写 HKCR，图标落到 %ProgramData%\Qingjian）；
; ③ 在「启动」文件夹放 Server 快捷方式（登录时由 Explorer 走 ShellExecute 拉起，uiAccess 才生效——
;    计划任务直接拉起拿不到 uiAccess）；④ 装完点 Finish 立即以原用户 ShellExecute 起一次 Server，免得先注销。
; 卸载反向：删旧任务（若有）、杀 Server、反注册 DLL，再删文件（用户数据 %APPDATA%\Qingjian 保留；启动快捷方式 Inno 自动删）。
;
; 版本号由打包脚本用 /DAppVersion=... 传入，缺省 0.1.0。用法见本目录 README.md。

#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#define AppName "青简"
#define Publisher "青简"
#define WebsiteUrl "https://qingjian.im"
; 脚本相对仓库根（ime/）：installer → windows → apps → ime
#define Repo "..\..\.."

[Setup]
AppId={{A7E3C1F2-5B94-4D6A-9C0E-2F8B1D3A6E70}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#Publisher}
AppSupportURL={#WebsiteUrl}
VersionInfoVersion={#AppVersion}
DefaultDirName={autopf}\Qingjian
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin
OutputDir={#Repo}\target\installer
OutputBaseFilename=Qingjian-{#AppVersion}-Setup
SetupIconFile={#Repo}\apps\windows\tsf\resources\qingjian.ico
UninstallDisplayIcon={app}\qingjian-settings.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "chs"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"

[Files]
; —— 二进制 ——
Source: "{#Repo}\target\release\qingjian_tsf.dll";      DestDir: "{app}"; Flags: ignoreversion
Source: "{#Repo}\target\release\qingjian-server.exe";   DestDir: "{app}"; Flags: ignoreversion
Source: "{#Repo}\target\release\qingjian-settings.exe"; DestDir: "{app}"; Flags: ignoreversion
; —— 随包生成数据（只装运行时要的 .qj / .tsv，不装 dev 中间产物）——
Source: "{#Repo}\data\generated\dict.qj";        DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\lm.qj";          DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\glossary-en.qj"; DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\glossary-ja.qj"; DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\glossary-zh.qj"; DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\english.tsv";    DestDir: "{app}\data\generated";       Flags: ignoreversion
Source: "{#Repo}\data\generated\dicts\*.qj";     DestDir: "{app}\data\generated\dicts";  Flags: ignoreversion
; —— 随 git 的资源 ——
Source: "{#Repo}\assets\emoji\emoji-zh.tsv";     DestDir: "{app}\assets\emoji";  Flags: ignoreversion
Source: "{#Repo}\assets\emoji\emoji-en.tsv";     DestDir: "{app}\assets\emoji";  Flags: ignoreversion
Source: "{#Repo}\assets\levels\levels-en.tsv";   DestDir: "{app}\assets\levels"; Flags: ignoreversion
Source: "{#Repo}\assets\levels\levels-ja.tsv";   DestDir: "{app}\assets\levels"; Flags: ignoreversion
Source: "{#Repo}\assets\sample\dict.tsv";        DestDir: "{app}\assets\sample"; Flags: ignoreversion

[Icons]
Name: "{group}\青简设置"; Filename: "{app}\qingjian-settings.exe"
Name: "{group}\卸载青简"; Filename: "{uninstallexe}"
; 登录自启：登录时 Explorer 走 ShellExecute 拉起本快捷方式 → AppInfo 授予 uiAccess，候选窗才能盖过商店 / 任务栏搜索。
; 用 {commonstartup}（所有用户「启动」文件夹）而非 {userstartup}：本安装器是 admin 机器级安装，
; admin 模式下写每用户区会落到「谁提权就写谁」的 profile（Inno 会告警且可能不是目标用户）；
; 机器级「启动」项对每个登录用户都在其会话里由该用户的 Explorer 拉起，仍是 per-user 运行、仍授予 uiAccess。
; （计划任务直接拉起拿不到 uiAccess，故不用 schtasks。）
Name: "{commonstartup}\青简 Server"; Filename: "{app}\qingjian-server.exe"; WorkingDir: "{app}"

[Run]
; ① UWP/AppContainer 应用要能读安装目录才能加载 DLL（*S-1-15-2-1 = ALL APPLICATION PACKAGES，按 SID 与语言无关）。
Filename: "{sys}\icacls.exe"; Parameters: """{app}"" /grant *S-1-15-2-1:(OI)(CI)RX /T /C /Q"; \
  Flags: runhidden waituntilterminated; StatusMsg: "配置应用容器权限…"
; ② 注册文本服务（写 HKCR + 图标到 %ProgramData%\Qingjian\qingjian.ico）。
Filename: "{sys}\regsvr32.exe"; Parameters: "/s ""{app}\qingjian_tsf.dll"""; \
  Flags: runhidden waituntilterminated; StatusMsg: "注册输入法…"
; ④ 装完立即起一次 Server 见 [Code] 的 NextButtonClick：uiAccess=true 的 exe 不能用
;    CreateProcess / runasoriginaluser 拉起（报 740），必须走 ShellExecute（等同双击）。

[UninstallRun]
; 反向：先删登录任务、杀 Server、反注册 DLL，Inno 再删文件（DLL 若仍被占用，重启后删）。
Filename: "{sys}\schtasks.exe"; Parameters: "/delete /tn ""Qingjian Server"" /f"; \
  Flags: runhidden; RunOnceId: "DelLogonTask"
Filename: "{sys}\taskkill.exe"; Parameters: "/im qingjian-server.exe /f"; \
  Flags: runhidden; RunOnceId: "KillServer"
Filename: "{sys}\regsvr32.exe"; Parameters: "/u /s ""{app}\qingjian_tsf.dll"""; \
  Flags: runhidden; RunOnceId: "UnregDll"

[Code]
{ 清掉旧版本建的「登录自启」计划任务（现在改用「启动」文件夹快捷方式，见 [Icons]）。
  计划任务直接拉起 Server 拿不到 uiAccess，升级安装时删掉它，免得它在登录时抢先以非 uiAccess 方式
  起 Server 并占住命名管道，让快捷方式那份起不来。没有旧任务时 schtasks 返回非 0，忽略即可。 }
procedure DeleteLegacyLogonTask;
var
  ResultCode: Integer;
begin
  Exec('schtasks.exe', '/delete /tn "Qingjian Server" /f', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    DeleteLegacyLogonTask;
end;

{ 装完在完成页点 Finish 后立即起一次 Server。
  uiAccess=true 的 exe 不能用 CreateProcess / runasoriginaluser 拉起（报 740），
  必须以原（非提升）用户身份 ShellExecute（等同双击），AppInfo 才会授予 uiAccess 高 z-band 权限。 }
function NextButtonClick(CurPageID: Integer): Boolean;
var
  ErrorCode: Integer;
begin
  Result := True;
  if (CurPageID = wpFinished) and (not WizardSilent) then
    ShellExecAsOriginalUser(
      '', ExpandConstant('{app}\qingjian-server.exe'), '', '',
      SW_SHOWNORMAL, ewNoWait, ErrorCode);
end;
