# Decisioni

Le ambiguità sono state risolte con il default più sicuro per un operatore sotto stress. Ogni voce è vincolante per l'implementazione.

| ID | Decisione | Motivazione |
|---|---|---|
| D-001 | Toolkit UI: **egui/eframe 0.37** con backend wgpu. | Pannelli e timeline custom, multi-viewport per il proiettore, font SF a runtime, maturità. Vedi `ARCHITECTURE.md`. |
| D-002 | Layout a pannelli con `SidePanel`/`TopBottomPanel`, **senza** `egui_dock`. | `egui_dock 0.21` dipende da una minor diversa di egui; i pannelli ridimensionabili richiesti sono nativi in egui. |
| D-003 | La state machine non possiede un orologio: riceve `tick(dt_ms)`. | Test deterministici, nessuna dipendenza da `Instant` o dal sistema. |
| D-004 | GO durante un brano: conferma, poi **salto immediato con crossfade** al brano successivo (non una pausa). | L'operatore che preme GO crede che il brano sia finito: vuole il successivo subito. |
| D-005 | Stop con fade marca il brano interrotto come **Saltato**, ferma il sottofondo e riporta a Idle; GO riparte dal prossimo in attesa. | Stop = fine dell'esecuzione corrente; evita di riprodurre due volte lo stesso brano per errore. |
| D-006 | Indietro seleziona il brano precedente e **attende sempre GO**, anche in automatico. | È un'azione di recupero: l'operatore deve riprendere il controllo. |
| D-007 | Il sottofondo parte a brano terminato; non c'è overlap sul finale del brano. | Il fade-out del brano è già gestito dal motore; un crossfade brano/sottofondo confonderebbe il finale musicale. |
| D-008 | `duration_ms = 0` nel template titoli significa "visibile per tutto il brano". | Default utile per i testi lunghi; il campo resta configurabile. |
| D-009 | La firma OTA copre il contenuto del file `.sha256` (stringa esadecimale + newline). | Firma piccola, verifica in due passi indipendenti: checksum del pacchetto e autenticità del checksum. |
| D-010 | Il font SF viene letto a runtime da `/System/Library/Fonts/SFNS.ttf`; se assente si usa il font di sistema di egui. | Licenza Apple: nessun font nel repository. |
| D-011 | Lo schermo di uscita è scelto per **indice monitor** (0-based) e riflette l'ordine di macOS. | `ViewportBuilder::with_monitor` di egui 0.37 usa lo stesso ordine; nessuna API privata. |
| D-012 | Gli stati dei brani sono persistiti ma vengono azzerati quando si apre una serata per una nuova esecuzione; la recovery li conserva. | Servono al riavvio dopo un crash, non sono parte della scaletta. |
| D-013 | I media di test sono generati dal repository (toni/chord e pattern video con AVAssetWriter). | Nessun materiale protetto da copyright. |
| D-014 | Versioni crate verificate sulla documentazione corrente: cpal 0.18, symphonia 0.6, rubato 5, egui/eframe 0.37, ed25519-dalek 3. | Evitare API inventate; il workspace dichiara `rust-version = 1.88`. |
| D-015 | Il tema è scuro per default, con chiaro/seguimi sistema; i materiali traslucidi usano superfici stratificate egui. La vibrancy AppKit nativa è applicata al chrome quando disponibile, non è un requisito bloccante. | Liquid Glass non ha un'API pubblica stabile per una superficie wgpu; l'aspetto resta HIG (gerarchia, spaziatura, materiali). |
| D-016 | Nessuna telemetria, nessuna chiamata di rete oltre all'OTA e alla verifica manuale. | Requisito di privacy e affidabilità offline. |
| D-017 | L'audio dei video è decodificato dal file video con symphonia e mixato dal motore; il layer video è muto. | Un solo clock audio, nessuna deriva A/V, rispetto del dispositivo di uscita scelto. |
| D-018 | L'ora di inizio del brano viene conteggiata dalla UI (tick ~60 Hz), non dal callback audio. | Semplice e sufficiente per countdown e playhead; il motore ha il proprio clock campione per i fade. |
| D-019 | La selezione del dispositivo audio usa il nome CoreAudio; se non trovato si ricade sul default con avviso. | I device ID cambiano tra riavvii; il nome è stabile nella pratica. |
| D-020 | Un solo progetto aperto alla volta, senza finestre multiple di editing. | Riduce lo stato e i rischi in show. |

## Convenzioni

- Codice e commenti in inglese, UI e documentazione in italiano.
- `unsafe` solo nei confini FFI (AVFoundation/corevideo, AppKit, SF Symbols), in moduli dedicati e commentati.
- Commit Conventional Commits, uno per milestone.
- Nessun `unwrap()` su input dell'utente, I/O o callback audio.
