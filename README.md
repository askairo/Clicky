# Clicky

Desktop tool inspired by SwitchHosts for switching environment variables (not only hosts).

## MVP status

- Windows first
- Tauri + React desktop app
- Environment config from `config/environments.yaml`
- One-click apply writes user-level variables via `setx`
- Sensitive values are masked in the UI by default
- Post-apply hooks can run commands after switching

## Run (Windows)

```powershell
npm install
npm run tauri dev
```

## Config file

`config/environments.yaml`

Example:

```yaml
environments:
  dev:
    variables:
      ZNDER_MYSQL_HOST: "192.168.60.176"
    hooks:
      post:
        - "echo Clicky switched to dev"
  sit:
    variables:
      ZNDER_MYSQL_HOST: "120.76.142.193"
```

`hooks.post` commands run after variables are applied. They can be used for
follow-up actions such as opening a new terminal, restarting a target tool, or
running a quick verification command.

## Note

`setx` updates user-level environment variables for new processes. Existing
terminals, IDEs, and app processes usually need to be reopened before they read
the updated values.
