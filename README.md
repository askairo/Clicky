# clicky

English | [中文](README_ZH.md)

clicky is a cross-platform desktop app inspired by SwitchHosts. It focuses on grouped environment-variable management and one-click environment switching.

## Current Status

- Windows and macOS support
- Desktop app built with Tauri and React
- Layered frontend architecture: `ui / appservice / service / domain / utils`
- Environment-variable configuration management and one-click apply
- Automatic export of an IDEA-friendly application-level `.env` snapshot after each switch
- Sensitive values are masked by default
- Post-switch `hooks.post` command support

## Install

### Option 1: Download from GitHub Releases

- Windows: download `.exe` or `.msi`
- macOS: download `.dmg`
- Releases: <https://github.com/askairo/clicky/releases>

### Option 2: Homebrew on macOS

```bash
brew tap askairo/tap
brew install clicky
```

## Local Development

```powershell
npm install
npm run tauri dev
```

## First Launch on macOS

If macOS reports that the app is damaged or cannot be opened, Gatekeeper is usually blocking an unsigned app. Run:

```bash
xattr -dr com.apple.quarantine /Applications/clicky.app
open /Applications/clicky.app
```

If needed, go to `System Settings -> Privacy & Security` and choose `Open Anyway` for clicky.

## Config and Storage

- Example config file: `config/environments.example.yaml`
- Local storage is the primary source of truth
- YAML is mainly used for import, export, and template examples

## Notes

Environment-variable updates apply to new processes. Existing terminals, IDEs, and target apps usually need to be restarted before they can read updated values.

If you point an IntelliJ IDEA `Run/Debug Configuration` to `~/.clicky/env/idea/current.env`, clicky refreshes that file after each switch so newly launched debug tasks can read the latest values.

## Related Docs

- Platform differences and FAQ: `docs/platform-differences.md`
- Release and signing strategy: `docs/release-signing.md`
- Acceptance scripts and cases: `docs/acceptance.md`
