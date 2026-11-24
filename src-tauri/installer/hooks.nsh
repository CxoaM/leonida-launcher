; Leonida Launcher installer hooks.
; Kept deliberately small: the Tauri/NSIS template owns the install flow, these
; hooks only add the optional desktop shortcut and the data-removal questions.

!macro NSIS_HOOK_POSTINSTALL
  MessageBox MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON1 "Create a desktop shortcut for Leonida Launcher?" /SD IDYES IDNO ll_no_desktop_shortcut
    CreateShortCut "$DESKTOP\Leonida Launcher.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  ll_no_desktop_shortcut:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Settings, logs and cache. Never removed unless the user says yes.
  MessageBox MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON2 "Remove your Leonida Launcher settings, logs and cache?$\r$\n$\r$\nThis does not touch the Leonida sources or your game files." /SD IDNO IDNO ll_keep_settings
    RMDir /r "$APPDATA\dev.leonida.launcher"
    RMDir /r "$LOCALAPPDATA\dev.leonida.launcher\cache"
    RMDir /r "$LOCALAPPDATA\dev.leonida.launcher\workspace\work"
  ll_keep_settings:

  ; Leonida sources and build output. Asked separately, because these can be
  ; hundreds of gigabytes and represent hours of work.
  MessageBox MB_YESNO|MB_ICONEXCLAMATION|MB_DEFBUTTON2 "Also delete the Leonida source checkout and build output?$\r$\n$\r$\nThis removes the default folders:$\r$\n  $LOCALAPPDATA\dev.leonida.launcher\workspace$\r$\n  $DOCUMENTS\Leonida$\r$\n$\r$\nThis cannot be undone." /SD IDNO IDNO ll_keep_sources
    RMDir /r "$LOCALAPPDATA\dev.leonida.launcher\workspace"
    RMDir /r "$DOCUMENTS\Leonida"
  ll_keep_sources:
!macroend
