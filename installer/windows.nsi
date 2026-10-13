Unicode true
RequestExecutionLevel user
!include "MUI2.nsh"
!include "LogicLib.nsh"

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

!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\ZHKU-NetAutoAuth.exe"
!define MUI_FINISHPAGE_RUN_TEXT "启动 ZHKU-NetAutoAuth"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
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
  ; Account credentials and desktop settings are preserved.
SectionEnd
