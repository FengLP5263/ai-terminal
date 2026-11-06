; Native per-user installer. Generated includes contain only validated package paths.
Unicode true
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetCompressorDictSize 16
ManifestDPIAware true
ManifestSupportedOS Win10
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"
!include "WinVer.nsh"
!include "build-config.nsh"
!macro Trace TEXT
    !ifdef ACCEPTANCE
        Push $R8
        Push $R9
        ${If} ${Errors}
            StrCpy $R8 1
        ${Else}
            StrCpy $R8 0
        ${EndIf}
        FileOpen $R9 "$EXEDIR\acceptance-run.log" a
        FileSeek $R9 0 END
        FileWriteUTF16LE $R9 "${TEXT}$\r$\n"
        FileClose $R9
        ${If} $R8 == 1
            SetErrors
        ${Else}
            ClearErrors
        ${EndIf}
        Pop $R9
        Pop $R8
    !endif
!macroend

Name "${PRODUCT_NAME}"
OutFile "${OUTPUT_EXE}"
InstallDir "$LOCALAPPDATA\Programs\${PRODUCT_FOLDER}"
BrandingText "AI Terminal"
VIProductVersion "${NUMERIC_VERSION}"
VIAddVersionKey /LANG=1033 "ProductName" "${PRODUCT_NAME}"
VIAddVersionKey /LANG=1033 "FileDescription" "AI Terminal Setup"
VIAddVersionKey /LANG=1033 "FileVersion" "${APP_VERSION}"
VIAddVersionKey /LANG=1033 "LegalCopyright" "AI Terminal contributors"
Icon "${APP_ICON}"
UninstallIcon "${APP_ICON}"
ShowInstDetails show
ShowUninstDetails show
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\ai-terminal.exe"
!define MUI_FINISHPAGE_RUN_NOTCHECKED
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${APP_LICENSE}"
!insertmacro MUI_PAGE_COMPONENTS
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE ValidateTarget
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!define MUI_UNCONFIRMPAGE_TEXT_TOP "$(KeepData)"
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimpChinese"

LangString AppFiles ${LANG_ENGLISH} "Program files (required)"
LangString AppFiles ${LANG_SIMPCHINESE} "程序文件（必需）"
LangString DesktopLink ${LANG_ENGLISH} "Desktop shortcut"
LangString DesktopLink ${LANG_SIMPCHINESE} "桌面快捷方式"
LangString Busy ${LANG_ENGLISH} "Files in this installation are in use. Close AI Terminal from this folder and retry. Your sessions and settings are kept."
LangString Busy ${LANG_SIMPCHINESE} "此安装目录的文件正在使用中。请关闭从该目录启动的 AI Terminal 后重试。会话与设置会保留。"
LangString BadFolder ${LANG_ENGLISH} "Choose an empty folder or an existing AI Terminal installation. Session data folders and redirected folders cannot be installation targets."
LangString BadFolder ${LANG_SIMPCHINESE} "请选择空目录或已有 AI Terminal 安装目录。不能安装到会话数据目录或重定向目录。"
LangString Unsupported ${LANG_ENGLISH} "AI Terminal requires 64-bit Windows 10 version 1809 or later."
LangString Unsupported ${LANG_SIMPCHINESE} "AI Terminal 需要 64 位 Windows 10 1809 或更新版本。"
LangString KeepData ${LANG_ENGLISH} "Uninstall removes program files. Sessions, settings and Agent CLI installations are kept."
LangString KeepData ${LANG_SIMPCHINESE} "卸载仅移除程序文件。会话、设置和已安装的 Agent CLI 将保留。"

Function .onInit
    !insertmacro Trace "init $INSTDIR"
    SetShellVarContext current
    SetRegView 64
    ; InstallDirRegKey ignores SetRegView. Restore the 64-bit registration here,
    ; but keep NSIS's explicit /D= command-line override when one was supplied.
    ${GetParameters} $0
    ClearErrors
    ${GetOptionsS} $0 "/D=" $1
    ${If} ${Errors}
        ClearErrors
        ReadRegStr $1 HKCU "${UNINSTALL_KEY}" "InstallLocation"
        ${IfNot} ${Errors}
        ${AndIf} $1 != ""
            StrCpy $INSTDIR $1
        ${EndIf}
    ${EndIf}
    ClearErrors
    ${IfNot} ${RunningX64}
        !insertmacro Trace "unsupported architecture"
        SetErrorLevel 2
        MessageBox MB_OK|MB_ICONSTOP "$(Unsupported)" /SD IDOK
        Abort
    ${EndIf}
    ${IfNot} ${AtLeastWin10}
        !insertmacro Trace "unsupported Windows version"
        SetErrorLevel 2
        MessageBox MB_OK|MB_ICONSTOP "$(Unsupported)" /SD IDOK
        Abort
    ${EndIf}
    ReadRegStr $0 HKLM "SOFTWARE\Microsoft\Windows NT\CurrentVersion" "CurrentBuildNumber"
    !insertmacro Trace "Windows build $0"
    IntCmp $0 17763 supported unsupported supported
    unsupported:
        !insertmacro Trace "unsupported Windows build"
        SetErrorLevel 2
        MessageBox MB_OK|MB_ICONSTOP "$(Unsupported)" /SD IDOK
        Abort
    supported:
    !insertmacro Trace "supported Windows"
    !insertmacro MUI_LANGDLL_DISPLAY
FunctionEnd

; Reject a junction before extracting/deleting. No recursive directory deletion.
!macro CheckDirectory DIRECTORY
    System::Call 'kernel32::GetFileAttributesW(w "${DIRECTORY}") i.r0'
    !insertmacro Trace "directory attributes $0 ${DIRECTORY}"
    ${If} $0 != -1
        IntOp $0 $0 & 0x400
        ${If} $0 != 0
            SetErrorLevel 2
            MessageBox MB_OK|MB_ICONSTOP "$(BadFolder)" /SD IDOK
            Abort
        ${EndIf}
    ${EndIf}
!macroend

!macro CheckOwned PREFIX
Function ${PREFIX}CheckOwned
    !insertmacro CheckDirectory "$INSTDIR"
    ClearErrors
    FileOpen $0 "$INSTDIR\.ai-terminal-installed" r
    IfErrors bad_owned
    FileRead $0 $1
    FileRead $0 $2
    FileClose $0
    StrCmp $1 "ai-terminal-install-v1$\r$\n" 0 bad_owned
    StrCmp $2 "${PRODUCT_KEY}$\r$\n" owned_good bad_owned
    bad_owned:
        !insertmacro Trace "ownership marker rejected"
        SetErrorLevel 2
        MessageBox MB_OK|MB_ICONSTOP "$(BadFolder)" /SD IDOK
        Abort
    owned_good:
        ClearErrors
FunctionEnd
!macroend
!insertmacro CheckOwned ""
!insertmacro CheckOwned "un."

Function ValidateTarget
    !insertmacro Trace "validate target $INSTDIR"
    ; GetFullPathName's NSIS instruction requires existing path components.
    ; The Win32 API also normalizes a new installation directory.
    System::Call 'kernel32::GetFullPathNameW(w "$INSTDIR", i ${NSIS_MAX_STRLEN}, w .r0, p 0) i.r1'
    ${If} $1 == 0
    ${OrIf} $1 >= ${NSIS_MAX_STRLEN}
        Goto invalid_target
    ${EndIf}
    StrCpy $INSTDIR $0
    !insertmacro Trace "canonical target $INSTDIR"
    System::Call 'kernel32::GetFullPathNameW(w "$LOCALAPPDATA\ai-terminal", i ${NSIS_MAX_STRLEN}, w .r0, p 0) i.r1'
    StrCmp $INSTDIR $0 invalid_target
    !insertmacro CheckDirectory "$INSTDIR"
    IfFileExists "$INSTDIR\.ai-terminal-installed" existing_target
    ClearErrors
    FindFirst $0 $1 "$INSTDIR\*"
    IfErrors empty_target
    enumerate:
        StrCmp $1 "" empty_close
        StrCmp $1 "." next_entry
        StrCmp $1 ".." next_entry
        FindClose $0
        Goto invalid_target
    next_entry:
        FindNext $0 $1
        Goto enumerate
    empty_close:
        FindClose $0
    empty_target:
        !insertmacro Trace "empty target accepted"
        ClearErrors
        Return
    existing_target:
        Call CheckOwned
        Return
    invalid_target:
        !insertmacro Trace "invalid target"
        SetErrorLevel 2
        MessageBox MB_OK|MB_ICONSTOP "$(BadFolder)" /SD IDOK
        Abort
FunctionEnd

; A write/share probe never closes an application or writes to its files.
!macro CheckFile FILENAME
    ${If} ${FileExists} "$INSTDIR\${FILENAME}"
        System::Call 'kernel32::CreateFileW(w "$INSTDIR\${FILENAME}", i 0x40000000, i 0, p 0, i 3, i 0x80, p 0) p.r0'
        ${If} $0 == -1
            !insertmacro Trace "busy ${FILENAME}"
            SetErrorLevel 3
            MessageBox MB_OK|MB_ICONSTOP "$(Busy)" /SD IDOK
            Abort
        ${Else}
            System::Call 'kernel32::CloseHandle(p r0)'
        ${EndIf}
    ${EndIf}
!macroend

Section "$(AppFiles)" SEC_PROGRAM
    !insertmacro Trace "install section $INSTDIR"
    SectionIn RO
    SetShellVarContext current
    SetRegView 64
    Call ValidateTarget
    !include "check-files.nsh"
    !include "install-files.nsh"
    SetOutPath "$INSTDIR"
    ClearErrors
    WriteUninstaller "$INSTDIR\uninstall.exe"
    IfErrors failed_install
    FileOpen $0 "$INSTDIR\.ai-terminal-installed" w
    FileWrite $0 "ai-terminal-install-v1$\r$\n${PRODUCT_KEY}$\r$\n${BUILD_ID}$\r$\n"
    FileClose $0
    IfErrors failed_install
    WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${PRODUCT_NAME}"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${APP_VERSION}"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "AI Terminal contributors"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '$\"$INSTDIR\uninstall.exe$\"'
    WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '$\"$INSTDIR\uninstall.exe$\" /S'
    WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\ai-terminal.exe"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "URLInfoAbout" "https://github.com/FengLP5263/ai-terminal"
    WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
    WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
    WriteRegDWORD HKCU "${UNINSTALL_KEY}" "EstimatedSize" ${INSTALLED_KIB}
    IfErrors failed_install
    CreateDirectory "$SMPROGRAMS\${PRODUCT_FOLDER}"
    CreateShortcut "$SMPROGRAMS\${PRODUCT_FOLDER}\AI Terminal.lnk" "$INSTDIR\ai-terminal.exe"
    CreateShortcut "$SMPROGRAMS\${PRODUCT_FOLDER}\Uninstall.lnk" "$INSTDIR\uninstall.exe"
    Goto installed
    failed_install:
        !insertmacro Trace "file installation failed"
        SetErrorLevel 4
        Abort
    installed:
        !insertmacro Trace "installed"
        SetErrorLevel 0
SectionEnd

Section /o "$(DesktopLink)" SEC_DESKTOP
    CreateShortcut "$DESKTOP\${PRODUCT_FOLDER}.lnk" "$INSTDIR\ai-terminal.exe"
    WriteRegDWORD HKCU "${UNINSTALL_KEY}" "DesktopShortcut" 1
SectionEnd

Section "Uninstall"
    SetShellVarContext current
    SetRegView 64
    Call un.CheckOwned
    !include "check-files.nsh"
    !include "uninstall-files.nsh"
    ReadRegDWORD $0 HKCU "${UNINSTALL_KEY}" "DesktopShortcut"
    ${If} $0 == 1
        Delete "$DESKTOP\${PRODUCT_FOLDER}.lnk"
    ${EndIf}
    Delete "$SMPROGRAMS\${PRODUCT_FOLDER}\AI Terminal.lnk"
    Delete "$SMPROGRAMS\${PRODUCT_FOLDER}\Uninstall.lnk"
    RMDir "$SMPROGRAMS\${PRODUCT_FOLDER}"
    Delete "$INSTDIR\.ai-terminal-installed"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"
    DeleteRegKey HKCU "${UNINSTALL_KEY}"
SectionEnd
