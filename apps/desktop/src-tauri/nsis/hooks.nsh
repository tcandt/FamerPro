; FarmerPro NSIS Installer Hooks (P5.4)
; Guarantees standard-user safe install/upgrade/uninstall while NEVER deleting user data, SQLite DBs, or GameProjects!

!macro NSIS_HOOK_PREINSTALL
  ; Preserve marker file so FarmerPro.exe isolates user databases in %LOCALAPPDATA%\FarmerPro\StudioData
  DetailPrint "FarmerPro Studio: Preparing clean installation preserving %LOCALAPPDATA%\FarmerPro\StudioData..."
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Create an explicit marker file next to FarmerPro.exe so Studio detects NSIS-installed mode deterministically
  FileOpen $0 "$INSTDIR\.nsis_installed" w
  FileWrite $0 "installed_by_nsis=true$\r$\n"
  FileClose $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Explicitly clean only the NSIS install marker inside $INSTDIR; NEVER remove %LOCALAPPDATA%\FarmerPro\StudioData
  Delete "$INSTDIR\.nsis_installed"
  DetailPrint "FarmerPro Uninstall: Preserving user GameProjects, SQLite databases, and vault credentials under %LOCALAPPDATA%\FarmerPro\StudioData."
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Do not wipe $LOCALAPPDATA\FarmerPro or $INSTDIR\data if user created portable projects alongside
  DetailPrint "FarmerPro Uninstall Complete: User data preserved intact."
!macroend
