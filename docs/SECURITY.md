# Security Model — PC Doctor

## Principles
1. **Zero External Data Exfiltration**: PC Doctor performs all metric collection and diagnostic evaluations locally in-process. No metrics are uploaded to cloud servers.
2. **Restricted Tauri Permissions**: Only explicitly declared Tauri commands are registered in capabilities. No shell command execution or arbitrary file system read/write permissions are granted to the webview.
3. **Least Privilege System Access**: PC Doctor runs as a standard user process. It does not require or request Administrator privileges for core monitoring tasks.
4. **Deterministic Local Diagnostics**: Recommendations and advice are generated from local rules and an offline knowledge base, preventing untrusted remote code injection.
