!include "WordFunc.nsh"

; Refuse to install if the currently installed version is newer than this one.
; Allows Intune-managed upgrades while preventing accidental downgrades when a
; user manually runs an older installer on a machine that already has a newer build.
!macro customInstall
  ReadRegStr $R0 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\TaskTracker Extreme 3000" "DisplayVersion"
  ${If} $R0 != ""
    ${VersionCompare} "$R0" "${VERSION}" $R1
    ${If} $R1 == 1
      IfSilent +2
        MessageBox MB_OK "A newer version ($R0) is already installed. This installer will not overwrite it."
      Abort
    ${EndIf}
  ${EndIf}
!macroend
