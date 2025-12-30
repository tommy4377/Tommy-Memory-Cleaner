# Script PowerShell per verificare la registrazione delle notifiche
Write-Host "=== Controllo Registro Windows Toast ===" -ForegroundColor Cyan

$appId = "TommyMemoryCleaner"
$regPath = "HKCU:\Software\Classes\AppUserModelId\$appId"

Write-Host "`nPercorso registro: $regPath" -ForegroundColor Yellow

if (Test-Path $regPath) {
    Write-Host "✓ Chiave registro trovata" -ForegroundColor Green
    
    # Leggi DisplayName
    $displayName = (Get-ItemProperty -Path $regPath -Name DisplayName -ErrorAction SilentlyContinue).DisplayName
    if ($displayName) {
        Write-Host "✓ DisplayName trovato: '$displayName'" -ForegroundColor Green
    } else {
        Write-Host "✗ DisplayName NON trovato!" -ForegroundColor Red
    }
    
    # Leggi IconUri
    $iconUri = (Get-ItemProperty -Path $regPath -Name IconUri -ErrorAction SilentlyContinue).IconUri
    if ($iconUri) {
        Write-Host "✓ IconUri trovato: '$iconUri'" -ForegroundColor Green
    } else {
        Write-Host "⚠ IconUri non trovato" -ForegroundColor Yellow
    }
    
    # Mostra tutti i valori
    Write-Host "`n--- Tutti i valori nella chiave ---" -ForegroundColor Cyan
    Get-ItemProperty -Path $regPath | Format-List
    
} else {
    Write-Host "✗ Chiave registro NON trovata!" -ForegroundColor Red
    Write-Host "La registrazione non è stata completata." -ForegroundColor Yellow
}

Write-Host "`n=== Controllo processi TMC in esecuzione ===" -ForegroundColor Cyan

$tmcProcesses = Get-Process -Name "TMC" -ErrorAction SilentlyContinue
if ($tmcProcesses) {
    Write-Host "✓ Processi TMC trovati: $($tmcProcesses.Count)" -ForegroundColor Green
    foreach ($proc in $tmcProcesses) {
        Write-Host "  - PID: $($proc.Id), Path: $($proc.Path)" -ForegroundColor Gray
    }
} else {
    Write-Host "⚠ Nessun processo TMC in esecuzione" -ForegroundColor Yellow
    Write-Host "  (Avvia l'app per vedere informazioni complete)" -ForegroundColor Gray
}

Write-Host "`n=== Informazioni sulle notifiche ===" -ForegroundColor Cyan
Write-Host "Se il DisplayName è impostato ma le notifiche mostrano ancora 'com.tommymemorycleaner.app',"
Write-Host "potrebbe essere necessario:"
Write-Host "1. Riavviare completamente Windows (non solo l'app)"
Write-Host "2. Eseguire clean-app-data.bat come amministratore"
Write-Host "3. Verificare che l'app sia stata compilata con le ultime modifiche"

