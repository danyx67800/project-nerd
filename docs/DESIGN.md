# Project Nerd — Design system

Il livello grafico di Project Nerd è un'app macOS nativa costruita con egui, ma con regole
visive proprie. La logica resta nei crate `nerd-core`, `nerd-audio` e `nerd-video`; questo
documento descrive solo il linguaggio visivo e il crate `nerd-ui-kit`.

## Principi

1. **Profondità per luminosità, non per bordi.** I pannelli si distinguono per superfici
   sovrapposte e ombre morbide. I separatori sono linee da 1 px (bianco 8–10%).
2. **Un solo accento.** Selezione, focus e azioni primarie usano
   `NSColor.controlAccentColor`, letto a runtime. Verde, menta, ambra e rosso sono riservati
   agli stati operativi.
3. **La scena prima dell'interfaccia.** Nelle pagine Live i titoli e i countdown dominano;
   nella timeline il contenuto ha più peso dei controlli.
4. **Nessun form web.** Menu, campi, toggle, stepper e selettori sono componenti del kit,
   non widget predefiniti di egui.

## Token

Il crate `crates/nerd-ui-kit` è l'unica fonte di colori, tipografia, raggi, spaziature e
durate. Le pagine non contengono colori o font hard-coded.

### Superfici

| Token | Scuro indicativo | Uso |
| --- | --- | --- |
| `well` | `#121214` | viewer, timeline, aree di contenuto profondo |
| `window` | `#1E1E20` | sfondo finestra e pagina |
| `panel` | `#242427` | pannelli, sidebar, card dei gruppi |
| `card` | `#2A2A2D` | campi, card sollevate |
| `hover` | `#343438` | hover e righe evidenziate |
| `popover` | `#303034` | menu e popover |

Il tema chiaro è derivato dalla stessa scala, con superfici `#F3F3F4`, `#FFFFFF`,
`#ECECEE`, `#E2E2E5` e testo quasi nero. I colori di sistema vengono da AppKit; il fallback
scuro è usato solo quando non è disponibile un contesto nativo.

### Tipografia

SF Pro di sistema, con fallback alla font di sistema egui. Scala minima 11 pt.

| Ruolo | Punti | Peso |
| --- | --- | --- |
| Titolo pagina | 22 | semibold |
| Intestazione sezione | 15 | semibold |
| Corpo | 13 | regular |
| Secondario | 12 | regular |
| Micro / etichette | 11 | medium |
| Titolo brano Live | ≥ 44 | semibold |
| Countdown Live | ≥ 96 | semibold, cifre tabulari |

Timecode, durate e countdown usano SF Mono/tabular. Non si scende sotto 11 pt.

### Griglia e forme

- Griglia base 4 pt; spaziature principali 4/8/12/16/20/24.
- Altezze controlli 28/32/40 pt; hit area ≥ 28 pt, ≥ 56 pt in Live.
- Raggi: pannelli 10–12, controlli 6–8, clip 6, capsule piene.
- Linee e separatori da 1 px allineati al pixel fisico.

### Movimento

Hover, selezione e comparsa dei pannelli usano 120–200 ms ease-out. Se macOS segnala
“Riduci movimento”, il kit usa transizioni istantanee. Il ridisegno avviene su input,
riproduzione o animazioni attive; a riposo l'app non ridisegna in continuazione.

## Componenti

`nerd-ui-kit` espone:

- **Azione:** Button (primary, secondary, tinted, plain, destructive, icon-only), Toggle,
  Segmented, MenuButton/Popup, Keycap.
- **Input:** SearchField, TextField, NumberField con stepper, Slider.
- **Contenuto:** Card, SectionHeader comprimibile, ListRow, Toolbar, StatusPill, Badge,
  ProgressBar.
- **Stato:** Sheet/Alert, Toast, EmptyState, DropZone, Tooltip contestuale.
- **Media:** Waveform, VuMeter stereo con peak hold, Timeline.
- **Icone:** ~50 glifi vettoriali disegnati con `Painter`, peso 1.5 pt, 16/20 pt.

Ogni componente implementa normale, hover, premuto, focus con anello d'accento, disabilitato
e selezionato. La pagina `--gallery` mostra tutti gli stati in scuro e chiaro affiancati.

## Struttura della finestra

- **Titlebar unificata, 52 pt.** Contenuto sotto i semafori nativi, area trascinabile.
  A sinistra toggle sidebar, nome serata modificabile e stato (`Salvato · 12 brani · 48 min`).
  Al centro capsula dello show con pallino di stato (Fermo, In onda, Sottofondo, Pausa).
  A destra uscita video, Blackout e inspector.
- **Page bar, 56 pt.** Libreria, Scaletta, Titoli, Live, Impostazioni con icona ed etichetta;
  a destra solo il pulsante `?` con il popover delle scorciatoie.
- Pannelli ridimensionabili con maniglia invisibile da 8 pt, comprimibili; larghezze minime
  e massime memorizzate per pagina. Finestra minima 1100×700.
- Scrollbar sottili, a comparsa. Le superfici sono opache e curate; non si usa vibrancy
  simulata quando non disponibile.

## Pagine

- **Libreria** — sidebar 280 pt, ricerca con lente, filtro Audio/Video, ordinamento; righe
  da 44 pt con onda, titolo, artista e durata; viewer 16:9 reale; inspector a sezioni
  comprimibili; drop zone e stato vuoto curati.
- **Scaletta** — viewer e inspector in alto, timeline in basso con righello, quattro
  tracce, clip arrotondate con onda, playhead, zoom e snap; modalità Timeline/Lista.
- **Titoli** — template a card, anteprima 16:9 con maniglie, striscia titolo, inspector per
  stile, testi e animazione.
- **Live** — card In onda e Prossimo, GO verde con keycap Spazio, trasporto, Panic rosso,
  anteprima uscita, sottofondo, VU stereo, checklist e scaletta compatta.
- **Impostazioni** — gruppi in stile Impostazioni di Sistema: Audio, Schermi, Sottofondo,
  Automazione, Titoli, Aspetto, Aggiornamenti, Serata, Dati, Scorciatoie.

## Accessibilità

- Contrasto testo ≥ 4.5:1; componenti e stati ≥ 3:1.
- Focus visibile con anello d'accento su tutti i controlli interattivi.
- Ordine di tabulazione naturale, scorciatoie documentate nel popover `?` e nei tooltip.
- Stati mai affidati al solo colore: sempre icona o etichetta.
- “Aumenta contrasto” di macOS rafforza separatori e testo secondario; “Riduci movimento”
  disattiva le transizioni.
- Etichette accessibili sui controlli icon-only.

## Avvio e strumenti

```bash
cargo run -p nerd-app -- --demo --theme=dark --page=live
cargo run -p nerd-app -- --gallery
```

Flag disponibili: `--demo`, `--gallery`, `--page=libreria|scaletta|titoli|live|impostazioni`,
`--theme=dark|light|system`, `--size=1100x700`, `--screenshot=path.png` (strumento di
verifica, chiude l'app dopo lo scatto).

## Screenshot

`docs/screenshots/` contiene le cinque pagine in scuro e chiaro, a 1100×700 e 2560×1440,
più gli stati vuoti in scuro. Gli scatti 1100×700 sono catturati a 2× e ridotti; quelli
2560×1440 sono catturati a 1280×720 con fattore Retina 2×. La cattura usa il framebuffer
egui (`--screenshot`), quindi non include i semafori nativi disegnati da macOS: la titlebar
personalizzata e il suo contenuto sono invece visibili.

## Icona

Sorgente vettoriale in `assets/icon.svg`: squircle grafite, onda verde–ciano e glifo play,
senza testo. Il bundle usa `assets/AppIcon.icns`, generato dal PNG 1024×1024 con `iconutil`.

## Verifica

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- Confronto visivo delle cinque pagine a 1100×700 (minimo) e 2560×1440, in entrambi i temi.
