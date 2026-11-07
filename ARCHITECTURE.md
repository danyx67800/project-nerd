# Architettura

Project Nerd è un'app macOS nativa scritta interamente in Rust. Non usa webview, Electron o Tauri: il rendering è GPU tramite `wgpu`, l'audio passa da CoreAudio con `cpal`, il video da AVFoundation tramite `objc2`, i menu sono `NSMenu` nativi.

```
                 ┌──────────────────────────────────────────────┐
                 │                 nerd-app                     │
                 │  shell egui · pagine · menu · OTA · output    │
                 └───────┬───────────────┬──────────────┬───────┘
                         │               │              │
              ┌──────────▼──────┐ ┌──────▼───────┐ ┌────▼─────────┐
              │   nerd-audio    │ │  nerd-video  │ │  nerd-core   │
              │ cpal/symphonia  │ │ AVFoundation │ │ modello +    │
              │ rubato          │ │ wgpu frames  │ │ state machine│
              └─────────────────┘ └──────────────┘ └──────────────┘
```

## Crate

| Crate | Responsabilità | Dipendenze macOS |
|---|---|---|
| `nerd-core` | modello serata, persistenza atomica, recovery, relink, state machine, libreria, waveform | nessuna |
| `nerd-audio` | decodifica, resampling, mix a campione, fade/crossfade, VU, dispositivi | CoreAudio via `cpal` |
| `nerd-video` | probe e thumbnail AVFoundation, frame RGBA, elenco display | AVFoundation/CoreMedia |
| `nerd-app` | shell egui, pagine, menu nativi, uscita su secondo schermo, OTA, packaging runtime | AppKit/winit |
| `nerd-tools` | generazione media sintetici, chiavi OTA, helper di build | nessuna |

La regola di dipendenza è unidirezionale: `nerd-core` non conosce né audio né UI. Le altre crate possono dipendere dal core, mai il contrario. Questo permette di eseguire l'intera suite dello show su qualunque macchina (e in CI) senza hardware audio.

## Toolkit UI: perché egui

Candidati valutati con un prototipo di pannelli ridimensionabili + timeline:

| Criterio | egui 0.37 + eframe | Slint + stile Cupertino | GPUI |
|---|---|---|---|
| Pannelli/timeline custom | Pittura immediata, ottima per clip e playhead | Layout dichiarativo, timeline custom faticosa | Buona, ma ecosistema giovane |
| Multi-viewport (proiettore) | Supporto nativo `show_viewport_deferred` con monitor | Finestre secondarie acerbe | Solo macOS, API instabile |
| Menu nativi | `muda` + `NSMenu` | supporto limitato | nativo |
| Font/theme runtime | Caricamento SF a runtime, tema completo | stile Cupertino preconfezionato | nativo |
| Integrazione wgpu/Metal | backend wgpu ufficiale | richiede integrazione custom | Metal diretto |
| Maturità / rischio alpha | alta | media | bassa |
| 100% Rust, no webview | sì | sì | sì |

**Decisione: egui/eframe.** La UI da video editor è fatta di pannelli ridimensionabili e di una timeline disegnata a mano; la modalità immediata di egui rende questi widget diretti e prevedibili, senza dover combattere un layout engine dichiarativo. `eframe` fornisce inoltre viewport multiple sulla stessa `wgpu` texture: anteprima e uscita proiettore condividono lo stesso renderer e quindi sono identiche per costruzione. Slint avrebbe dato un aspetto Cupertino più pronto ma una timeline e un'uscita multi-schermo molto più rigide; GPUI è troppo giovane per un'app da palco.

Non uso `egui_dock`: la sua versione corrente è legata a una minor di egui diversa da quella di eframe. I pannelli ridimensionabili richiesti sono ottenuti con `SidePanel`/`TopBottomPanel` e una barra pagine disegnata in basso, evitando una dipendenza fragile senza perdere funzionalità.

## State machine dello show

`nerd-core::show` non possiede un orologio e non fa I/O. Riceve `tick(dt_ms)` e restituisce `Effect`:

```
GO ──► Playing ──fine brano──► Waiting (sottofondo + countdown)
         │  ▲                         │
     Pausa│  │Riprendi          GO / countdown
         ▼  │                         ▼
       Paused                     Playing(next)
Stop/Panic ──► Stopping/Panic ──► Idle
```

Punti fermi:

- `Manual`: ogni brano richiede GO; a fine brano parte il sottofondo.
- `Automatic`: dopo il brano parte il sottofondo per `gap` (globale o per transizione), countdown visibile, poi il brano successivo. La sospensione congela il countdown.
- GO durante un brano → `GoOutcome::ConfirmRequired`; la conferma è un salto immediato con crossfade.
- Stop con fade interrompe brano e sottofondo e marca il brano interrotto come `Skipped`; GO riparte dal successivo.
- Panic usa un fade breve configurabile e lascia la scaletta pronta per ripartire.
- Fine scaletta: il sottofondo continua finché non si preme Stop; GO riparte dall'inizio (bis).
- Indietro seleziona il brano precedente e attende sempre un GO, anche in automatico.

La macchina mantiene gli stati dei brani (in attesa / in onda / fatto / saltato) e li sincronizza nella serata a ogni transizione; gli stati non entrano nella cronologia undo.

## Motore audio

```
UI thread ──prepare(path,trim)──► decoder worker ──Arc<DecodedAudio>──► cache LRU
   │                                                                      │
   └──Effect::PlaySong──► ArrayQueue<Command> ──► callback cpal ◄─────────┘
                                                     │
                                          song A/B + background bus
                                                     │
                                             mix → device buffer
```

- Il callback CoreAudio non alloca e non prende lock: legge comandi da una `ArrayQueue` lock-free, mixa buffer già decodificati e restituisce gli `Arc` ritirati a una coda di garbage letta dal thread di controllo.
- Il brano successivo viene decodificato in anticipo; due voci brano permettono il crossfade di GO/skip senza click.
- Bus separati Brani e Sottofondo, con envelope per campione (fade in/out, ducking del sottofondo durante il brano).
- Il sottofondo è una playlist concatenata in un unico buffer (loop/shuffle decisi dal thread di controllo) per garantire gapless.
- VU meter: picchi calcolati nel callback e pubblicati come `AtomicU32` (bit di `f32`).
- Disconnessione dispositivo: l'error callback segnala il guasto; il thread di controllo ritenta il default device e la UI mostra un avviso, senza popup durante lo show.
- Le API sono verificate sulle versioni correnti: `cpal 0.18`, `symphonia 0.6`, `rubato 5`.

## Video e uscita

- Probe (durata, dimensioni, thumbnail) con AVFoundation; sul ramo non macOS la crate degrada con un errore esplicito.
- I frame video sono decodificati in RGBA e caricati in una texture egui/wgpu. Il renderer di uscita disegna, nello stesso ordine: frame video, overlay titoli, schermata di cortesia/blackout. L'anteprima nella pagina Live e la finestra proiettore chiamano **la stessa funzione** su viewport diverse.
- L'uscita è una viewport egui borderless a schermo intero sul monitor scelto (`ViewportBuilder::with_monitor`). L'elenco display viene riletto periodicamente per l'hot-plug; se il display sparisce si torna al principale con un avviso.
- Titoli: dimensioni relative all'altezza dell'output, quindi preview e proiettore sono identici a qualsiasi risoluzione. Animazioni di entrata/uscita, ritardo e durata sono parametri del template.
- Audio dei video: il video è muto a livello di layer; l'audio dello stesso file viene decodificato da symphonia e riprodotto dal motore, così c'è un solo clock audio e non può esserci deriva A/V.

## Persistenza

- Serata: JSON con `schema_version`, estensione `.nerdshow`.
- Scrittura atomica: file temporaneo nella stessa cartella, `fsync`, `rename`.
- Autosave su file dell'utente; snapshot di recovery in `~/Library/Application Support/Project Nerd/recovery.nerdshow`.
- All'avvio, se esiste una recovery più recente e non ancora archiviata, viene proposta all'operatore.
- File spostati: la scansione di una cartella ricollega per nome file (case-insensitive, profondità 8) audio, video e sottofondo.
- Libreria e preferenze sono file separati nella stessa cartella. Nessuna telemetria.

## OTA

- GitHub Releases con tag SemVer (`vX.Y.Z`, pre-release `-alpha.N`).
- Asset: zip del `.app` universale, file `.sha256`, firma Ed25519 del contenuto del file checksum.
- La chiave pubblica è compilata nell'app; la privata vive solo nei secret di GitHub Actions.
- Download con avanzamento, verifica checksum + firma, estrazione con `ditto`, sostituzione dell'intero bundle tramite helper che attende la chiusura del processo, riavvio e rollback.
- Mai durante uno show attivo: solo badge; installazione su conferma.
- Canali Stabile/Alpha, gestione 403/rate limit e assenza di rete.
- Senza permessi di scrittura sul bundle: download nella cartella Download, verifica e apertura del `.dmg`.

## Affidabilità

- `panic hook` che salva la recovery prima di terminare.
- Errori di file mancanti/corrotti e dispositivi scollegati sono stati espliciti nel modello e nella UI, mai `unwrap` nel percorso audio.
- Durante lo show: `caffeinate` come figlio dell'app impedisce sleep del display e del sistema; attività `NSProcessInfo` per limitare App Nap.
- Test: 37 test del core, inclusa simulazione di 2 ore; soak reale documentato in `README.md`.

## Limiti dichiarati dell'alpha

- Codec video: solo formati/container nativi macOS (H.264, HEVC, ProRes in MP4/MOV/M4V).
- Un solo progetto aperto alla volta; niente sync cloud.
- L'updater richiede un repository GitHub configurato e una release firmata.
- La localizzazione è italiana con stringhe centralizzate in `i18n.rs`, predisposte per altre lingue.
