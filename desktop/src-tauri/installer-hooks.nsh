!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegKey HKCU "Software\Classes\*\shell\SmartiLocalSearch"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\SmartiLocalSearch"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "SmartiLocalSearch"
!macroend
