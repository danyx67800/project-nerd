# Project Nerd

App macOS per gestire brani e video musicali durante i concerti di una scuola di musica. Pensata per un operatore sotto stress: comandi grandi, stati sempre leggibili, niente sorprese.

> Versione alpha. L'app non è notarizzata: al primo avvio macOS mostrerà un avviso.

## Stato alpha

Sono completi: modello e state machine, motore audio, libreria, scaletta, titoli,
pagina Live, impostazioni, OTA firmato, packaging e workflow di release.
L'integrazione video AVFoundation (elenco schermi, probe, anteprime e playback a
frame) è l'ultimo blocco ancora in corso; in questa build i file video vengono
importati e il loro audio è usato dallo show, ma il quadro non è ancora proiettato.

## Requisiti

- macOS 13 o successivo (Apple Silicon o Intel)
- Rust stable (1.95+)
- Xcode Command Line Tools per packaging e codec video

## Avvio in sviluppo

```bash
cargo run -p nerd-app
```

La prima compilazione scarica e compila le dipendenze (qualche minuto).

## Test

```bash
cargo test -p nerd-core          # logica dello show, persistenza, libreria
cargo test --workspace           # tutto ciò che non richiede hardware
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

La state machine include una simulazione di due ore di show automatico. Per il soak reale con audio:

```bash
cargo run -p nerd-tools -- soak --minutes 120
```

## Media di test

Il repository non contiene materiale protetto. Genera toni e pattern video sintetici:

```bash
cargo run -p nerd-tools -- gen-media --out test-media
```

## Pacchetto `.app` e `.dmg`

```bash
./scripts/bundle.sh                 # app ad-hoc firmata
./scripts/bundle.sh --universal     # installa i target aarch64 + x86_64
OTA_KEY=.ota/ota-private.key ./scripts/bundle.sh --universal --dmg
```

Il bundle produce `dist/Project Nerd.app`, lo zip, il file `.sha256` e, se
`OTA_KEY` è impostata, la firma `.zip.sha256.sig` richiesta dall'updater.

Il bundle identifier è configurabile con `BUNDLE_ID`. Il nome dell'app è definito solo in `crates/nerd-core/src/branding.rs`.

## Primo avvio di un'app non notarizzata

1. Scompatta `Project Nerd.zip` o apri il `.dmg`.
2. Trascina `Project Nerd.app` in `/Applications`.
3. Tasto destro sull'app → **Apri** → **Apri**.
   In alternativa: `xattr -dr com.apple.quarantine "/Applications/Project Nerd.app"`.

Se l'app è firmata con Developer ID e notarizzata, il doppio click è sufficiente.

## Dove vivono i dati

`~/Library/Application Support/Project Nerd/`

- `preferences.json`, `library.json`
- `recovery.nerdshow` (snapshot di emergenza)
- log in `Logs/`

Le serate sono file `.nerdshow` salvati dove decide l'utente. L'app funziona offline; la rete serve solo per gli aggiornamenti.

## Scorciatoie principali

| Azione | Tasto |
|---|---|
| GO (avvia / prossimo) | Spazio |
| Pausa / riprendi | P |
| Stop con fade | Esc |
| Salta | N |
| Indietro | B |
| Panic | ⇧⌘P |
| Blackout proiettore | ⌘B |
| Nuova serata | ⌘N |
| Apri | ⌘O |
| Salva | ⌘S |
| Impostazioni | ⌘, |
| Pagine | ⌘1 … ⌘5 |

## Aggiornamenti

Il repository pubblico è già impostato su `danyx67800/project-nerd`; puoi cambiarlo in Impostazioni → Aggiornamenti e scegliere Stabile o Alpha. Le release devono includere zip, `.sha256` e firma Ed25519. Le release sono pubblicate su <https://github.com/danyx67800/project-nerd/releases>. Le chiavi si generano con:

```bash
cargo run -p nerd-tools -- keygen --out .ota/ota-private.key
cargo run -p nerd-tools -- sign --key .ota/ota-private.key --file dist/Project\ Nerd.zip.sha256
cargo run -p nerd-tools -- verify --key .ota/ota-private.key.pub --file dist/Project\ Nerd.zip.sha256 --sig dist/Project\ Nerd.zip.sha256.sig
```

`keygen` scrive la chiave privata (`.ota/`, ignorata da git) e quella pubblica in
`.pub`. La pubblica è già installata in `crates/nerd-app/src/update.rs`; la privata
deve essere copiata nel secret GitHub `OTA_PRIVATE_KEY` e non va mai committata.
