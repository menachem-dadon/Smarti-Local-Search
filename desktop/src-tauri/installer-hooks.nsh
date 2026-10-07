; Restart Manager can report a failed shutdown when closing the parent also
; closes its inference job. Verify actual remaining file users in fresh sessions
; before deciding whether an upgrade is blocked.
!macro SmartiCloseRuntime executablePath
  !define SmartiCloseID ${__LINE__}
  !insertmacro RestartManager_StartSession $R0
  ${If} $R0 == ""
    SetErrorLevel 3
    Abort "$(failedToKillApp)"
  ${EndIf}
  !insertmacro RestartManager_RegisterFile $R0 "${executablePath}"
  ${If} $0 = 0
    System::Call 'RSTRTMGR::RmGetList(i R0, *i 0, *i 0, p 0, *i 0) i .r0'
    ${If} $0 = ${ERROR_MORE_DATA}
      IfSilent smarti_shutdown_${SmartiCloseID} 0
      nsis_tauri_utils::StrReplace "$(appRunningOkKill)" "{{product_name}}" "${PRODUCTNAME}"
      Pop $R2
      MessageBox MB_OKCANCEL $R2 IDOK smarti_shutdown_${SmartiCloseID}
      !insertmacro RestartManager_EndSession $R0
      Abort
      smarti_shutdown_${SmartiCloseID}:
      System::Call 'RSTRTMGR::RmShutdown(i R0, i ${RmForceShutdown}, p 0) i .r0'
    ${EndIf}
  ${EndIf}
  !insertmacro RestartManager_EndSession $R0
  StrCpy $R4 40
  smarti_wait_${SmartiCloseID}:
    !insertmacro RestartManager_StartSession $R0
    ${If} $R0 == ""
      SetErrorLevel 3
      Abort "$(failedToKillApp)"
    ${EndIf}
    !insertmacro RestartManager_RegisterFile $R0 "${executablePath}"
    ${If} $0 = 0
      System::Call 'RSTRTMGR::RmGetList(i R0, *i 0, *i 0, p 0, *i 0) i .r0'
    ${EndIf}
    !insertmacro RestartManager_EndSession $R0
    ${If} $0 = 0
      Goto smarti_closed_${SmartiCloseID}
    ${EndIf}
    IntOp $R4 $R4 - 1
    ${If} $R4 > 0
      Sleep 250
      Goto smarti_wait_${SmartiCloseID}
    ${EndIf}
    SetErrorLevel 3
    Abort "$(failedToKillApp)"
  smarti_closed_${SmartiCloseID}:
  !undef SmartiCloseID
!macroend

; Keep one installation directory and one uninstall entry across upgrades.
!macro NSIS_HOOK_PREINSTALL
  ReadRegStr $0 SHCTX "${UNINSTKEY}" "DisplayVersion"
  ${If} $0 != ""
    nsis_tauri_utils::SemverCompare "${VERSION}" $0
    Pop $1
    ${If} $1 = -1
      SetErrorLevel 2
      Abort "$(newerVersionInstalled)"
    ${EndIf}
  ${EndIf}
  ReadRegStr $0 SHCTX "${MANUPRODUCTKEY}" ""
  ${If} $0 != ""
    ${If} ${FileExists} "$0\${MAINBINARYNAME}.exe"
      StrCpy $INSTDIR $0
    ${ElseIf} ${FileExists} "$LOCALAPPDATA\${PRODUCTNAME}\${MAINBINARYNAME}.exe"
      ; Recover a real default installation when an earlier removed installation
      ; left a stale saved path. Never create a second copy beside that app.
      StrCpy $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}"
    ${EndIf}
  ${EndIf}
  SetOutPath "$INSTDIR"
  !insertmacro SmartiCloseRuntime "$INSTDIR\${MAINBINARYNAME}.exe"
  !insertmacro SmartiCloseRuntime "$INSTDIR\resources\inference\smarti-local-search-inference.exe"
  ; This directory contains the packaged runtime only. User data/cache and
  ; location.json are siblings and are never removed during installation.
  RmDir /r "$INSTDIR\resources\inference"
  ${If} ${FileExists} "$INSTDIR\resources\inference\*.*"
    SetErrorLevel 3
    Abort "$(failedToKillApp)"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro SmartiCloseRuntime "$INSTDIR\${MAINBINARYNAME}.exe"
  !insertmacro SmartiCloseRuntime "$INSTDIR\resources\inference\smarti-local-search-inference.exe"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !if "${BUNDLEID}" == "com.smarti.localsearch"
    ${If} $UpdateMode <> 1
      DeleteRegKey HKCU "Software\Classes\*\shell\SmartiLocalSearch"
      DeleteRegKey HKCU "Software\Classes\Directory\shell\SmartiLocalSearch"
      DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "SmartiLocalSearch"
    ${EndIf}
  !endif
!macroend
