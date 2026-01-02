# Tommy Memory Cleaner - Guida CLI

## Overview

Tommy Memory Cleaner (TMC) può essere utilizzato da linea di comando per ottimizzare la memoria del sistema senza avviare l'interfaccia grafica. Questa modalità è utile per script, automazioni o sistemi senza GUI.

## Requisiti

- Windows 10/11 (x64)
- Eseguibile `tmc.exe` compilato
- Privilegi di amministratore (richiesti per l'ottimizzazione della memoria)

## Sintassi di Base

```cmd
tmc.exe [OPZIONI]
```

## Opzioni Disponibili

### Aree di Memoria

| Opzione | Descrizione |
|---------|-------------|
| `/WorkingSet` | Ottimizza il Working Set dei processi |
| `/ModifiedPageList` | Ottimizza la Modified Page List |
| `/StandbyList` | Ottimizza la Standby List |
| `/StandbyListLow` | Ottimizza la Standby List a bassa priorità |
| `/SystemFileCache` | Ottimizza la cache dei file di sistema |
| `/CombinedPageList` | Ottimizza la Combined Page List |
| `/ModifiedFileCache` | Ottimizza la Modified File Cache |
| `/RegistryCache` | Ottimizza la cache del Registro di sistema |

### Profili Predefiniti

| Opzione | Descrizione | Aree Incluse |
|---------|-------------|--------------|
| `/Profile:Normal` | Profilo normale (conservativo) | Working Set, System File Cache |
| `/Profile:Balanced` | Profilo bilanciato (default) | Working Set, Standby List, System File Cache |
| `/Profile:Gaming` | Profilo gaming (aggressivo) | Tutte le aree eccetto Registry Cache |

### Utilità

| Opzione | Descrizione |
|---------|-------------|
| `/?`, `/help`, `-h`, `--help` | Mostra la guida rapida |

## Esempi di Utilizzo

### 1. Ottimizzazione Base (Default)
```cmd
tmc.exe
```
Utilizza automaticamente il profilo Balanced.

### 2. Ottimizzazione di Aree Specifiche
```cmd
tmc.exe /WorkingSet /StandbyList
```
Ottimizza solo il Working Set e la Standby List.

### 3. Utilizzo dei Profili
```cmd
tmc.exe /Profile:Gaming
```
Applica il profilo Gaming per ottimizzare tutte le aree principali.

### 4. Ottimizzazione Completa
```cmd
tmc.exe /WorkingSet /ModifiedPageList /StandbyList /StandbyListLow /SystemFileCache /CombinedPageList /ModifiedFileCache /RegistryCache
```
Ottimizza tutte le aree disponibili.

### 5. Visualizzazione Aiuto
```cmd
tmc.exe /?
```

## Output del Programma

### Esecuzione Riuscita
```
Using profile: Balanced
Optimizing memory areas: ["Working Set", "Standby List", "System File Cache"]
Optimization completed successfully
Freed: 512.34 MB
  Working Set: OK
  Standby List: OK
  System File Cache: OK
```

### Errori Comuni
```
Unknown argument: /InvalidArea
Use /? for help
```

```
Optimization failed: Access denied. Run as Administrator.
```

## Script PowerShell Esempio

### Script di Ottimizzazione Programmata
```powershell
# optimize-memory.ps1
# Esegui come amministratore

Write-Host "Tommy Memory Cleaner - Ottimizzazione Programmata" -ForegroundColor Green

# Ottimizza con profilo Balanced
& ".\tmc.exe" /Profile:Balanced

if ($LASTEXITCODE -eq 0) {
    Write-Host "Ottimizzazione completata con successo!" -ForegroundColor Green
} else {
    Write-Host "Errore durante l'ottimizzazione!" -ForegroundColor Red
    exit 1
}
```

### Script per Gaming
```powershell
# gaming-optimize.ps1
# Ottimizzazione prima di sessioni gaming intensive

Write-Host "Modalità Gaming - Ottimizzazione Memoria" -ForegroundColor Yellow

# Chiudi processi non essenziali (esempio)
Stop-Process -Name "chrome" -Force -ErrorAction SilentlyContinue
Stop-Process -Name "discord" -Force -ErrorAction SilentlyContinue

# Ottimizzazione aggressiva
& ".\tmc.exe" /Profile:Gaming

Write-Host "Sistema ottimizzato per gaming!" -ForegroundColor Green
```

## Task Scheduler

### Creare un Task Programmato

1. Apri **Task Scheduler** di Windows
2. Crea un nuovo Task
3. Imposta:
   - **Trigger**: Giornaliero alle 02:00 AM
   - **Action**: Avvia il programma
   - **Program/Script**: Percorso completo di `tmc.exe`
   - **Arguments**: `/Profile:Balanced`
   - **Conditions**: Spunta "Run with highest privileges"

## Tips & Best Practices

### ✅ Best Practices
- Esegui sempre come amministratore
- Usa il profilo Balanced per uso quotidiano
- Usa il profilo Gaming solo prima di sessioni intensive
- Combina l'ottimizzazione con la chiusura di programmi non necessari

### ⚠️ Avvertenze
- Non eseguire troppe ottimizzazioni ravvicinate (aspetta almeno 5 minuti)
- L'ottimizzazione frequente della Modified Page List può influire sulle performance
- Chiudi applicazioni importanti prima di ottimizzazioni aggressive

### 🔧 Troubleshooting

| Problema | Soluzione |
|----------|-----------|
| "Access denied" | Esegui come amministratore |
| "Unknown argument" | Verifica la sintassi con `tmc.exe /?` |
| Nessuna memoria liberata | Prova un profilo più aggressivo o riavvia il sistema |

## Integrazione con Altri Tool

### Batch File
```batch
@echo off
echo Ottimizzazione memoria in corso...
tmc.exe /Profile:Balanced
if %errorlevel% equ 0 (
    echo Successo!
) else (
    echo Errore durante l'ottimizzazione
)
pause
```

### Integration con Node.js
```javascript
const { execFile } = require('child_process');

function optimizeMemory(profile = 'Balanced') {
    return new Promise((resolve, reject) => {
        execFile('tmc.exe', [`/Profile:${profile}`], (error, stdout) => {
            if (error) reject(error);
            else resolve(stdout);
        });
    });
}

// Utilizzo
optimizeMemory('Gaming')
    .then(output => console.log(output))
    .catch(err => console.error(err));
```

## Riferimento Rapido

| Comando | Uso |
|---------|-----|
| `tmc.exe` | Ottimizzazione base (Balanced) |
| `tmc.exe /?` | Mostra aiuto |
| `tmc.exe /Profile:Normal` | Ottimizzazione conservativa |
| `tmc.exe /Profile:Gaming` | Ottimizzazione aggressiva |
| `tmc.exe /WorkingSet` | Ottimizza solo Working Set |

---

**Nota**: L'uso eccessivo dell'ottimizzazione della memoria può ridurre le performance del sistema. Utilizza con moderazione e quando necessario.
