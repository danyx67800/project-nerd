# Project Nerd

App macOS per gestire brani e video musicali durante i concerti di una scuola di musica. Pensata per un operatore sotto stress: comandi grandi, stati sempre leggibili, niente sorprese.

> Versione alpha. L'app non è notarizzata: al primo avvio macOS mostrerà un avviso.

## Requisiti

- macOS 13 o successivo (Apple Silicon o Intel)
- Rust stable (1.88+)
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
./scripts/bundle.sh --universal     # richiede i target aarch64 + x86_64
./scripts/bundle.sh --dmg
```

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

Imposta il repository `owner/repo` in Impostazioni → Aggiornamenti e scegli Stabile o Alpha. Le release devono includere zip, `.sha256` e firma Ed25519. Le chiavi si generano con:

```bash
cargo run -p nerd-tools -- keygen --out ota-private.key
```

La chiave pubblica va incollata in `crates/nerd-app/src/update.rs`; quella privata nei secret GitHub `OTA_PRIVATE_KEY`.
