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
  !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
  !insertmacro CheckIfAppIsRunning "$INSTDIR\resources\inference\smarti-local-search-inference.exe" "${PRODUCTNAME}"
  ; This directory contains the packaged runtime only. User data/cache and
  ; location.json are siblings and are never removed during installation.
  RmDir /r "$INSTDIR\resources\inference"
  ${If} ${FileExists} "$INSTDIR\resources\inference\*.*"
    SetErrorLevel 3
    Abort "$(failedToKillApp)"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
  !insertmacro CheckIfAppIsRunning "$INSTDIR\resources\inference\smarti-local-search-inference.exe" "${PRODUCTNAME}"
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
