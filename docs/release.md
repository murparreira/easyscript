# Releasing

What a public release of easyscript needs, what is done, and what only the team can do: the name, the certificates, the update keys.

## Done

- Installers for Linux (AppImage, `.deb`, `.rpm`), macOS (one `.dmg` for Apple silicon and Intel) and Windows (`-setup.exe`, `.msi`), and the command line for each, from the release workflow (`.github/workflows/release.yml`). A `v*` tag makes a draft release; running the workflow by hand builds the same as workflow artifacts. Not signed yet.
- Installing in one command, with nothing to install first: `scripts/install.sh` (macOS, Linux) and `scripts/install.ps1` (Windows) download the latest published release and install the app and the command line for the user, without an administrator. Downloaded that way the installers carry no quarantine or mark of the web, so macOS and Windows do not warn. The `Install` workflow tries them on clean machines each time a release is published.
- The macOS app is signed ad hoc (`bundle.macOS.signingIdentity: "-"`), which Apple silicon needs to run it at all; a Developer ID certificate replaces that (below).
- Building from source in one command: `scripts/build-from-source.sh` and `scripts/build-from-source.ps1`, for developers ([developing](development.md)).
- The app in English and Portuguese (Brazil), following the system's language, with a menu to change it. The checks' messages too; exports write their own words in either.
- Documents: [the README](../README.md), [the guide](guide.md), [the project file](project-file.md), [the formats](formats/README.md), [developing](development.md).
- MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`).

## The name

The app is *easyscript*, as its repository on GitHub. The name is in:

- `app/src-tauri/tauri.conf.json`: `productName`, `identifier` (`com.busyboys.easyscript`), the window title.
- The binaries: `easyscript` for the command line (`crates/cli/Cargo.toml`), `easyscript-app` for the app (`app/src-tauri/Cargo.toml`).
- The project file, `easyscript.toml`, and its documents folder, `.easyscript/`.
- The password store entry (`KEYRING_SERVICE` in `app/src-tauri/src/main.rs`) and the user agent sent to GitHub (`crates/github`).
- The icon: `app/icon.svg`, from which `npm run tauri icon app/icon.png` makes every size (`app/src-tauri/icons`) once it is drawn at 1024 pixels (`rsvg-convert -w 1024 -h 1024 app/icon.svg -o app/icon.png`).

The team's own to change: the GitHub OAuth app's name, which GitHub shows when it asks to authorize (github.com → Settings → Developer settings → OAuth Apps). Its client id does not change. For a public app, an organisation should own it rather than a person.

The identifier decides where each system keeps the app's data: changing it again loses settings and sign-ins.

## Signing

Unsigned installers work, but each system warns: macOS refuses to open the app until it is allowed in System Settings, Windows SmartScreen asks for confirmation. Signing is per system, with certificates the team holds. The release workflow gets them as repository secrets (Settings → Secrets and variables → Actions); none goes in the repository.

**macOS** (Apple Developer Program, about 99 USD a year):

1. A *Developer ID Application* certificate, exported from Keychain Access as `.p12` with a password.
2. Secrets: `APPLE_CERTIFICATE` (the `.p12`, base64: `base64 -i cert.p12`), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` (`Developer ID Application: Name (TEAMID)`), and for notarisation `APPLE_ID`, `APPLE_PASSWORD` (an app-specific password from appleid.apple.com) and `APPLE_TEAM_ID`.
3. In the workflow's `tauri-action` step, pass those secrets as `env`. Tauri signs and notarises when they are set.

**Windows**, one of:

- Azure Trusted Signing (cheapest for an organisation, a few USD a month): an account and a certificate profile, then a `signCommand` in `bundle.windows` of `tauri.conf.json` and the Azure credentials as secrets.
- An OV or EV code-signing certificate from a certificate authority: its thumbprint in `bundle.windows.certificateThumbprint`, the certificate imported on the runner from a secret.

**Linux**: nothing to sign for `.deb` and AppImage.

## Updates

Not built in yet, on purpose. Rule 5 says nothing leaves the machine unless the writer asks: so an update check would be a **Check for updates…** button, never a check at start. Tauri's updater needs a key pair to sign updates, and the private key must be the team's alone:

1. `npm run tauri signer generate -- -w ~/.tauri/easyscript.key` (choose a password; keep the key and the password safe: lose them, and installed apps can no longer be updated).
2. Secrets: `TAURI_SIGNING_PRIVATE_KEY` (the key file's text) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
3. Give the public key (`~/.tauri/easyscript.key.pub`) to whoever wires the updater: `tauri-plugin-updater` with that key and the endpoint `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`, `bundle.createUpdaterArtifacts`, and the button.

## Before going public

- Rule 4: no team's script in the repository. The tests use fixtures written for them (`crates/*/tests/fixtures`); tests on a real project read it from `EASYSCRIPT_PROJECT` and skip without it. Check with `git grep` for character names, dialogue and paths from the reference game before the repository goes public; the planning documents (`PLAN.md`, `CLAUDE.md`) name it on purpose.
- The repository's history has nothing secret (no tokens: the GitHub client id is public by design; there is no client secret).
- CI passes on the three systems (`.github/workflows/ci.yml`).
- A first release from a tag, installed and opened on each system: open a project, edit, save, commit, export, import.
