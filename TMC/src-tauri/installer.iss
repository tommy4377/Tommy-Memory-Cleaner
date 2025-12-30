; Inno Setup Script per Tommy Memory Cleaner
; Installer personalizzato con interfaccia moderna e design elegante
; Creato da tommy437

#define AppName "Tommy Memory Cleaner"
#define AppVersion "1.0.0"
#define AppPublisher "tommy437"
#define AppURL "https://github.com/tommy437"
#define AppExeName "Tommy Memory Cleaner.exe"
#define AppId "9B8F5C4D-3E2A-4F1B-8C7D-6E5F4A3B2C1D"

[Setup]
AppId={{{#AppId}}}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
AllowNoIcons=yes
LicenseFile=
UninstallDisplayIcon={app}\{#AppExeName}
OutputDir=installer\output
CreateUninstallRegKey=yes
Uninstallable=yes
OutputBaseFilename=TommyMemoryCleaner-Setup
SetupIconFile=icons\icon.ico
Compression=lzma2/ultra
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesInstallIn64BitMode=x64
DisableProgramGroupPage=yes
DisableReadyPage=yes
DisableFinishedPage=no
DisableWelcomePage=yes
WizardImageStretch=no
WizardImageBackColor=$F5F5F5
; WizardImageFile=installer\wizard-sidebar.bmp  ; Commentato: file non presente
; WizardSmallImageFile=icons\icon-32.bmp  ; Commentato: file non presente, usiamo solo SetupIconFile
WizardResizable=no
WizardSizePercent=100,100

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "portuguese"; MessagesFile: "compiler:Languages\Portuguese.isl"

[CustomMessages]
english.FinishedLabel=Tommy Memory Cleaner has been installed on your computer.%n%nCreated by tommy437%n%nClick Finish to close this wizard.
english.CreateStartupIcon=Create a startup shortcut
italian.FinishedLabel=Tommy Memory Cleaner è stato installato sul tuo computer.%n%nCreato da tommy437%n%nClicca Fine per chiudere questa procedura.
italian.CreateStartupIcon=Crea un collegamento all'avvio
spanish.FinishedLabel=Tommy Memory Cleaner ha sido instalado en tu computadora.%n%nCreado por tommy437%n%nHaz clic en Finalizar para cerrar este asistente.
spanish.CreateStartupIcon=Crear un acceso directo al inicio
french.FinishedLabel=Tommy Memory Cleaner a été installé sur votre ordinateur.%n%nCréé par tommy437%n%nCliquez sur Terminer pour fermer cet assistant.
french.CreateStartupIcon=Créer un raccourci au démarrage
german.FinishedLabel=Tommy Memory Cleaner wurde auf Ihrem Computer installiert.%n%nErstellt von tommy437%n%nKlicken Sie auf Fertigstellen, um diesen Assistenten zu schließen.
german.CreateStartupIcon=Erstellen Sie eine Startverknüpfung
portuguese.FinishedLabel=Tommy Memory Cleaner foi instalado no seu computador.%n%nCriado por tommy437%n%nClique em Concluir para fechar este assistente.
portuguese.CreateStartupIcon=Criar um atalho de inicialização

[Files]
; Il file eseguibile - il .bat lo prepara nella directory corretta
Source: "target\release\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion
; File DLL e risorse
Source: "target\release\*.dll"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "target\release\resources\*"; DestDir: "{app}\resources"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist
; Fallback per _up_ se necessario
Source: "target\release\_up_\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "target\release\_up_\resources\*"; DestDir: "{app}\resources"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\{#AppExeName}"
Name: "{group}\{cm:UninstallProgram,{#AppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon; IconFilename: "{app}\{#AppExeName}"
Name: "{userstartup}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: startupicon; IconFilename: "{app}\{#AppExeName}"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startupicon"; Description: "{cm:CreateStartupIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "runonstartup"; Description: "Run Tommy Memory Cleaner when Windows starts"; GroupDescription: "Startup Options"; Flags: unchecked

[Run]
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
Filename: "{app}\{#AppExeName}"; Description: "Configure startup"; Flags: nowait postinstall skipifsilent; Tasks: runonstartup; Parameters: "--startup-config"

[UninstallDelete]
Type: filesandordirs; Name: "{app}\resources"

[Code]
var
  WelcomePage: TWizardPage;
  ConfigPage: TWizardPage;
  WelcomeMemo: TNewMemo;
  WelcomeTitle: TLabel;
  WelcomeSubtitle: TLabel;
  ConfigTitle: TLabel;
  ConfigSubtitle: TLabel;
  AlwaysOnTopCheck: TNewCheckBox;
  NotificationsCheck: TNewCheckBox;
  ThemeRadioLight: TNewRadioButton;
  ThemeRadioDark: TNewRadioButton;
  ThemeGroup: TNewStaticText;

procedure InitializeWizard;
var
  TopMargin: Integer;
  LeftMargin: Integer;
begin
  TopMargin := 10;
  LeftMargin := 0;
  
  // Stile moderno per la wizard form
  WizardForm.Color := $F5F5F5;
  WizardForm.Font.Name := 'Segoe UI';
  WizardForm.Font.Size := 9;
  
  // ========== PAGINA WELCOME ==========
  WelcomePage := CreateCustomPage(wpWelcome, 'Welcome', '');
  
  // Titolo principale - Più compatto
  WelcomeTitle := TLabel.Create(WelcomePage);
  WelcomeTitle.Parent := WelcomePage.Surface;
  WelcomeTitle.Caption := '🧠 Tommy Memory Cleaner';
  WelcomeTitle.Left := LeftMargin;
  WelcomeTitle.Top := TopMargin;
  WelcomeTitle.Width := 520;
  WelcomeTitle.Height := 30;
  WelcomeTitle.Font.Name := 'Segoe UI';
  WelcomeTitle.Font.Size := 20;
  WelcomeTitle.Font.Style := [fsBold];
  WelcomeTitle.Font.Color := $00333333; // Grigio scuro normale (#333333)
  WelcomeTitle.Transparent := False;
  WelcomeTitle.Color := $F5F5F5;
  
  // Sottotitolo
  WelcomeSubtitle := TLabel.Create(WelcomePage);
  WelcomeSubtitle.Parent := WelcomePage.Surface;
  WelcomeSubtitle.Caption := 'Advanced Memory Optimization Tool';
  WelcomeSubtitle.Left := LeftMargin;
  WelcomeSubtitle.Top := TopMargin + 32;
  WelcomeSubtitle.Width := 520;
  WelcomeSubtitle.Height := 18;
  WelcomeSubtitle.Font.Name := 'Segoe UI';
  WelcomeSubtitle.Font.Size := 10;
  WelcomeSubtitle.Font.Color := $009A8A72;
  WelcomeSubtitle.Transparent := True;
  
  // Descrizione più compatta e scorrevole
  WelcomeMemo := TNewMemo.Create(WelcomePage);
  WelcomeMemo.Parent := WelcomePage.Surface;
  WelcomeMemo.Left := LeftMargin;
  WelcomeMemo.Top := TopMargin + 55;
  WelcomeMemo.Width := 520;
  WelcomeMemo.Height := 180;
  WelcomeMemo.ReadOnly := True;
  WelcomeMemo.ScrollBars := ssVertical;
  WelcomeMemo.BorderStyle := bsSingle;
  WelcomeMemo.Color := $FFFFFF;
  WelcomeMemo.Font.Name := 'Segoe UI';
  WelcomeMemo.Font.Size := 9;
  WelcomeMemo.Font.Color := $333333;
  WelcomeMemo.Text := 
    'Professional memory optimization tool for Windows' + #13#10 + #13#10 +
    'Key features:' + #13#10 +
    '  ✓ Free up system memory automatically' + #13#10 +
    '  ✓ Improve overall system performance' + #13#10 +
    '  ✓ Optimize memory usage in real-time' + #13#10 +
    '  ✓ Customize optimization profiles' + #13#10 +
    '  ✓ Monitor memory from system tray' + #13#10 + #13#10 +
    'Created with ❤️ by tommy437';
  
  // ========== PAGINA CONFIGURAZIONE ==========
  ConfigPage := CreateCustomPage(WelcomePage.ID, 'Initial Configuration', '');
  
  // Rimuoviamo la pagina Tutorial per semplificare
  
  // Titolo pagina configurazione - più compatto
  ConfigTitle := TLabel.Create(ConfigPage);
  ConfigTitle.Parent := ConfigPage.Surface;
  ConfigTitle.Caption := '⚙️ Initial Settings';
  ConfigTitle.Left := LeftMargin;
  ConfigTitle.Top := TopMargin;
  ConfigTitle.Width := 520;
  ConfigTitle.Height := 25;
  ConfigTitle.Font.Name := 'Segoe UI';
  ConfigTitle.Font.Size := 16;
  ConfigTitle.Font.Style := [fsBold];
  ConfigTitle.Font.Color := $00333333; // Grigio scuro normale (#333333)
  ConfigTitle.Transparent := False;
  ConfigTitle.Color := $F5F5F5;
  
  // Sottotitolo
  ConfigSubtitle := TLabel.Create(ConfigPage);
  ConfigSubtitle.Parent := ConfigPage.Surface;
  ConfigSubtitle.Caption := 'Configure your initial preferences (you can change them later)';
  ConfigSubtitle.Left := LeftMargin;
  ConfigSubtitle.Top := TopMargin + 28;
  ConfigSubtitle.Width := 520;
  ConfigSubtitle.Height := 18;
  ConfigSubtitle.Font.Name := 'Segoe UI';
  ConfigSubtitle.Font.Size := 9;
  ConfigSubtitle.Font.Color := $666666;
  ConfigSubtitle.Transparent := True;
  
  // Always on top checkbox - più compatto
  AlwaysOnTopCheck := TNewCheckBox.Create(ConfigPage);
  AlwaysOnTopCheck.Parent := ConfigPage.Surface;
  AlwaysOnTopCheck.Left := LeftMargin + 10;
  AlwaysOnTopCheck.Top := TopMargin + 55;
  AlwaysOnTopCheck.Width := 500;
  AlwaysOnTopCheck.Height := 20;
  AlwaysOnTopCheck.Caption := '  Keep window always on top';
  AlwaysOnTopCheck.Checked := False;
  AlwaysOnTopCheck.Font.Name := 'Segoe UI';
  AlwaysOnTopCheck.Font.Size := 9;
  
  // Notifiche checkbox - più compatto
  NotificationsCheck := TNewCheckBox.Create(ConfigPage);
  NotificationsCheck.Parent := ConfigPage.Surface;
  NotificationsCheck.Left := LeftMargin + 10;
  NotificationsCheck.Top := TopMargin + 80;
  NotificationsCheck.Width := 500;
  NotificationsCheck.Height := 20;
  NotificationsCheck.Caption := '  Enable optimization notifications';
  NotificationsCheck.Checked := True;
  NotificationsCheck.Font.Name := 'Segoe UI';
  NotificationsCheck.Font.Size := 9;
  
  // Gruppo tema - più compatto
  ThemeGroup := TNewStaticText.Create(ConfigPage);
  ThemeGroup.Parent := ConfigPage.Surface;
  ThemeGroup.Left := LeftMargin + 10;
  ThemeGroup.Top := TopMargin + 110;
  ThemeGroup.Width := 500;
  ThemeGroup.Height := 18;
  ThemeGroup.Caption := 'Theme:';
  ThemeGroup.Font.Name := 'Segoe UI';
  ThemeGroup.Font.Size := 10;
  ThemeGroup.Font.Style := [fsBold];
  
  // Radio button Light - più compatto
  ThemeRadioLight := TNewRadioButton.Create(ConfigPage);
  ThemeRadioLight.Parent := ConfigPage.Surface;
  ThemeRadioLight.Left := LeftMargin + 25;
  ThemeRadioLight.Top := TopMargin + 135;
  ThemeRadioLight.Width := 200;
  ThemeRadioLight.Height := 20;
  ThemeRadioLight.Caption := '  Light Theme';
  ThemeRadioLight.Checked := False;
  ThemeRadioLight.Font.Name := 'Segoe UI';
  ThemeRadioLight.Font.Size := 9;
  
  // Radio button Dark - più compatto
  ThemeRadioDark := TNewRadioButton.Create(ConfigPage);
  ThemeRadioDark.Parent := ConfigPage.Surface;
  ThemeRadioDark.Left := LeftMargin + 25;
  ThemeRadioDark.Top := TopMargin + 160;
  ThemeRadioDark.Width := 250;
  ThemeRadioDark.Height := 20;
  ThemeRadioDark.Caption := '  Dark Theme (Recommended)';
  ThemeRadioDark.Checked := True;
  ThemeRadioDark.Font.Name := 'Segoe UI';
  ThemeRadioDark.Font.Size := 9;
end;

function InitializeSetup(): Boolean;
var
  LangCode: String;
  ConfigPath: String;
  ConfigFile: String;
  ConfigJson: String;
  ThemeValue: String;
begin
  Result := True;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  LangCode: String;
  ConfigPath: String;
  ConfigFile: String;
  ConfigJson: String;
  ThemeValue: String;
begin
  // Salva le impostazioni solo quando l'installazione è completata
  if CurStep = ssPostInstall then
  begin
    // Converti lingua
    LangCode := ActiveLanguage;
    if LangCode = 'italian' then LangCode := 'it'
    else if LangCode = 'spanish' then LangCode := 'es'
    else if LangCode = 'french' then LangCode := 'fr'
    else if LangCode = 'german' then LangCode := 'de'
    else if LangCode = 'portuguese' then LangCode := 'pt'
    else LangCode := 'en';
    
    // Determina tema
    if ThemeRadioLight.Checked then
      ThemeValue := 'light'
    else
      ThemeValue := 'dark';
    
    // Path configurazione
    ConfigPath := ExpandConstant('{userappdata}\TommyMemoryCleaner');
    ConfigFile := ConfigPath + '\config.json';
    
    // Crea directory se non esiste
    if not DirExists(ConfigPath) then
      CreateDir(ConfigPath);
    
    // Costruisci JSON con tutte le impostazioni
    // Converti booleani in stringhe JSON
    if AlwaysOnTopCheck.Checked then
      ConfigJson := '"always_on_top":true,'
    else
      ConfigJson := '"always_on_top":false,';
    
    if NotificationsCheck.Checked then
      ConfigJson := ConfigJson + '"show_opt_notifications":true'
    else
      ConfigJson := ConfigJson + '"show_opt_notifications":false';
    
    // Costruisci JSON completo
    ConfigJson := '{' +
      '"language":"' + LangCode + '",' +
      '"theme":"' + ThemeValue + '",' +
      ConfigJson +
      '}';
    
    // Salva configurazione
    SaveStringToFile(ConfigFile, ConfigJson, False);
  end;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  // Applica stile moderno e colori eleganti alle pagine
  WizardForm.Color := $F5F5F5;
  WizardForm.InnerPage.Color := $F5F5F5;
  
  // Migliora i pulsanti con colori moderni
  WizardForm.NextButton.Font.Name := 'Segoe UI';
  WizardForm.NextButton.Font.Size := 9;
  WizardForm.NextButton.Font.Style := [fsBold];
  
  WizardForm.BackButton.Font.Name := 'Segoe UI';
  WizardForm.BackButton.Font.Size := 9;
  
  WizardForm.CancelButton.Font.Name := 'Segoe UI';
  WizardForm.CancelButton.Font.Size := 9;
  
  // Applica stile specifico per ogni pagina
  if CurPageID = WelcomePage.ID then
  begin
    WizardForm.InnerPage.Color := $F5F5F5;
  end
  else if CurPageID = ConfigPage.ID then
  begin
    WizardForm.InnerPage.Color := $F5F5F5;
  end;
end;
