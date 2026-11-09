; Hooks de l'installeur NSIS (Tauri). Le nœud tourne après la fermeture de l'application : il doit
; être arrêté avant d'écraser ou de retirer son exécutable (fichier verrouillé sous Windows).

!macro NSIS_HOOK_PREINSTALL
  nsExec::Exec 'taskkill /F /IM silicium-node.exe'
  Pop $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec 'taskkill /F /IM silicium-node.exe'
  Pop $0
!macroend
