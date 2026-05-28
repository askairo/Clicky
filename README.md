# envflow

Desktop tool inspired by SwitchHosts for switching environment variables (not only hosts).

## MVP status

- Windows first
- Tauri + React desktop app
- Environment config from `desktop/config/environments.yaml`
- One-click apply writes variables via `setx`

## Run (Windows)

```powershell
cd desktop
npm install
npm run tauri dev
```

## Config file

`desktop/config/environments.yaml`

Example:

```yaml
environments:
  dev:
    variables:
      ZNDER_MYSQL_HOST: "192.168.60.176"
  sit:
    variables:
      ZNDER_MYSQL_HOST: "120.76.142.193"
```

## Note

`setx` updates user-level environment variables for new processes.
Open a new terminal/session for downstream tools to read the updated values.
