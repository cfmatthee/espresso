Unicode true

!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.1.0"
!endif

Name "Espresso"
Caption "Espresso Setup"
OutFile "..\target\installers\espresso-${VERSION}-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\Espresso"
InstallDirRegKey HKCU "Software\CF Matthee\Espresso" "InstallDir"
RequestExecutionLevel user
ShowInstDetails show
ShowUninstDetails show

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "Espresso"
VIAddVersionKey "CompanyName" "CF Matthee"
VIAddVersionKey "FileDescription" "Prevents Windows from sleeping"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "Unlicense"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Function .onInit
  SetShellVarContext current
FunctionEnd

Function un.onInit
  SetShellVarContext current
FunctionEnd

Section "Espresso" MainSection
  SetOutPath "$INSTDIR"
  File "..\target\release\espresso.exe"

  CreateDirectory "$SMPROGRAMS\Espresso"
  CreateShortcut "$SMPROGRAMS\Espresso\Espresso.lnk" "$INSTDIR\espresso.exe"

  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\CF Matthee\Espresso" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Espresso" "DisplayName" "Espresso"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Espresso" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Espresso" "Publisher" "CF Matthee"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Espresso" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Espresso" "UninstallString" '"$INSTDIR\Uninstall.exe"'
SectionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\Espresso\Espresso.lnk"
  RMDir "$SMPROGRAMS\Espresso"
  Delete "$INSTDIR\espresso.exe"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Espresso"
  DeleteRegKey HKCU "Software\CF Matthee\Espresso"
SectionEnd
