use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::{self, BufRead, BufReader, Read, Write},
    net::{IpAddr, Shutdown, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    thread,
    time::Duration,
};
use tauri::{Manager, State};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
struct AppPaths {
    app_data_dir: PathBuf,
    database_path: PathBuf,
    config_path: PathBuf,
    credentials_path: PathBuf,
    profiles_dir: PathBuf,
    backups_dir: PathBuf,
    browsers_dir: PathBuf,
}

impl AppPaths {
    fn new(app_data_dir: PathBuf) -> Self {
        Self {
            database_path: app_data_dir.join("app.db"),
            config_path: app_data_dir.join("config.json"),
            credentials_path: app_data_dir.join("credentials.enc"),
            profiles_dir: app_data_dir.join("profiles"),
            backups_dir: app_data_dir.join("backups"),
            browsers_dir: app_data_dir.join("browsers"),
            app_data_dir,
        }
    }

    fn ensure(&self) -> Result<(), String> {
        fs::create_dir_all(&self.app_data_dir).map_err(|err| err.to_string())?;
        fs::create_dir_all(&self.profiles_dir).map_err(|err| err.to_string())?;
        fs::create_dir_all(&self.backups_dir).map_err(|err| err.to_string())?;
        fs::create_dir_all(&self.browsers_dir).map_err(|err| err.to_string())?;
        Ok(())
    }
}

struct AppState {
    paths: AppPaths,
    db: Mutex<Connection>,
}

#[derive(Debug, Serialize)]
struct Profile {
    id: String,
    name: String,
    notes: String,
    tags: Vec<String>,
    browser_binary_path: Option<String>,
    user_data_dir: String,
    proxy_id: Option<String>,
    proxy_scheme: Option<String>,
    proxy_host: Option<String>,
    proxy_port: Option<i64>,
    proxy_username: Option<String>,
    proxy_password_saved: bool,
    user_agent: Option<String>,
    language: Option<String>,
    timezone: Option<String>,
    profile_color: Option<String>,
    webrtc_policy: String,
    webrtc_disabled: bool,
    window_width: Option<i64>,
    window_height: Option<i64>,
    window_x: Option<i64>,
    window_y: Option<i64>,
    launch_args: Vec<String>,
    startup_urls: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_launched_at: Option<DateTime<Utc>>,
    running: bool,
}

#[derive(Clone, Debug, Serialize)]
struct BrowserCandidate {
    name: String,
    app_path: String,
    binary_path: String,
    exists: bool,
}

#[derive(Debug, Serialize)]
struct ManagedBrowserStatus {
    installed: bool,
    install_dir: String,
    binary_path: Option<String>,
    message: String,
}

#[derive(Debug, Serialize)]
struct ProxyProfile {
    id: String,
    name: String,
    scheme: String,
    host: String,
    port: i64,
    username: Option<String>,
    password_saved: bool,
    notes: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct LaunchProfileResult {
    profile_id: String,
    browser_binary_path: String,
    args: Vec<String>,
    launched_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ProxyTestResult {
    ok: bool,
    message: String,
    observed_ip: Option<String>,
}

#[derive(Debug, Serialize)]
struct RunningProfileProcess {
    pid: u32,
    command: String,
}

#[derive(Debug, Deserialize)]
struct CreateProfileInput {
    name: String,
    notes: Option<String>,
    tags: Option<Vec<String>>,
    browser_binary_path: Option<String>,
    proxy_id: Option<String>,
    proxy_scheme: Option<String>,
    proxy_host: Option<String>,
    proxy_port: Option<i64>,
    proxy_username: Option<String>,
    proxy_password: Option<String>,
    user_agent: Option<String>,
    language: Option<String>,
    timezone: Option<String>,
    profile_color: Option<String>,
    webrtc_policy: Option<String>,
    webrtc_disabled: Option<bool>,
    window_width: Option<i64>,
    window_height: Option<i64>,
    window_x: Option<i64>,
    window_y: Option<i64>,
    launch_args: Option<Vec<String>>,
    startup_urls: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct UpdateProfileInput {
    name: String,
    notes: Option<String>,
    tags: Option<Vec<String>>,
    browser_binary_path: Option<String>,
    proxy_id: Option<String>,
    proxy_scheme: Option<String>,
    proxy_host: Option<String>,
    proxy_port: Option<i64>,
    proxy_username: Option<String>,
    proxy_password: Option<String>,
    user_agent: Option<String>,
    language: Option<String>,
    timezone: Option<String>,
    profile_color: Option<String>,
    webrtc_policy: Option<String>,
    webrtc_disabled: Option<bool>,
    window_width: Option<i64>,
    window_height: Option<i64>,
    window_x: Option<i64>,
    window_y: Option<i64>,
    launch_args: Option<Vec<String>>,
    startup_urls: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ProxyProfileInput {
    name: String,
    scheme: String,
    host: String,
    port: i64,
    username: Option<String>,
    password: Option<String>,
    notes: Option<String>,
}

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn encode_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|err| err.to_string())
}

fn decode_json_vec(value: String) -> Result<Vec<String>, String> {
    serde_json::from_str(&value).map_err(|err| err.to_string())
}

fn parse_datetime(value: String) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(&value)
        .map(|datetime| datetime.with_timezone(&Utc))
        .map_err(|err| err.to_string())
}

fn parse_optional_datetime(value: Option<String>) -> Result<Option<DateTime<Utc>>, String> {
    value.map(parse_datetime).transpose()
}

fn normalize_profile_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Profile name is required.".to_string());
    }
    Ok(trimmed.to_string())
}

fn normalize_required_name(name: &str, label: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is required."));
    }
    Ok(trimmed.to_string())
}

fn normalize_proxy_scheme(scheme: Option<String>) -> Result<Option<String>, String> {
    match scheme
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
    {
        Some(value) if matches!(value.as_str(), "http" | "https" | "socks4" | "socks5") => {
            Ok(Some(value))
        }
        Some(value) => Err(format!("Unsupported proxy scheme: {value}")),
        None => Ok(None),
    }
}

fn normalize_proxy_host(host: Option<String>) -> Option<String> {
    host.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_profile_color(value: Option<String>) -> Result<Option<String>, String> {
    let Some(value) = normalize_optional_text(value) else {
        return Ok(None);
    };
    let color = value.trim();
    if color.len() == 7
        && color.starts_with('#')
        && color[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Ok(Some(color.to_ascii_uppercase()))
    } else {
        Err("Profile color must use #RRGGBB format.".to_string())
    }
}

fn default_profile_color(index: usize) -> &'static str {
    const COLORS: [&str; 10] = [
        "#2563EB", "#059669", "#DC2626", "#7C3AED", "#D97706", "#0F766E", "#DB2777", "#4F46E5",
        "#65A30D", "#0891B2",
    ];
    COLORS[index % COLORS.len()]
}

fn parse_profile_color(color: &str) -> Result<(u8, u8, u8), String> {
    let normalized = normalize_profile_color(Some(color.to_string()))?
        .ok_or_else(|| "Profile color is required.".to_string())?;
    let r = u8::from_str_radix(&normalized[1..3], 16).map_err(|err| err.to_string())?;
    let g = u8::from_str_radix(&normalized[3..5], 16).map_err(|err| err.to_string())?;
    let b = u8::from_str_radix(&normalized[5..7], 16).map_err(|err| err.to_string())?;
    Ok((r, g, b))
}

fn chrome_argb_color(r: u8, g: u8, b: u8) -> i32 {
    let value = 0xff00_0000u32 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
    value as i32
}

fn object_slot<'a>(value: &'a mut Value, key: &str) -> &'a mut serde_json::Map<String, Value> {
    if !value.is_object() {
        *value = json!({});
    }
    let object = value.as_object_mut().expect("value was just set to object");
    object
        .entry(key.to_string())
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .expect("object slot should be an object")
}

fn ensure_chrome_profile_preferences(profile: &Profile) -> Result<(), String> {
    let default_profile_dir = Path::new(&profile.user_data_dir).join("Default");
    fs::create_dir_all(&default_profile_dir).map_err(|err| err.to_string())?;
    let preferences_path = default_profile_dir.join("Preferences");
    let mut preferences = if preferences_path.exists() {
        let contents = fs::read_to_string(&preferences_path).map_err(|err| err.to_string())?;
        serde_json::from_str::<Value>(&contents).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    let root = preferences
        .as_object_mut()
        .ok_or_else(|| "Chrome preferences root is invalid.".to_string())?;
    let profile_prefs = root
        .entry("profile".to_string())
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| "Chrome profile preferences are invalid.".to_string())?;
    profile_prefs.insert("name".to_string(), json!(profile.name));
    profile_prefs.insert("avatar_index".to_string(), json!(26));

    if let Some(color) = profile.profile_color.as_deref() {
        let (r, g, b) = parse_profile_color(color)?;
        let user_color = chrome_argb_color(r, g, b);
        let extensions = object_slot(&mut preferences, "extensions");
        extensions.insert("theme".to_string(), json!({ "id": "user_color_theme_id" }));
        let browser = object_slot(&mut preferences, "browser");
        browser.insert(
            "theme".to_string(),
            json!({
                "color_scheme": 1,
                "color_scheme2": 1,
                "color_variant": 1,
                "color_variant2": 1,
                "follows_system_colors": false,
                "user_color": user_color,
                "user_color2": user_color
            }),
        );
    }

    fs::write(
        preferences_path,
        serde_json::to_string_pretty(&preferences).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())
}

fn normalize_webrtc_policy(
    policy: Option<String>,
    disabled: Option<bool>,
) -> Result<String, String> {
    match policy
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
    {
        Some(value) if matches!(value.as_str(), "default" | "public_only" | "proxy_only") => {
            Ok(value)
        }
        Some(value) => Err(format!("Unsupported WebRTC policy: {value}")),
        None if disabled.unwrap_or(false) => Ok("proxy_only".to_string()),
        None => Ok("default".to_string()),
    }
}

fn is_webrtc_direct_udp_restricted(policy: &str) -> bool {
    matches!(policy, "public_only" | "proxy_only")
}

fn normalize_proxy_port(port: Option<i64>) -> Result<Option<i64>, String> {
    match port {
        Some(value) if (1..=65535).contains(&value) => Ok(Some(value)),
        Some(value) => Err(format!(
            "Proxy port must be between 1 and 65535, got {value}."
        )),
        None => Ok(None),
    }
}

fn normalize_proxy_settings(
    scheme: Option<String>,
    host: Option<String>,
    port: Option<i64>,
) -> Result<(Option<String>, Option<String>, Option<i64>), String> {
    let scheme = normalize_proxy_scheme(scheme)?;
    let host = normalize_proxy_host(host);
    let port = normalize_proxy_port(port)?;

    if scheme.is_some() || host.is_some() || port.is_some() {
        if scheme.is_none() || host.is_none() || port.is_none() {
            return Err(
                "Proxy scheme, host, and port are all required when proxy is enabled.".to_string(),
            );
        }
    }

    Ok((scheme, host, port))
}

fn normalize_required_proxy_settings(
    scheme: String,
    host: String,
    port: i64,
) -> Result<(String, String, i64), String> {
    let scheme = normalize_proxy_scheme(Some(scheme))?
        .ok_or_else(|| "Proxy scheme is required.".to_string())?;
    let host =
        normalize_proxy_host(Some(host)).ok_or_else(|| "Proxy host is required.".to_string())?;
    let port =
        normalize_proxy_port(Some(port))?.ok_or_else(|| "Proxy port is required.".to_string())?;
    Ok((scheme, host, port))
}

fn normalize_dimension(value: Option<i64>, name: &str) -> Result<Option<i64>, String> {
    match value {
        Some(value) if (1..=10_000).contains(&value) => Ok(Some(value)),
        Some(value) => Err(format!("{name} must be between 1 and 10000, got {value}.")),
        None => Ok(None),
    }
}

fn normalize_coordinate(value: Option<i64>, name: &str) -> Result<Option<i64>, String> {
    match value {
        Some(value) if (-10_000..=10_000).contains(&value) => Ok(Some(value)),
        Some(value) => Err(format!(
            "{name} must be between -10000 and 10000, got {value}."
        )),
        None => Ok(None),
    }
}

fn keychain_service(profile_id: &str) -> String {
    format!("com.xiaochi.local-chromium-manager.proxy.{profile_id}")
}

fn save_proxy_password(profile_id: &str, username: &str, password: &str) -> Result<String, String> {
    let service = keychain_service(profile_id);
    let status = Command::new("security")
        .args([
            "add-generic-password",
            "-a",
            username,
            "-s",
            &service,
            "-w",
            password,
            "-U",
        ])
        .status()
        .map_err(|err| format!("Failed to write proxy password to Keychain: {err}"))?;

    if !status.success() {
        return Err("Failed to write proxy password to Keychain.".to_string());
    }

    Ok(service)
}

fn delete_proxy_password(secret_ref: &str) -> Result<(), String> {
    let output = Command::new("security")
        .args(["delete-generic-password", "-s", secret_ref])
        .output()
        .map_err(|err| format!("Failed to delete proxy password from Keychain: {err}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("could not be found")
        || stderr.contains("The specified item could not be found")
        || stderr.contains("-25300")
    {
        return Ok(());
    }

    Err(format!(
        "Failed to delete proxy password from Keychain: {}",
        stderr.trim()
    ))
}

fn read_proxy_password(secret_ref: &str) -> Result<String, String> {
    let output = Command::new("security")
        .args(["find-generic-password", "-s", secret_ref, "-w"])
        .output()
        .map_err(|err| format!("Failed to read proxy password from Keychain: {err}"))?;

    if !output.status.success() {
        return Err("Failed to read proxy password from Keychain.".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string())
}

#[derive(Clone, Debug)]
struct UpstreamProxy {
    scheme: String,
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
}

fn start_proxy_relay(upstream: UpstreamProxy) -> Result<u16, String> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|err| format!("Failed to start local proxy relay: {err}"))?;
    let port = listener
        .local_addr()
        .map_err(|err| format!("Failed to read local proxy relay port: {err}"))?
        .port();

    thread::spawn(move || {
        for client in listener.incoming() {
            match client {
                Ok(client) => {
                    let upstream = upstream.clone();
                    thread::spawn(move || {
                        if let Err(err) = handle_proxy_client(client, upstream) {
                            eprintln!("local proxy relay error: {err}");
                        }
                    });
                }
                Err(err) => eprintln!("local proxy relay accept error: {err}"),
            }
        }
    });

    Ok(port)
}

fn handle_proxy_client(mut client: TcpStream, upstream: UpstreamProxy) -> Result<(), String> {
    client
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|err| err.to_string())?;
    let mut reader = BufReader::new(client.try_clone().map_err(|err| err.to_string())?);
    let mut first_line = String::new();
    reader
        .read_line(&mut first_line)
        .map_err(|err| format!("Failed to read proxy request: {err}"))?;
    if first_line.trim().is_empty() {
        return Ok(());
    }

    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|err| format!("Failed to read proxy headers: {err}"))?;
        if line == "\r\n" || line == "\n" || line.is_empty() {
            break;
        }
        headers.push(line);
    }

    if first_line.to_ascii_uppercase().starts_with("CONNECT ") {
        let target = first_line
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| "CONNECT request is missing target.".to_string())?;
        let (host, port) = split_host_port(target, 443)?;
        let upstream_stream = connect_via_socks5(&upstream, &host, port)?;
        client
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .map_err(|err| err.to_string())?;
        relay_streams(client, upstream_stream)?;
        return Ok(());
    }

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 3 {
        return Err("HTTP proxy request has an invalid request line.".to_string());
    }

    let (host, port, path) = parse_http_proxy_target(parts[1], &headers)?;
    let mut upstream_stream = connect_via_socks5(&upstream, &host, port)?;
    let rewritten_first_line = format!("{} {} {}\r\n", parts[0], path, parts[2]);
    upstream_stream
        .write_all(rewritten_first_line.as_bytes())
        .map_err(|err| err.to_string())?;
    for header in headers {
        upstream_stream
            .write_all(header.as_bytes())
            .map_err(|err| err.to_string())?;
    }
    upstream_stream
        .write_all(b"\r\n")
        .map_err(|err| err.to_string())?;
    relay_streams(client, upstream_stream)?;
    Ok(())
}

fn parse_http_proxy_target(
    target: &str,
    headers: &[String],
) -> Result<(String, u16, String), String> {
    if let Some(rest) = target.strip_prefix("http://") {
        let (authority, path) = rest
            .split_once('/')
            .map(|(authority, path)| (authority, format!("/{path}")))
            .unwrap_or((rest, "/".to_string()));
        let (host, port) = split_host_port(authority, 80)?;
        return Ok((host, port, path));
    }

    let host_header = headers
        .iter()
        .find(|header| header.to_ascii_lowercase().starts_with("host:"))
        .ok_or_else(|| "HTTP proxy request is missing Host header.".to_string())?;
    let authority = host_header
        .split_once(':')
        .map(|(_, value)| value.trim())
        .ok_or_else(|| "HTTP proxy Host header is invalid.".to_string())?;
    let (host, port) = split_host_port(authority, 80)?;
    Ok((host, port, target.to_string()))
}

fn http_response_body(response: &str) -> String {
    let mut parts = response.splitn(2, "\r\n\r\n");
    let headers = parts.next().unwrap_or_default();
    let body = parts.next().unwrap_or_default();

    if headers
        .lines()
        .any(|line| line.eq_ignore_ascii_case("transfer-encoding: chunked"))
    {
        return decode_chunked_body(body).unwrap_or_else(|_| body.to_string());
    }

    body.to_string()
}

fn decode_chunked_body(body: &str) -> Result<String, String> {
    let mut rest = body.as_bytes();
    let mut decoded = Vec::new();

    loop {
        let line_end = rest
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| "Invalid chunked response.".to_string())?;
        let size_line = std::str::from_utf8(&rest[..line_end]).map_err(|err| err.to_string())?;
        let size_hex = size_line.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size_hex, 16).map_err(|err| err.to_string())?;
        rest = &rest[line_end + 2..];

        if size == 0 {
            break;
        }
        if rest.len() < size + 2 {
            return Err("Invalid chunked response length.".to_string());
        }
        decoded.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }

    String::from_utf8(decoded).map_err(|err| err.to_string())
}

fn extract_ip_from_body(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return value
            .get("ip")
            .and_then(|value| value.as_str())
            .map(ToString::to_string);
    }

    trimmed
        .split(|char: char| !(char.is_ascii_alphanumeric() || char == '.' || char == ':'))
        .find(|part| part.contains('.') || part.matches(':').count() >= 2)
        .map(ToString::to_string)
}

fn split_host_port(value: &str, default_port: u16) -> Result<(String, u16), String> {
    if let Some((host, port)) = value.rsplit_once(':') {
        if !host.contains(']') {
            let parsed_port = port
                .parse::<u16>()
                .map_err(|_| format!("Invalid port in proxy target: {value}"))?;
            return Ok((host.trim_matches(['[', ']']).to_string(), parsed_port));
        }
    }
    Ok((value.trim_matches(['[', ']']).to_string(), default_port))
}

fn connect_via_socks5(
    upstream: &UpstreamProxy,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream, String> {
    if upstream.scheme != "socks5" {
        return Err(
            "Local proxy relay currently supports SOCKS5 upstream proxies only.".to_string(),
        );
    }

    let upstream_addr = format!("{}:{}", upstream.host, upstream.port);
    let mut stream = TcpStream::connect(upstream_addr)
        .map_err(|err| format!("Failed to connect to upstream SOCKS5 proxy: {err}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|err| err.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|err| err.to_string())?;

    if upstream.username.is_some() {
        stream
            .write_all(&[0x05, 0x01, 0x02])
            .map_err(|err| err.to_string())?;
    } else {
        stream
            .write_all(&[0x05, 0x01, 0x00])
            .map_err(|err| err.to_string())?;
    }

    let mut method_response = [0_u8; 2];
    stream
        .read_exact(&mut method_response)
        .map_err(|err| format!("Failed during SOCKS5 method negotiation: {err}"))?;
    if method_response[0] != 0x05 {
        return Err("Upstream proxy returned an invalid SOCKS5 version.".to_string());
    }
    match method_response[1] {
        0x00 => {}
        0x02 => authenticate_socks5(&mut stream, upstream)?,
        0xff => return Err("Upstream SOCKS5 proxy rejected available auth methods.".to_string()),
        method => return Err(format!("Unsupported SOCKS5 auth method: {method}")),
    }

    let mut request = vec![0x05, 0x01, 0x00];
    append_socks5_address(&mut request, target_host)?;
    request.extend_from_slice(&target_port.to_be_bytes());
    stream
        .write_all(&request)
        .map_err(|err| format!("Failed to send SOCKS5 connect request: {err}"))?;

    read_socks5_connect_response(&mut stream)?;
    Ok(stream)
}

fn authenticate_socks5(stream: &mut TcpStream, upstream: &UpstreamProxy) -> Result<(), String> {
    let username = upstream
        .username
        .as_deref()
        .ok_or_else(|| "SOCKS5 proxy requested auth but no username is configured.".to_string())?;
    let password = upstream
        .password
        .as_deref()
        .ok_or_else(|| "SOCKS5 proxy requested auth but no password is saved.".to_string())?;
    if username.len() > 255 || password.len() > 255 {
        return Err("SOCKS5 username/password must be 255 bytes or fewer.".to_string());
    }
    let mut request = vec![0x01, username.len() as u8];
    request.extend_from_slice(username.as_bytes());
    request.push(password.len() as u8);
    request.extend_from_slice(password.as_bytes());
    stream
        .write_all(&request)
        .map_err(|err| format!("Failed to send SOCKS5 auth request: {err}"))?;
    let mut response = [0_u8; 2];
    stream
        .read_exact(&mut response)
        .map_err(|err| format!("Failed to read SOCKS5 auth response: {err}"))?;
    if response != [0x01, 0x00] {
        return Err("SOCKS5 username/password authentication failed.".to_string());
    }
    Ok(())
}

fn append_socks5_address(request: &mut Vec<u8>, host: &str) -> Result<(), String> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(addr) => {
                request.push(0x01);
                request.extend_from_slice(&addr.octets());
            }
            IpAddr::V6(addr) => {
                request.push(0x04);
                request.extend_from_slice(&addr.octets());
            }
        }
        return Ok(());
    }

    if host.len() > 255 {
        return Err("SOCKS5 target hostname is too long.".to_string());
    }
    request.push(0x03);
    request.push(host.len() as u8);
    request.extend_from_slice(host.as_bytes());
    Ok(())
}

fn read_socks5_connect_response(stream: &mut TcpStream) -> Result<(), String> {
    let mut header = [0_u8; 4];
    stream
        .read_exact(&mut header)
        .map_err(|err| format!("Failed to read SOCKS5 connect response: {err}"))?;
    if header[0] != 0x05 {
        return Err("Upstream proxy returned an invalid SOCKS5 response.".to_string());
    }
    if header[1] != 0x00 {
        return Err(format!("SOCKS5 connect failed with code {}.", header[1]));
    }
    match header[3] {
        0x01 => {
            let mut buf = [0_u8; 4 + 2];
            stream.read_exact(&mut buf).map_err(|err| err.to_string())?;
        }
        0x03 => {
            let mut len = [0_u8; 1];
            stream.read_exact(&mut len).map_err(|err| err.to_string())?;
            let mut buf = vec![0_u8; len[0] as usize + 2];
            stream.read_exact(&mut buf).map_err(|err| err.to_string())?;
        }
        0x04 => {
            let mut buf = [0_u8; 16 + 2];
            stream.read_exact(&mut buf).map_err(|err| err.to_string())?;
        }
        atyp => {
            return Err(format!(
                "Unsupported SOCKS5 address type in response: {atyp}"
            ))
        }
    }
    Ok(())
}

fn relay_streams(client: TcpStream, upstream: TcpStream) -> Result<(), String> {
    let mut client_read = client.try_clone().map_err(|err| err.to_string())?;
    let mut client_write = client;
    let mut upstream_read = upstream.try_clone().map_err(|err| err.to_string())?;
    let mut upstream_write = upstream;

    let upload = thread::spawn(move || {
        let _ = io::copy(&mut client_read, &mut upstream_write);
        let _ = upstream_write.shutdown(Shutdown::Write);
    });
    let download = thread::spawn(move || {
        let _ = io::copy(&mut upstream_read, &mut client_write);
        let _ = client_write.shutdown(Shutdown::Write);
    });
    let _ = upload.join();
    let _ = download.join();
    Ok(())
}

fn apply_authenticated_proxy_relay(
    profile: &mut Profile,
    profile_id: &str,
    state: State<'_, AppState>,
) -> Result<Option<u16>, String> {
    if !(matches!(profile.proxy_scheme.as_deref(), Some("socks5"))
        && profile.proxy_username.is_some())
    {
        return Ok(None);
    }

    let secret_ref: Option<String> = {
        let conn = state.db.lock().map_err(|err| err.to_string())?;
        conn.query_row(
            r#"
            SELECT COALESCE(profiles.proxy_password_secret_ref, proxies.password_secret_ref)
            FROM profiles
            LEFT JOIN proxies ON proxies.id = profiles.proxy_id
            WHERE profiles.id = ?1
            "#,
            [profile_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| err.to_string())?
        .flatten()
    };
    let secret_ref = secret_ref.ok_or_else(|| {
        "This profile has a proxy username but no saved Keychain password.".to_string()
    })?;
    let password = read_proxy_password(&secret_ref)?;
    let upstream = UpstreamProxy {
        scheme: "socks5".to_string(),
        host: profile
            .proxy_host
            .clone()
            .ok_or_else(|| "Proxy host is missing.".to_string())?,
        port: profile
            .proxy_port
            .ok_or_else(|| "Proxy port is missing.".to_string())?
            .try_into()
            .map_err(|_| "Proxy port is invalid.".to_string())?,
        username: profile.proxy_username.clone(),
        password: Some(password),
    };
    let relay_port = start_proxy_relay(upstream)?;
    profile.proxy_scheme = Some("http".to_string());
    profile.proxy_host = Some("127.0.0.1".to_string());
    profile.proxy_port = Some(i64::from(relay_port));
    profile.proxy_username = None;
    Ok(Some(relay_port))
}

fn chrome_for_testing_platform() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("mac-arm64"),
        ("macos", "x86_64") => Ok("mac-x64"),
        ("linux", "x86_64") => Ok("linux64"),
        (os, arch) => Err(format!(
            "Managed Chrome is not wired for this platform yet: {os}/{arch}."
        )),
    }
}

fn managed_browser_root(paths: &AppPaths) -> PathBuf {
    paths.browsers_dir.join("chrome-for-testing")
}

fn managed_browser_install_dir(paths: &AppPaths) -> Result<PathBuf, String> {
    Ok(managed_browser_root(paths).join(format!("chrome-{}", chrome_for_testing_platform()?)))
}

fn managed_browser_binary_path(paths: &AppPaths) -> Result<PathBuf, String> {
    let install_dir = managed_browser_install_dir(paths)?;
    match std::env::consts::OS {
        "macos" => Ok(install_dir
            .join("Google Chrome for Testing.app")
            .join("Contents")
            .join("MacOS")
            .join("Google Chrome for Testing")),
        "linux" => Ok(install_dir.join("chrome")),
        os => Err(format!(
            "Managed Chrome is not wired for this OS yet: {os}."
        )),
    }
}

fn managed_browser_status(paths: &AppPaths) -> ManagedBrowserStatus {
    match managed_browser_binary_path(paths) {
        Ok(binary_path) => {
            let installed = binary_path.exists();
            ManagedBrowserStatus {
                installed,
                install_dir: managed_browser_root(paths).to_string_lossy().to_string(),
                binary_path: installed.then(|| binary_path.to_string_lossy().to_string()),
                message: if installed {
                    "Managed Chrome for Testing is installed and will be used by default."
                        .to_string()
                } else {
                    "Managed Chrome for Testing is not installed.".to_string()
                },
            }
        }
        Err(message) => ManagedBrowserStatus {
            installed: false,
            install_dir: managed_browser_root(paths).to_string_lossy().to_string(),
            binary_path: None,
            message,
        },
    }
}

fn chrome_for_testing_download_url(platform: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json",
        ])
        .output()
        .map_err(|err| format!("Failed to run curl: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "Failed to fetch Chrome for Testing metadata: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let value: Value = serde_json::from_slice(&output.stdout).map_err(|err| err.to_string())?;
    value["channels"]["Stable"]["downloads"]["chrome"]
        .as_array()
        .and_then(|downloads| {
            downloads.iter().find_map(|download| {
                (download["platform"].as_str()? == platform)
                    .then(|| download["url"].as_str().map(str::to_string))
                    .flatten()
            })
        })
        .ok_or_else(|| format!("Chrome for Testing download URL not found for {platform}."))
}

fn run_command(mut command: Command, action: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|err| format!("Failed to {action}: {err}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Failed to {action}: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn install_managed_browser_inner(paths: &AppPaths) -> Result<ManagedBrowserStatus, String> {
    let platform = chrome_for_testing_platform()?;
    let url = chrome_for_testing_download_url(platform)?;
    let root = managed_browser_root(paths);
    let extract_dir = root.join("extracting");
    let install_dir = managed_browser_install_dir(paths)?;
    let zip_path = root.join("chrome-for-testing.zip");

    fs::create_dir_all(&root).map_err(|err| err.to_string())?;
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).map_err(|err| err.to_string())?;
    }
    fs::create_dir_all(&extract_dir).map_err(|err| err.to_string())?;
    if zip_path.exists() {
        fs::remove_file(&zip_path).map_err(|err| err.to_string())?;
    }

    let mut curl = Command::new("curl");
    curl.args(["-fL", "--progress-bar", "-o"])
        .arg(&zip_path)
        .arg(&url);
    run_command(curl, "download managed Chrome")?;

    if std::env::consts::OS == "macos" {
        let mut ditto = Command::new("ditto");
        ditto.args(["-x", "-k"]).arg(&zip_path).arg(&extract_dir);
        run_command(ditto, "extract managed Chrome")?;
    } else {
        let mut unzip = Command::new("unzip");
        unzip
            .args(["-q", "-o"])
            .arg(&zip_path)
            .arg("-d")
            .arg(&extract_dir);
        run_command(unzip, "extract managed Chrome")?;
    }

    let extracted_dir = extract_dir.join(format!("chrome-{platform}"));
    if !extracted_dir.exists() {
        return Err(format!(
            "Downloaded Chrome archive did not contain expected folder: {}",
            extracted_dir.display()
        ));
    }
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|err| err.to_string())?;
    }
    fs::rename(&extracted_dir, &install_dir).map_err(|err| err.to_string())?;
    let _ = fs::remove_dir_all(&extract_dir);
    let _ = fs::remove_file(&zip_path);

    let binary_path = managed_browser_binary_path(paths)?;
    if !binary_path.exists() {
        return Err(format!(
            "Managed Chrome installed, but binary was not found: {}",
            binary_path.display()
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&binary_path)
            .map_err(|err| err.to_string())?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&binary_path, permissions).map_err(|err| err.to_string())?;
    }

    Ok(managed_browser_status(paths))
}

fn browser_candidates(paths: Option<&AppPaths>) -> Vec<BrowserCandidate> {
    let mut candidates = Vec::new();
    if let Some(paths) = paths {
        if let Ok(binary_path) = managed_browser_binary_path(paths) {
            let install_dir = managed_browser_root(paths);
            candidates.push(BrowserCandidate {
                name: "FingerBrow Managed Chrome".to_string(),
                app_path: install_dir.to_string_lossy().to_string(),
                binary_path: binary_path.to_string_lossy().to_string(),
                exists: binary_path.exists(),
            });
        }
    }

    candidates.extend(
        [
            (
                "Google Chrome",
                "/Applications/Google Chrome.app",
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            ),
            (
                "Chromium",
                "/Applications/Chromium.app",
                "/Applications/Chromium.app/Contents/MacOS/Chromium",
            ),
        ]
        .into_iter()
        .map(|(name, app_path, binary_path)| BrowserCandidate {
            name: name.to_string(),
            app_path: app_path.to_string(),
            binary_path: binary_path.to_string(),
            exists: Path::new(binary_path).exists(),
        }),
    );

    candidates
}

fn detect_default_browser_binary(paths: &AppPaths) -> Option<String> {
    browser_candidates(Some(paths))
        .into_iter()
        .find(|candidate| candidate.exists)
        .map(|candidate| candidate.binary_path)
}

fn build_launch_args(profile: &Profile) -> Vec<String> {
    let mut args = vec![
        format!("--user-data-dir={}", profile.user_data_dir),
        "--no-first-run".to_string(),
        "--no-default-browser-check".to_string(),
        "--new-window".to_string(),
    ];

    let width = profile.window_width.unwrap_or(1280);
    let height = profile.window_height.unwrap_or(900);
    let x = profile.window_x.unwrap_or(80);
    let y = profile.window_y.unwrap_or(80);
    args.push(format!("--window-size={width},{height}"));
    args.push(format!("--window-position={x},{y}"));

    if let Some(user_agent) = profile.user_agent.as_deref() {
        args.push(format!("--user-agent={user_agent}"));
    }

    if let Some(language) = profile.language.as_deref() {
        args.push(format!("--lang={language}"));
        args.push(format!("--accept-lang={language}"));
    }

    match profile.webrtc_policy.as_str() {
        "public_only" => {
            args.push("--webrtc-ip-handling-policy=default_public_interface_only".to_string());
            args.push("--force-webrtc-ip-handling-policy".to_string());
        }
        "proxy_only" => {
            args.push("--webrtc-ip-handling-policy=disable_non_proxied_udp".to_string());
            args.push("--force-webrtc-ip-handling-policy".to_string());
        }
        _ => {}
    }

    if profile.webrtc_disabled && profile.webrtc_policy == "default" {
        args.push("--webrtc-ip-handling-policy=disable_non_proxied_udp".to_string());
        args.push("--force-webrtc-ip-handling-policy".to_string());
    }

    let custom_has_proxy = profile
        .launch_args
        .iter()
        .any(|arg| arg.trim().starts_with("--proxy-server"));

    if !custom_has_proxy {
        if let (Some(scheme), Some(host), Some(port)) = (
            profile.proxy_scheme.as_deref(),
            profile.proxy_host.as_deref(),
            profile.proxy_port,
        ) {
            args.push(format!("--proxy-server={scheme}://{host}:{port}"));
        }
    }

    args.extend(
        profile
            .launch_args
            .iter()
            .map(|arg| arg.trim())
            .filter(|arg| !arg.is_empty())
            .filter(|arg| !arg.starts_with("--user-data-dir"))
            .map(String::from),
    );

    let startup_urls: Vec<String> = profile
        .startup_urls
        .iter()
        .map(|url| url.trim())
        .filter(|url| !url.is_empty())
        .map(String::from)
        .collect();

    if startup_urls.is_empty() {
        args.push("about:blank".to_string());
    } else {
        args.extend(startup_urls);
    }

    args
}

fn running_profile_processes(user_data_dir: &str) -> Result<Vec<RunningProfileProcess>, String> {
    let needle = format!("--user-data-dir={user_data_dir}");
    let output = Command::new("ps")
        .args(["-axo", "pid=,command="])
        .output()
        .map_err(|err| format!("Failed to inspect running browser processes: {err}"))?;

    if !output.status.success() {
        return Err("Failed to inspect running browser processes.".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let processes = stdout
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let (pid_text, command) = trimmed.split_once(' ')?;
            if !command.contains(&needle) {
                return None;
            }
            let pid = pid_text.parse::<u32>().ok()?;
            Some(RunningProfileProcess {
                pid,
                command: command.to_string(),
            })
        })
        .collect();

    Ok(processes)
}

fn terminate_profile_processes(user_data_dir: &str) -> Result<Vec<u32>, String> {
    let processes = running_profile_processes(user_data_dir)?;
    let pids: Vec<u32> = processes.into_iter().map(|process| process.pid).collect();
    for pid in &pids {
        let status = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .map_err(|err| format!("Failed to terminate browser process {pid}: {err}"))?;
        if !status.success() {
            eprintln!("close_profile: process {pid} did not accept SIGTERM");
        }
    }

    if !pids.is_empty() {
        thread::sleep(Duration::from_millis(800));
    }

    for pid in running_profile_processes(user_data_dir)?
        .into_iter()
        .map(|process| process.pid)
    {
        let status = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status()
            .map_err(|err| format!("Failed to kill browser process {pid}: {err}"))?;
        if !status.success() {
            eprintln!("close_profile: process {pid} did not accept SIGKILL");
        }
    }

    Ok(pids)
}

fn row_to_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<Profile> {
    let tags_json: String = row.get("tags_json")?;
    let launch_args_json: String = row.get("launch_args_json")?;
    let startup_urls_json: String = row.get("startup_urls_json")?;
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;
    let last_launched_at: Option<String> = row.get("last_launched_at")?;
    let proxy_password_secret_ref: Option<String> = row.get("proxy_password_secret_ref")?;
    let webrtc_policy: Option<String> = row.get("webrtc_policy")?;
    let webrtc_disabled = row.get::<_, i64>("webrtc_disabled")? != 0;
    let webrtc_policy = webrtc_policy.unwrap_or_else(|| {
        if webrtc_disabled {
            "proxy_only".to_string()
        } else {
            "default".to_string()
        }
    });

    Ok(Profile {
        id: row.get("id")?,
        name: row.get("name")?,
        notes: row.get("notes")?,
        tags: decode_json_vec(tags_json)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(err.into()))?,
        browser_binary_path: row.get("browser_binary_path")?,
        user_data_dir: row.get("user_data_dir")?,
        proxy_id: row.get("proxy_id")?,
        proxy_scheme: row.get("proxy_scheme")?,
        proxy_host: row.get("proxy_host")?,
        proxy_port: row.get("proxy_port")?,
        proxy_username: row.get("proxy_username")?,
        proxy_password_saved: proxy_password_secret_ref.is_some(),
        user_agent: row.get("user_agent")?,
        language: row.get("language")?,
        timezone: row.get("timezone")?,
        profile_color: row.get("profile_color")?,
        webrtc_disabled: is_webrtc_direct_udp_restricted(&webrtc_policy),
        webrtc_policy,
        window_width: row.get("window_width")?,
        window_height: row.get("window_height")?,
        window_x: row.get("window_x")?,
        window_y: row.get("window_y")?,
        launch_args: decode_json_vec(launch_args_json)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(err.into()))?,
        startup_urls: decode_json_vec(startup_urls_json)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(err.into()))?,
        created_at: parse_datetime(created_at)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(err.into()))?,
        updated_at: parse_datetime(updated_at)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(err.into()))?,
        last_launched_at: parse_optional_datetime(last_launched_at)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(err.into()))?,
        running: false,
    })
}

fn row_to_proxy_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProxyProfile> {
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;
    let password_secret_ref: Option<String> = row.get("password_secret_ref")?;

    Ok(ProxyProfile {
        id: row.get("id")?,
        name: row.get("name")?,
        scheme: row.get("type")?,
        host: row.get("host")?,
        port: row.get("port")?,
        username: row.get("username")?,
        password_saved: password_secret_ref.is_some(),
        notes: row.get("notes")?,
        created_at: parse_datetime(created_at)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(err.into()))?,
        updated_at: parse_datetime(updated_at)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(err.into()))?,
    })
}

fn get_proxy_profile_by_id(
    conn: &Connection,
    proxy_id: &str,
) -> Result<Option<ProxyProfile>, String> {
    conn.query_row(
        r#"
        SELECT id, name, type, host, port, username, password_secret_ref, notes, created_at, updated_at
        FROM proxies
        WHERE id = ?1
        "#,
        [proxy_id],
        row_to_proxy_profile,
    )
    .optional()
    .map_err(|err| err.to_string())
}

fn run_migrations(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS schema_migrations (
          version INTEGER PRIMARY KEY,
          name TEXT NOT NULL,
          applied_at TEXT NOT NULL
        );
        "#,
    )
    .map_err(|err| err.to_string())?;

    let migration_exists: Option<i64> = conn
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = ?1",
            [1],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| err.to_string())?;

    if migration_exists.is_none() {
        let applied_at = now().to_rfc3339();
        conn.execute_batch(
            r#"
            CREATE TABLE profiles (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              notes TEXT NOT NULL DEFAULT '',
              tags_json TEXT NOT NULL DEFAULT '[]',
              browser_binary_path TEXT,
              user_data_dir TEXT NOT NULL,
              proxy_id TEXT,
              launch_args_json TEXT NOT NULL DEFAULT '[]',
              startup_urls_json TEXT NOT NULL DEFAULT '[]',
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              last_launched_at TEXT,
              FOREIGN KEY (proxy_id) REFERENCES proxies(id)
            );

            CREATE TABLE proxies (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              type TEXT NOT NULL,
              host TEXT NOT NULL,
              port INTEGER NOT NULL,
              username TEXT,
              password_secret_ref TEXT,
              notes TEXT NOT NULL DEFAULT '',
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            "#,
        )
        .map_err(|err| err.to_string())?;
        conn.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![1, "initial_profile_and_proxy_schema", applied_at],
        )
        .map_err(|err| err.to_string())?;
    }

    let migration_exists: Option<i64> = conn
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = ?1",
            [2],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| err.to_string())?;

    if migration_exists.is_none() {
        let applied_at = now().to_rfc3339();
        conn.execute_batch(
            r#"
            ALTER TABLE profiles ADD COLUMN proxy_scheme TEXT;
            ALTER TABLE profiles ADD COLUMN proxy_host TEXT;
            ALTER TABLE profiles ADD COLUMN proxy_port INTEGER;
            "#,
        )
        .map_err(|err| err.to_string())?;
        conn.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![2, "profile_proxy_settings", applied_at],
        )
        .map_err(|err| err.to_string())?;
    }

    let migration_exists: Option<i64> = conn
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = ?1",
            [3],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| err.to_string())?;

    if migration_exists.is_none() {
        let applied_at = now().to_rfc3339();
        conn.execute_batch(
            r#"
            ALTER TABLE profiles ADD COLUMN proxy_username TEXT;
            ALTER TABLE profiles ADD COLUMN proxy_password_secret_ref TEXT;
            ALTER TABLE profiles ADD COLUMN user_agent TEXT;
            ALTER TABLE profiles ADD COLUMN language TEXT;
            ALTER TABLE profiles ADD COLUMN webrtc_disabled INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE profiles ADD COLUMN window_width INTEGER;
            ALTER TABLE profiles ADD COLUMN window_height INTEGER;
            ALTER TABLE profiles ADD COLUMN window_x INTEGER;
            ALTER TABLE profiles ADD COLUMN window_y INTEGER;
            "#,
        )
        .map_err(|err| err.to_string())?;
        conn.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![3, "profile_browser_controls_and_keychain_proxy", applied_at],
        )
        .map_err(|err| err.to_string())?;
    }

    let migration_exists: Option<i64> = conn
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = ?1",
            [4],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| err.to_string())?;

    if migration_exists.is_none() {
        let applied_at = now().to_rfc3339();
        conn.execute_batch(
            r#"
            ALTER TABLE profiles ADD COLUMN timezone TEXT;
            ALTER TABLE profiles ADD COLUMN webrtc_policy TEXT NOT NULL DEFAULT 'default';
            UPDATE profiles
            SET webrtc_policy = CASE
              WHEN webrtc_disabled = 1 THEN 'proxy_only'
              ELSE 'default'
            END;
            "#,
        )
        .map_err(|err| err.to_string())?;
        conn.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![4, "profile_timezone_and_webrtc_policy", applied_at],
        )
        .map_err(|err| err.to_string())?;
    }

    let migration_exists: Option<i64> = conn
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = ?1",
            [5],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| err.to_string())?;

    if migration_exists.is_none() {
        let applied_at = now().to_rfc3339();
        conn.execute_batch(
            r#"
            ALTER TABLE profiles ADD COLUMN profile_color TEXT;
            "#,
        )
        .map_err(|err| err.to_string())?;
        let profile_ids = {
            let mut stmt = conn
                .prepare("SELECT id FROM profiles ORDER BY created_at ASC, name ASC")
                .map_err(|err| err.to_string())?;
            let ids = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|err| err.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| err.to_string())?;
            ids
        };
        for (index, id) in profile_ids.iter().enumerate() {
            conn.execute(
                "UPDATE profiles SET profile_color = ?1 WHERE id = ?2",
                params![default_profile_color(index), id],
            )
            .map_err(|err| err.to_string())?;
        }
        conn.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![5, "profile_theme_color", applied_at],
        )
        .map_err(|err| err.to_string())?;
    }

    Ok(())
}

fn init_state(app_data_dir: PathBuf) -> Result<AppState, String> {
    let paths = AppPaths::new(app_data_dir);
    paths.ensure()?;
    let conn = Connection::open(&paths.database_path).map_err(|err| err.to_string())?;
    run_migrations(&conn)?;
    Ok(AppState {
        paths,
        db: Mutex::new(conn),
    })
}

#[tauri::command]
fn get_app_paths(state: State<'_, AppState>) -> AppPaths {
    state.paths.clone()
}

#[tauri::command]
fn detect_browsers(state: State<'_, AppState>) -> Vec<BrowserCandidate> {
    browser_candidates(Some(&state.paths))
}

#[tauri::command]
fn get_managed_browser_status(state: State<'_, AppState>) -> ManagedBrowserStatus {
    managed_browser_status(&state.paths)
}

#[tauri::command]
fn install_managed_browser(state: State<'_, AppState>) -> Result<ManagedBrowserStatus, String> {
    install_managed_browser_inner(&state.paths)
}

#[tauri::command]
fn list_proxy_profiles(state: State<'_, AppState>) -> Result<Vec<ProxyProfile>, String> {
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, name, type, host, port, username, password_secret_ref, notes, created_at, updated_at
            FROM proxies
            ORDER BY updated_at DESC, name ASC
            "#,
        )
        .map_err(|err| err.to_string())?;

    let proxy_profiles = stmt
        .query_map([], row_to_proxy_profile)
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;

    Ok(proxy_profiles)
}

#[tauri::command]
fn create_proxy_profile(
    input: ProxyProfileInput,
    state: State<'_, AppState>,
) -> Result<ProxyProfile, String> {
    let id = Uuid::new_v4().to_string();
    let name = normalize_required_name(&input.name, "Proxy name")?;
    let (scheme, host, port) =
        normalize_required_proxy_settings(input.scheme, input.host, input.port)?;
    let username = normalize_optional_text(input.username);
    let password_secret_ref = match (&username, input.password.as_deref()) {
        (Some(username), Some(password)) if !password.is_empty() => {
            Some(save_proxy_password(&id, username, password)?)
        }
        _ => None,
    };
    let now = now();
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    conn.execute(
        r#"
        INSERT INTO proxies (
          id, name, type, host, port, username, password_secret_ref, notes, created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
        params![
            id,
            name,
            scheme,
            host,
            port,
            username,
            password_secret_ref,
            input.notes.unwrap_or_default(),
            now.to_rfc3339(),
            now.to_rfc3339(),
        ],
    )
    .map_err(|err| err.to_string())?;
    get_proxy_profile_by_id(&conn, &id)?.ok_or_else(|| "Proxy profile not found.".to_string())
}

#[tauri::command]
fn update_proxy_profile(
    id: String,
    input: ProxyProfileInput,
    state: State<'_, AppState>,
) -> Result<ProxyProfile, String> {
    let name = normalize_required_name(&input.name, "Proxy name")?;
    let (scheme, host, port) =
        normalize_required_proxy_settings(input.scheme, input.host, input.port)?;
    let username = normalize_optional_text(input.username);
    let existing_secret_ref: Option<String> = {
        let conn = state.db.lock().map_err(|err| err.to_string())?;
        conn.query_row(
            "SELECT password_secret_ref FROM proxies WHERE id = ?1",
            [&id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| err.to_string())?
        .flatten()
    };
    let password_secret_ref = match (&username, input.password.as_deref()) {
        (Some(username), Some(password)) if !password.is_empty() => {
            Some(save_proxy_password(&id, username, password)?)
        }
        (None, _) => {
            if let Some(secret_ref) = existing_secret_ref {
                delete_proxy_password(&secret_ref)?;
            }
            None
        }
        _ => existing_secret_ref,
    };
    let updated_at = now();
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    let changed = conn
        .execute(
            r#"
            UPDATE proxies
            SET name = ?1,
                type = ?2,
                host = ?3,
                port = ?4,
                username = ?5,
                password_secret_ref = ?6,
                notes = ?7,
                updated_at = ?8
            WHERE id = ?9
            "#,
            params![
                name,
                scheme,
                host,
                port,
                username,
                password_secret_ref,
                input.notes.unwrap_or_default(),
                updated_at.to_rfc3339(),
                id,
            ],
        )
        .map_err(|err| err.to_string())?;
    if changed == 0 {
        return Err("Proxy profile not found.".to_string());
    }
    get_proxy_profile_by_id(&conn, &id)?.ok_or_else(|| "Proxy profile not found.".to_string())
}

#[tauri::command]
fn delete_proxy_profile(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let secret_ref: Option<String> = {
        let conn = state.db.lock().map_err(|err| err.to_string())?;
        conn.query_row(
            "SELECT password_secret_ref FROM proxies WHERE id = ?1",
            [&id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| err.to_string())?
        .flatten()
    };
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    conn.execute(
        r#"
        UPDATE profiles
        SET proxy_id = NULL,
            proxy_scheme = NULL,
            proxy_host = NULL,
            proxy_port = NULL,
            proxy_username = NULL,
            proxy_password_secret_ref = NULL
        WHERE proxy_id = ?1
        "#,
        [&id],
    )
    .map_err(|err| err.to_string())?;
    let changed = conn
        .execute("DELETE FROM proxies WHERE id = ?1", [&id])
        .map_err(|err| err.to_string())?;
    drop(conn);
    if changed == 0 {
        return Err("Proxy profile not found.".to_string());
    }
    if let Some(secret_ref) = secret_ref {
        delete_proxy_password(&secret_ref)?;
    }
    Ok(())
}

#[tauri::command]
fn list_profiles(state: State<'_, AppState>) -> Result<Vec<Profile>, String> {
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, name, notes, tags_json, browser_binary_path, user_data_dir, proxy_id,
                   proxy_scheme, proxy_host, proxy_port, proxy_username,
                   proxy_password_secret_ref, user_agent, language, timezone, profile_color,
                   webrtc_policy, webrtc_disabled,
                   window_width, window_height, window_x, window_y, launch_args_json,
                   startup_urls_json, created_at, updated_at, last_launched_at
            FROM profiles
            ORDER BY updated_at DESC, name ASC
            "#,
        )
        .map_err(|err| err.to_string())?;

    let mut profiles = stmt
        .query_map([], row_to_profile)
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;

    for profile in &mut profiles {
        profile.running = running_profile_processes(&profile.user_data_dir)
            .map(|processes| !processes.is_empty())
            .unwrap_or(false);
    }

    Ok(profiles)
}

#[tauri::command]
fn create_profile(
    input: CreateProfileInput,
    state: State<'_, AppState>,
) -> Result<Profile, String> {
    let name = normalize_profile_name(&input.name)?;
    let id = Uuid::new_v4().to_string();
    let profile_dir = state.paths.profiles_dir.join(format!("profile_{id}"));
    let user_data_dir = profile_dir.join("chrome-user-data");
    fs::create_dir_all(&user_data_dir).map_err(|err| err.to_string())?;

    let notes = input.notes.unwrap_or_default();
    let tags = input.tags.unwrap_or_default();
    let (proxy_scheme, proxy_host, proxy_port) =
        normalize_proxy_settings(input.proxy_scheme, input.proxy_host, input.proxy_port)?;
    let proxy_username = normalize_optional_text(input.proxy_username);
    let (proxy_id, proxy_scheme, proxy_host, proxy_port, proxy_username, proxy_password_secret_ref) =
        if let Some(proxy_id) = normalize_optional_text(input.proxy_id) {
            let conn = state.db.lock().map_err(|err| err.to_string())?;
            let proxy = get_proxy_profile_by_id(&conn, &proxy_id)?
                .ok_or_else(|| "Selected proxy profile was not found.".to_string())?;
            (
                Some(proxy.id),
                Some(proxy.scheme),
                Some(proxy.host),
                Some(proxy.port),
                proxy.username,
                None,
            )
        } else {
            let secret_ref = match (&proxy_username, input.proxy_password.as_deref()) {
                (Some(username), Some(password)) if !password.is_empty() => {
                    Some(save_proxy_password(&id, username, password)?)
                }
                _ => None,
            };
            (
                None,
                proxy_scheme,
                proxy_host,
                proxy_port,
                proxy_username,
                secret_ref,
            )
        };
    let user_agent = normalize_optional_text(input.user_agent);
    let language = normalize_optional_text(input.language);
    let timezone = normalize_optional_text(input.timezone);
    let profile_color = normalize_profile_color(input.profile_color)?;
    let webrtc_policy = normalize_webrtc_policy(input.webrtc_policy, input.webrtc_disabled)?;
    let window_width = normalize_dimension(input.window_width, "Window width")?;
    let window_height = normalize_dimension(input.window_height, "Window height")?;
    let window_x = normalize_coordinate(input.window_x, "Window X")?;
    let window_y = normalize_coordinate(input.window_y, "Window Y")?;
    let launch_args = input.launch_args.unwrap_or_default();
    let startup_urls = input.startup_urls.unwrap_or_default();
    let webrtc_disabled = is_webrtc_direct_udp_restricted(&webrtc_policy);
    let created_at = now();
    let updated_at = created_at;
    let user_data_dir_string = user_data_dir.to_string_lossy().to_string();

    let conn = state.db.lock().map_err(|err| err.to_string())?;
    conn.execute(
        r#"
        INSERT INTO profiles (
          id, name, notes, tags_json, browser_binary_path, user_data_dir, proxy_id,
          proxy_scheme, proxy_host, proxy_port, proxy_username, proxy_password_secret_ref,
          user_agent, language, timezone, profile_color, webrtc_policy, webrtc_disabled, window_width,
          window_height, window_x, window_y, launch_args_json, startup_urls_json, created_at,
          updated_at
        )
        VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
          ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26
        )
        "#,
        params![
            id,
            name,
            notes,
            encode_json(&tags)?,
            input.browser_binary_path,
            user_data_dir_string,
            proxy_id,
            proxy_scheme,
            proxy_host,
            proxy_port,
            proxy_username,
            proxy_password_secret_ref,
            user_agent,
            language,
            timezone,
            profile_color,
            webrtc_policy,
            webrtc_disabled as i64,
            window_width,
            window_height,
            window_x,
            window_y,
            encode_json(&launch_args)?,
            encode_json(&startup_urls)?,
            created_at.to_rfc3339(),
            updated_at.to_rfc3339(),
        ],
    )
    .map_err(|err| err.to_string())?;
    drop(conn);

    get_profile(&id, state)
}

#[tauri::command]
fn update_profile(
    id: String,
    input: UpdateProfileInput,
    state: State<'_, AppState>,
) -> Result<Profile, String> {
    let name = normalize_profile_name(&input.name)?;
    let updated_at = now();
    let tags = input.tags.unwrap_or_default();
    let (proxy_scheme, proxy_host, proxy_port) =
        normalize_proxy_settings(input.proxy_scheme, input.proxy_host, input.proxy_port)?;
    let proxy_username = normalize_optional_text(input.proxy_username);
    let user_agent = normalize_optional_text(input.user_agent);
    let language = normalize_optional_text(input.language);
    let timezone = normalize_optional_text(input.timezone);
    let profile_color = normalize_profile_color(input.profile_color)?;
    let webrtc_policy = normalize_webrtc_policy(input.webrtc_policy, input.webrtc_disabled)?;
    let window_width = normalize_dimension(input.window_width, "Window width")?;
    let window_height = normalize_dimension(input.window_height, "Window height")?;
    let window_x = normalize_coordinate(input.window_x, "Window X")?;
    let window_y = normalize_coordinate(input.window_y, "Window Y")?;
    let webrtc_disabled = is_webrtc_direct_udp_restricted(&webrtc_policy);
    let existing_secret_ref: Option<String> = {
        let conn = state.db.lock().map_err(|err| err.to_string())?;
        conn.query_row(
            "SELECT proxy_password_secret_ref FROM profiles WHERE id = ?1",
            [&id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| err.to_string())?
        .flatten()
    };
    let (proxy_id, proxy_scheme, proxy_host, proxy_port, proxy_username, proxy_password_secret_ref) =
        if let Some(proxy_id) = normalize_optional_text(input.proxy_id) {
            if let Some(secret_ref) = existing_secret_ref {
                delete_proxy_password(&secret_ref)?;
            }
            let conn = state.db.lock().map_err(|err| err.to_string())?;
            let proxy = get_proxy_profile_by_id(&conn, &proxy_id)?
                .ok_or_else(|| "Selected proxy profile was not found.".to_string())?;
            (
                Some(proxy.id),
                Some(proxy.scheme),
                Some(proxy.host),
                Some(proxy.port),
                proxy.username,
                None,
            )
        } else if proxy_scheme.is_none() {
            if let Some(secret_ref) = existing_secret_ref {
                delete_proxy_password(&secret_ref)?;
            }
            (
                None,
                proxy_scheme,
                proxy_host,
                proxy_port,
                proxy_username,
                None,
            )
        } else {
            let secret_ref = match (&proxy_username, input.proxy_password.as_deref()) {
                (Some(username), Some(password)) if !password.is_empty() => {
                    Some(save_proxy_password(&id, username, password)?)
                }
                _ => existing_secret_ref,
            };
            (
                None,
                proxy_scheme,
                proxy_host,
                proxy_port,
                proxy_username,
                secret_ref,
            )
        };
    let launch_args = input.launch_args.unwrap_or_default();
    let startup_urls = input.startup_urls.unwrap_or_default();
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    let changed = conn
        .execute(
            r#"
            UPDATE profiles
            SET name = ?1,
                notes = ?2,
                tags_json = ?3,
                browser_binary_path = ?4,
                proxy_id = ?5,
                proxy_scheme = ?6,
                proxy_host = ?7,
                proxy_port = ?8,
                proxy_username = ?9,
                proxy_password_secret_ref = ?10,
                user_agent = ?11,
                language = ?12,
                timezone = ?13,
                profile_color = ?14,
                webrtc_policy = ?15,
                webrtc_disabled = ?16,
                window_width = ?17,
                window_height = ?18,
                window_x = ?19,
                window_y = ?20,
                launch_args_json = ?21,
                startup_urls_json = ?22,
                updated_at = ?23
            WHERE id = ?24
            "#,
            params![
                name,
                input.notes.unwrap_or_default(),
                encode_json(&tags)?,
                input.browser_binary_path,
                proxy_id,
                proxy_scheme,
                proxy_host,
                proxy_port,
                proxy_username,
                proxy_password_secret_ref,
                user_agent,
                language,
                timezone,
                profile_color,
                webrtc_policy,
                webrtc_disabled as i64,
                window_width,
                window_height,
                window_x,
                window_y,
                encode_json(&launch_args)?,
                encode_json(&startup_urls)?,
                updated_at.to_rfc3339(),
                id,
            ],
        )
        .map_err(|err| err.to_string())?;
    drop(conn);

    if changed == 0 {
        return Err("Profile not found.".to_string());
    }

    get_profile(&id, state)
}

#[tauri::command]
fn delete_profile(
    id: String,
    delete_files: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile = get_profile(&id, state.clone())?;
    let secret_ref: Option<String> = {
        let conn = state.db.lock().map_err(|err| err.to_string())?;
        conn.query_row(
            "SELECT proxy_password_secret_ref FROM profiles WHERE id = ?1",
            [&id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| err.to_string())?
        .flatten()
    };
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    let changed = conn
        .execute("DELETE FROM profiles WHERE id = ?1", [id])
        .map_err(|err| err.to_string())?;
    drop(conn);

    if changed == 0 {
        return Err("Profile not found.".to_string());
    }

    if let Some(secret_ref) = secret_ref {
        delete_proxy_password(&secret_ref)?;
    }

    if delete_files {
        let profile_root = Path::new(&profile.user_data_dir)
            .parent()
            .ok_or_else(|| "Profile directory is invalid.".to_string())?;
        if profile_root.starts_with(&state.paths.profiles_dir) && profile_root.exists() {
            fs::remove_dir_all(profile_root).map_err(|err| err.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
fn launch_profile(id: String, state: State<'_, AppState>) -> Result<LaunchProfileResult, String> {
    let mut profile = get_profile(&id, state.clone())?;
    fs::create_dir_all(&profile.user_data_dir).map_err(|err| err.to_string())?;

    let browser_binary_path = profile
        .browser_binary_path
        .clone()
        .filter(|path| !path.trim().is_empty())
        .or_else(|| detect_default_browser_binary(&state.paths))
        .ok_or_else(|| {
            "No Chrome/Chromium binary found. Set a browser binary path on the profile.".to_string()
        })?;

    if !Path::new(&browser_binary_path).exists() {
        return Err(format!(
            "Browser binary does not exist: {browser_binary_path}"
        ));
    }

    let running_pids: Vec<u32> = running_profile_processes(&profile.user_data_dir)?
        .into_iter()
        .map(|process| process.pid)
        .collect();
    if !running_pids.is_empty() {
        return Err(format!(
            "This profile is already running in Chrome process {}. Close that profile window and launch again so the latest proxy and browser flags can apply.",
            running_pids
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let relay_port = apply_authenticated_proxy_relay(&mut profile, &id, state.clone())?;
    ensure_chrome_profile_preferences(&profile)?;

    let args = build_launch_args(&profile);
    if let Some(relay_port) = relay_port {
        eprintln!("launch_profile: using local proxy relay at 127.0.0.1:{relay_port}");
    }
    eprintln!("launch_profile args: {}", args.join(" "));
    let mut command = Command::new(&browser_binary_path);
    command.args(&args);
    if let Some(timezone) = profile.timezone.as_deref() {
        command.env("TZ", timezone);
        eprintln!("launch_profile timezone: {timezone}");
    }
    command
        .spawn()
        .map_err(|err| format!("Failed to launch browser: {err}"))?;

    let launched_at = now();
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    conn.execute(
        "UPDATE profiles SET last_launched_at = ?1 WHERE id = ?2",
        params![launched_at.to_rfc3339(), id],
    )
    .map_err(|err| err.to_string())?;

    Ok(LaunchProfileResult {
        profile_id: profile.id,
        browser_binary_path,
        args,
        launched_at,
    })
}

#[tauri::command]
fn close_profile(id: String, state: State<'_, AppState>) -> Result<Vec<u32>, String> {
    let profile = get_profile(&id, state)?;
    terminate_profile_processes(&profile.user_data_dir)
}

#[tauri::command]
fn test_profile_proxy(id: String, state: State<'_, AppState>) -> Result<ProxyTestResult, String> {
    let mut profile = get_profile(&id, state.clone())?;
    let relay_port = apply_authenticated_proxy_relay(&mut profile, &id, state)?;
    let (scheme, host, port) = match (
        profile.proxy_scheme.as_deref(),
        profile.proxy_host.as_deref(),
        profile.proxy_port,
    ) {
        (Some(scheme), Some(host), Some(port)) => (scheme, host, port),
        _ => {
            return Ok(ProxyTestResult {
                ok: false,
                message: "No proxy is configured for this profile.".to_string(),
                observed_ip: None,
            });
        }
    };

    if scheme != "http" {
        return Ok(ProxyTestResult {
            ok: false,
            message: "Proxy test currently uses the local HTTP relay path.".to_string(),
            observed_ip: None,
        });
    }

    let mut stream = TcpStream::connect(format!("{host}:{port}"))
        .map_err(|err| format!("Failed to connect to local proxy relay: {err}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|err| err.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|err| err.to_string())?;
    stream
        .write_all(
            b"GET http://api.ipify.org/?format=json HTTP/1.1\r\nHost: api.ipify.org\r\nConnection: close\r\n\r\n",
        )
        .map_err(|err| format!("Failed to send proxy test request: {err}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|err| format!("Failed to read proxy test response: {err}"))?;
    let body = http_response_body(&response);
    let observed_ip = extract_ip_from_body(&body);

    Ok(ProxyTestResult {
        ok: response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200"),
        message: relay_port
            .map(|port| format!("Proxy test used local relay 127.0.0.1:{port}."))
            .unwrap_or_else(|| "Proxy test used configured proxy directly.".to_string()),
        observed_ip,
    })
}

fn get_profile(id: &str, state: State<'_, AppState>) -> Result<Profile, String> {
    let conn = state.db.lock().map_err(|err| err.to_string())?;
    conn.query_row(
        r#"
        SELECT id, name, notes, tags_json, browser_binary_path, user_data_dir, proxy_id,
               proxy_scheme, proxy_host, proxy_port, proxy_username, proxy_password_secret_ref,
               user_agent, language, timezone, profile_color, webrtc_policy, webrtc_disabled, window_width,
               window_height, window_x, window_y, launch_args_json, startup_urls_json, created_at, updated_at,
               last_launched_at
        FROM profiles
        WHERE id = ?1
        "#,
        [id],
        row_to_profile,
    )
    .optional()
    .map_err(|err| err.to_string())?
    .ok_or_else(|| "Profile not found.".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().map_err(|err| err.to_string())?;
            let state = init_state(app_data_dir).map_err(|err| err.to_string())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_paths,
            detect_browsers,
            get_managed_browser_status,
            install_managed_browser,
            list_proxy_profiles,
            create_proxy_profile,
            update_proxy_profile,
            delete_proxy_profile,
            list_profiles,
            create_profile,
            update_profile,
            delete_profile,
            launch_profile,
            close_profile,
            test_profile_proxy
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_profile() -> Profile {
        Profile {
            id: "profile-id".to_string(),
            name: "Test".to_string(),
            notes: String::new(),
            tags: vec![],
            browser_binary_path: None,
            user_data_dir: "/tmp/local-chromium/profile-id/chrome-user-data".to_string(),
            proxy_id: None,
            proxy_scheme: None,
            proxy_host: None,
            proxy_port: None,
            proxy_username: None,
            proxy_password_saved: false,
            user_agent: None,
            language: None,
            timezone: None,
            profile_color: None,
            webrtc_policy: "default".to_string(),
            webrtc_disabled: false,
            window_width: None,
            window_height: None,
            window_x: None,
            window_y: None,
            launch_args: vec![
                "--proxy-server=http://127.0.0.1:7890".to_string(),
                "--user-data-dir=/tmp/ignored".to_string(),
                "--lang=en-US".to_string(),
            ],
            startup_urls: vec!["https://example.com".to_string()],
            created_at: now(),
            updated_at: now(),
            last_launched_at: None,
            running: false,
        }
    }

    #[test]
    fn chrome_argb_color_matches_chrome_user_color_format() {
        assert_eq!(chrome_argb_color(0, 177, 255), -16_731_649);
        assert_eq!(chrome_argb_color(5, 150, 105), -16_411_031);
    }

    #[test]
    fn launch_args_keep_profile_isolation_and_allow_clash_proxy() {
        let args = build_launch_args(&test_profile());

        assert_eq!(
            args[0],
            "--user-data-dir=/tmp/local-chromium/profile-id/chrome-user-data"
        );
        assert!(args.contains(&"--no-first-run".to_string()));
        assert!(args.contains(&"--no-default-browser-check".to_string()));
        assert!(args.contains(&"--proxy-server=http://127.0.0.1:7890".to_string()));
        assert!(args.contains(&"--lang=en-US".to_string()));
        assert!(args.contains(&"https://example.com".to_string()));
        assert!(!args.contains(&"--user-data-dir=/tmp/ignored".to_string()));
    }

    #[test]
    fn launch_args_generate_profile_socks5_proxy() {
        let mut profile = test_profile();
        profile.launch_args = vec![];
        profile.startup_urls = vec![];
        profile.proxy_scheme = Some("socks5".to_string());
        profile.proxy_host = Some("127.0.0.1".to_string());
        profile.proxy_port = Some(7891);

        let args = build_launch_args(&profile);

        assert!(args.contains(&"--proxy-server=socks5://127.0.0.1:7891".to_string()));
        assert!(args.contains(&"--new-window".to_string()));
        assert!(args.contains(&"--window-size=1280,900".to_string()));
        assert!(args.contains(&"--window-position=80,80".to_string()));
        assert!(args.contains(&"about:blank".to_string()));
    }

    #[test]
    fn launch_args_generate_ua_language_and_webrtc_controls() {
        let mut profile = test_profile();
        profile.launch_args = vec![];
        profile.startup_urls = vec![];
        profile.user_agent = Some("Custom UA".to_string());
        profile.language = Some("en-US".to_string());
        profile.timezone = Some("America/Los_Angeles".to_string());
        profile.webrtc_policy = "proxy_only".to_string();
        profile.webrtc_disabled = true;
        profile.window_width = Some(1440);
        profile.window_height = Some(960);
        profile.window_x = Some(20);
        profile.window_y = Some(30);

        let args = build_launch_args(&profile);

        assert!(args.contains(&"--user-agent=Custom UA".to_string()));
        assert!(args.contains(&"--lang=en-US".to_string()));
        assert!(args.contains(&"--accept-lang=en-US".to_string()));
        assert!(args.contains(&"--webrtc-ip-handling-policy=disable_non_proxied_udp".to_string()));
        assert!(args.contains(&"--force-webrtc-ip-handling-policy".to_string()));
        assert!(args.contains(&"--window-size=1440,960".to_string()));
        assert!(args.contains(&"--window-position=20,30".to_string()));
    }

    #[test]
    fn launch_args_generate_public_only_webrtc_policy() {
        let mut profile = test_profile();
        profile.launch_args = vec![];
        profile.startup_urls = vec![];
        profile.webrtc_policy = "public_only".to_string();

        let args = build_launch_args(&profile);

        assert!(
            args.contains(&"--webrtc-ip-handling-policy=default_public_interface_only".to_string())
        );
        assert!(args.contains(&"--force-webrtc-ip-handling-policy".to_string()));
    }

    #[test]
    fn extracts_ip_from_json_body() {
        assert_eq!(
            extract_ip_from_body(r#"{"ip":"192.0.2.10"}"#),
            Some("192.0.2.10".to_string())
        );
    }
}
