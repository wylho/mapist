//! Registering Mapist with an MCP client.
//!
//! Mirrors the Ideario's `mcp/setup.rs`, including its two rules that matter
//! more than they look:
//!
//! * A config file that is not valid JSON is reported and **left untouched**.
//!   Clobbering someone's editor config to install ourselves would be worse
//!   than failing.
//! * Writes go to a temporary file that is then renamed over the target, so the
//!   config is never observed half-written.
//!
//! Everything else in the client's config is preserved.

use serde::Serialize;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// Errors from config handling. Plain strings, because every one of them ends
/// up shown to a person as-is.
pub type Result<T> = std::result::Result<T, String>;

/// Key this server is registered under in `mcpServers`.
const NAME: &str = "mapist";

/// What the frontend needs to show on a settings page.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInfo {
    /// The program a client should run.
    pub command: String,
    /// Arguments passed to it.
    pub args: Vec<String>,
    /// Ready-to-paste command for Claude Code.
    pub claude_code: String,
    /// Path of the Claude Desktop config, when one can be located.
    pub desktop_config: Option<String>,
    /// Whether that config's directory exists.
    pub desktop_found: bool,
    /// Whether it already points at this executable.
    pub desktop_installed: bool,
}

/// This executable's path.
///
/// On Linux an AppImage runs from a temporary mount, so the path of the
/// `.AppImage` itself is the one worth registering.
pub fn command() -> Result<PathBuf> {
    if let Some(p) = std::env::var_os("APPIMAGE").filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(p));
    }
    std::env::current_exe().map_err(|e| e.to_string())
}

/// Where Claude Desktop keeps its config, if the platform has a config dir.
pub fn desktop_config() -> Option<PathBuf> {
    config_dir().map(|d| d.join("Claude").join("claude_desktop_config.json"))
}

fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }
}

fn quote(p: &str) -> String {
    format!("\"{}\"", p.replace('"', "\\\""))
}

/// Describe the current registration state.
pub fn info() -> Result<McpInfo> {
    let cmd = command()?.display().to_string();
    let config = desktop_config();
    Ok(McpInfo {
        claude_code: format!(
            "claude mcp add --scope user {NAME} -- {} --mcp",
            quote(&cmd)
        ),
        desktop_found: config
            .as_ref()
            .and_then(|c| c.parent())
            .is_some_and(Path::exists),
        desktop_installed: config.as_ref().is_some_and(|c| installed(c, &cmd)),
        desktop_config: config.map(|c| c.display().to_string()),
        command: cmd,
        args: vec!["--mcp".into()],
    })
}

/// Read a client config. A missing file reads as empty; a malformed one is an
/// error the caller must surface rather than overwrite.
fn read(config: &Path) -> Result<Map<String, Value>> {
    match std::fs::read_to_string(config) {
        Ok(text) if text.trim().is_empty() => Ok(Map::new()),
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(o)) => Ok(o),
            _ => Err(format!(
                "{} is not valid JSON; it was left untouched",
                config.display()
            )),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
        Err(e) => Err(e.to_string()),
    }
}

/// Write a client config atomically.
fn write(config: &Path, o: Map<String, Value>) -> Result<()> {
    if let Some(dir) = config.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&Value::Object(o)).map_err(|e| e.to_string())? + "\n";
    let tmp = config.with_extension("json.mapist-tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, config).map_err(|e| e.to_string())
}

/// Whether the config already registers this server at `command`.
pub fn installed(config: &Path, command: &str) -> bool {
    read(config)
        .ok()
        .and_then(|o| {
            o.get("mcpServers")?
                .get(NAME)?
                .get("command")?
                .as_str()
                .map(|c| c == command)
        })
        .unwrap_or(false)
}

/// Add or update this server under `mcpServers`, leaving everything else alone.
pub fn install(config: &Path, command: &str) -> Result<()> {
    let mut o = read(config)?;
    let servers = o.entry("mcpServers").or_insert_with(|| json!({}));
    let Some(servers) = servers.as_object_mut() else {
        return Err("mcpServers is not an object; it was left untouched".into());
    };
    servers.insert(
        NAME.into(),
        json!({ "command": command, "args": ["--mcp"] }),
    );
    write(config, o)
}

/// Remove this server from `mcpServers`, leaving everything else alone.
pub fn uninstall(config: &Path) -> Result<()> {
    let mut o = read(config)?;
    if let Some(servers) = o.get_mut("mcpServers").and_then(Value::as_object_mut) {
        servers.remove(NAME);
    }
    write(config, o)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "mapist-mcp-setup-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn install_creates_the_file_when_absent() {
        let d = tempdir();
        let c = d.join("Claude").join("claude_desktop_config.json");
        install(&c, "/apps/mapist").unwrap();
        assert!(installed(&c, "/apps/mapist"));
        assert!(!installed(&c, "/other/mapist"));
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn install_keeps_the_rest_of_the_config() {
        let d = tempdir();
        let c = d.join("claude_desktop_config.json");
        std::fs::write(
            &c,
            r#"{"theme":"dark","mcpServers":{"files":{"command":"npx","args":["x"]}}}"#,
        )
        .unwrap();

        install(&c, "/apps/mapist").unwrap();
        install(&c, "/apps/mapist").unwrap(); // idempotent

        let v: Value = serde_json::from_str(&std::fs::read_to_string(&c).unwrap()).unwrap();
        assert_eq!(v["theme"], "dark", "unrelated keys must survive");
        assert_eq!(v["mcpServers"]["files"]["command"], "npx");
        assert_eq!(
            v["mcpServers"]["mapist"],
            json!({ "command": "/apps/mapist", "args": ["--mcp"] })
        );

        uninstall(&c).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&c).unwrap()).unwrap();
        assert!(v["mcpServers"].get("mapist").is_none());
        assert_eq!(v["mcpServers"]["files"]["command"], "npx");
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn broken_config_is_left_alone() {
        let d = tempdir();
        let c = d.join("claude_desktop_config.json");
        std::fs::write(&c, "{ this is not json").unwrap();
        assert!(install(&c, "/apps/mapist").is_err());
        assert_eq!(std::fs::read_to_string(&c).unwrap(), "{ this is not json");
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn mcp_servers_of_the_wrong_shape_is_refused() {
        let d = tempdir();
        let c = d.join("claude_desktop_config.json");
        std::fs::write(&c, r#"{"mcpServers": "oops"}"#).unwrap();
        assert!(install(&c, "/apps/mapist").is_err());
        assert_eq!(
            std::fs::read_to_string(&c).unwrap(),
            r#"{"mcpServers": "oops"}"#
        );
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn claude_code_command_quotes_the_path() {
        let i = info().unwrap();
        assert!(i
            .claude_code
            .starts_with("claude mcp add --scope user mapist -- \""));
        assert!(i.claude_code.ends_with("\" --mcp"));
        assert_eq!(i.args, vec!["--mcp".to_string()]);
    }

    #[test]
    fn uninstall_on_missing_file_is_harmless() {
        let d = tempdir();
        let c = d.join("nope").join("claude_desktop_config.json");
        uninstall(&c).unwrap();
        assert!(
            c.exists(),
            "uninstall writes an empty config rather than failing"
        );
        std::fs::remove_dir_all(d).unwrap();
    }
}
