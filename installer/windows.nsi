Unicode true
RequestExecutionLevel user
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "nsDialogs.nsh"

!ifndef APP_VERSION
  !error "APP_VERSION is required"
!endif
!ifndef APP_EXE
  !error "APP_EXE is required"
!endif
!ifndef OUT_FILE
  !error "OUT_FILE is required"
!endif

Name "ZHKU-NetAutoAuth"
OutFile "${OUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\ZHKU-NetAutoAuth"
SetCompressor /SOLID lzma
ShowInstDetails show
ShowUninstDetails show
Var DeleteUserData
Var DeleteUserDataCheckbox

!macro DeleteUserFile FilePath
  IfFileExists "${FilePath}" 0 +2
    Delete "${FilePath}"
!macroend

!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\ZHKU-NetAutoAuth.exe"
!define MUI_FINISHPAGE_RUN_TEXT "启动 ZHKU-NetAutoAuth"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
UninstPage custom un.UserDataPage un.UserDataPageLeave
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Function .onInit
  SetRegView 64
  ${If} $INSTDIR == "$LOCALAPPDATA\Programs\ZHKU-NetAutoAuth"
    ReadRegStr $0 HKCU "Software\ZHKU-NetAutoAuth" "InstallDir"
    ${If} $0 != ""
      StrCpy $INSTDIR $0
    ${EndIf}
  ${EndIf}
FunctionEnd

Function un.onInit
  SetRegView 64
  StrCpy $DeleteUserData ${BST_UNCHECKED}
FunctionEnd

Function un.UserDataPage
  !insertmacro MUI_HEADER_TEXT "账号与设置" "默认保留，重新安装后可以继续使用。"
  nsDialogs::Create 1018
  Pop $0
  ${If} $0 == error
    Abort
  ${EndIf}
  ${NSD_CreateLabel} 0 0 100% 32u "账号与设置由安装版和便携版共用。选择删除后，便携版也需要重新配置账号。"
  Pop $0
  ${NSD_CreateCheckbox} 0 42u 100% 16u "同时删除账号与设置"
  Pop $DeleteUserDataCheckbox
  ${NSD_SetState} $DeleteUserDataCheckbox $DeleteUserData
  nsDialogs::Show
FunctionEnd

Function un.UserDataPageLeave
  ${NSD_GetState} $DeleteUserDataCheckbox $DeleteUserData
FunctionEnd

Section "安装"
  SetRegView 64
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  ClearErrors
  SetOverwrite on
  File /oname=ZHKU-NetAutoAuth.exe "${APP_EXE}"
  ${If} ${Errors}
    IfSilent +2
      MessageBox MB_ICONSTOP "无法更新程序文件，请先退出正在运行的 ZHKU-NetAutoAuth 后重试。"
    SetErrorLevel 1
    Abort
  ${EndIf}
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\ZHKU-NetAutoAuth"
  CreateShortcut "$SMPROGRAMS\ZHKU-NetAutoAuth\ZHKU-NetAutoAuth.lnk" "$INSTDIR\ZHKU-NetAutoAuth.exe"
  CreateShortcut "$SMPROGRAMS\ZHKU-NetAutoAuth\卸载.lnk" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\ZHKU-NetAutoAuth" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ZHKU-NetAutoAuth" "DisplayName" "ZHKU-NetAutoAuth"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ZHKU-NetAutoAuth" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ZHKU-NetAutoAuth" "Publisher" "ZHKU-NetAutoAuth"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ZHKU-NetAutoAuth" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ZHKU-NetAutoAuth" "DisplayIcon" "$INSTDIR\ZHKU-NetAutoAuth.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ZHKU-NetAutoAuth" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ZHKU-NetAutoAuth" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ZHKU-NetAutoAuth" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ZHKU-NetAutoAuth" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetRegView 64
  SetShellVarContext current
  ClearErrors
  Delete "$INSTDIR\ZHKU-NetAutoAuth.exe"
  ${If} ${Errors}
    IfSilent +2
      MessageBox MB_ICONSTOP "请先退出 ZHKU-NetAutoAuth，再执行卸载。"
    SetErrorLevel 1
    Abort
  ${EndIf}
  Delete "$SMPROGRAMS\ZHKU-NetAutoAuth\ZHKU-NetAutoAuth.lnk"
  Delete "$SMPROGRAMS\ZHKU-NetAutoAuth\卸载.lnk"
  RMDir "$SMPROGRAMS\ZHKU-NetAutoAuth"
  DeleteRegKey HKCU "Software\ZHKU-NetAutoAuth"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ZHKU-NetAutoAuth"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  ${If} $DeleteUserData == ${BST_CHECKED}
    ; Delete only application-owned files; never recursively remove user data.
    ClearErrors
    !insertmacro DeleteUserFile "$LOCALAPPDATA\ZHKU-NetAutoAuth\rust-gui\credentials.dat"
    !insertmacro DeleteUserFile "$LOCALAPPDATA\ZHKU-NetAutoAuth\rust-gui\credentials.tmp"
    !insertmacro DeleteUserFile "$LOCALAPPDATA\ZHKU-NetAutoAuth\rust-gui\desktop.json"
    ${IfNot} ${Errors}
      !insertmacro DeleteUserFile "$LOCALAPPDATA\ZHKU-NetAutoAuth\credentials.dat"
      !insertmacro DeleteUserFile "$LOCALAPPDATA\ZHKU-NetAutoAuth\credentials.tmp"
      !insertmacro DeleteUserFile "$LOCALAPPDATA\ZHKU-NetAutoAuth\desktop.json"
      !insertmacro DeleteUserFile "$LOCALAPPDATA\ZHKU-NetAutoAuth\network-tests.json"
    ${EndIf}
    ${If} ${Errors}
      IfSilent +2
        MessageBox MB_ICONEXCLAMATION "部分账号或设置文件无法删除，请检查文件权限。"
      SetErrorLevel 1
    ${EndIf}
    RMDir "$LOCALAPPDATA\ZHKU-NetAutoAuth\rust-gui"
    RMDir "$LOCALAPPDATA\ZHKU-NetAutoAuth"
  ${EndIf}
SectionEnd
