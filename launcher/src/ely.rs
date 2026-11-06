//! Ely.by OAuth2 authentication.
//!
//! Device Code is the primary flow because it needs only a public client ID.
//! The loopback Authorization Code flow remains available only when an
//! explicitly configured client secret is present.

use crate::config::{ely_client_id_from_config, resolve_client_id};
use serde_json::Value;
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const AUTHORIZATION_URL: &str = "https://account.ely.by/oauth2/v1";
const DEVICE_CODE_URL: &str = "https://account.ely.by/api/oauth2/v1/devicecode";
const DEVICE_VERIFICATION_URL: &str = "https://account.ely.by/code";
const TOKEN_URL: &str = "https://account.ely.by/api/oauth2/v1/token";
const INFO_URL: &str = "https://account.ely.by/api/account/v1/info";
const DEFAULT_PORT: u16 = 39871;
const SCOPES: &str = "account_info offline_access minecraft_server_session";
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// Public client ID of the Horizon-registered Ely.by application.
/// Replace this value if you register your own application.
const DEFAULT_ELY_CLIENT_ID: &str = "horizon-launcher2";

#[derive(Clone, Debug)]
pub struct Profile {
    pub uuid: String,
    pub name: String,
}

#[derive(Clone)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64,
    pub profile: Profile,
}

impl std::fmt::Debug for Tokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tokens")
            .field("access_token", &"<hidden>")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "<hidden>"),
            )
            .field("expires_at", &self.expires_at)
            .field("profile", &self.profile)
            .finish()
    }
}

/// Public information shown while waiting for device authorization.
#[derive(Clone, PartialEq, Eq)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_at: u64,
    device_code: String,
}

// The device code is the secret half of the pair: it stays out of logs and `{:?}`.
impl std::fmt::Debug for DeviceCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceCode")
            .field("user_code", &self.user_code)
            .field("verification_uri", &self.verification_uri)
            .field("interval", &self.interval)
            .field("expires_at", &self.expires_at)
            .field("device_code", &"<hidden>")
            .finish()
    }
}

pub enum DevicePoll {
    Pending,
    SlowDown,
    /// Ely.by could not be reached (network trouble or a 5xx): worth trying again.
    Offline(String),
    Done(Tokens),
}

/// How many polls in a row may fail to reach Ely.by before the sign-in gives up.
const MAX_OFFLINE_POLLS: u32 = 6;
const MAX_POLL_INTERVAL: u64 = 60;

fn bounded_poll_interval(seconds: u64) -> u64 {
    seconds.clamp(1, MAX_POLL_INTERVAL)
}

fn slowed_poll_interval(seconds: u64) -> u64 {
    seconds.saturating_add(5).min(MAX_POLL_INTERVAL)
}

fn backoff_poll_interval(seconds: u64) -> u64 {
    seconds.saturating_mul(2).min(MAX_POLL_INTERVAL)
}

fn choose_client_id(config: Option<&str>, runtime: Option<&str>, built_in: &str) -> Option<String> {
    resolve_client_id(config, runtime, built_in)
}

pub fn client_id() -> Option<String> {
    choose_client_id(
        ely_client_id_from_config().as_deref(),
        std::env::var("HORIZON_ELY_CLIENT_ID").ok().as_deref(),
        DEFAULT_ELY_CLIENT_ID,
    )
}

fn client_secret() -> Option<String> {
    std::env::var("HORIZON_ELY_CLIENT_SECRET")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub fn configured() -> bool {
    client_id().is_some()
}

pub fn callback_configured() -> bool {
    client_id().is_some() && client_secret().is_some()
}

pub fn device_code() -> Result<DeviceCode, String> {
    let client_id = client_id().ok_or("This build has no Ely.by public client ID.")?;
    let (status, body) = post_form(
        DEVICE_CODE_URL,
        &[("client_id", &client_id), ("scope", &SCOPES.to_owned())],
    )?;
    match (
        status,
        string_field(&body, "user_code"),
        string_field(&body, "device_code"),
    ) {
        (200, Some(user_code), Some(device_code)) => Ok(DeviceCode {
            user_code,
            verification_uri: string_field(&body, "verification_uri")
                .unwrap_or_else(|| DEVICE_VERIFICATION_URL.to_owned()),
            interval: bounded_poll_interval(
                body.get("interval").and_then(Value::as_u64).unwrap_or(5),
            ),
            expires_at: now().saturating_add(
                body.get("expires_in")
                    .and_then(Value::as_u64)
                    .unwrap_or(900),
            ),
            device_code,
        }),
        _ => Err(error_message(
            &body,
            "Ely.by device sign-in is unavailable.",
        )),
    }
}

pub fn wait_for_device_code(
    code: &DeviceCode,
    cancelled: &dyn Fn() -> bool,
) -> Result<Tokens, String> {
    let mut interval = code.interval;
    let mut offline = 0u32;
    loop {
        let until = Instant::now() + Duration::from_secs(interval);
        while Instant::now() < until {
            if cancelled() {
                return Err("Sign-in cancelled.".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if cancelled() {
            return Err("Sign-in cancelled.".into());
        }
        if now() >= code.expires_at {
            return Err("The Ely.by sign-in code expired. Start again.".into());
        }
        match poll_device(code)? {
            DevicePoll::Pending => offline = 0,
            DevicePoll::SlowDown => {
                offline = 0;
                interval = slowed_poll_interval(interval);
            }
            // RFC 8628 recommends backing off after a timeout. Treat server/network outages
            // conservatively too, while keeping the code alive for a few retries.
            DevicePoll::Offline(error) => {
                offline += 1;
                interval = backoff_poll_interval(interval);
                if offline >= MAX_OFFLINE_POLLS {
                    return Err(error);
                }
            }
            DevicePoll::Done(tokens) => return Ok(tokens),
        }
    }
}

fn poll_device(code: &DeviceCode) -> Result<DevicePoll, String> {
    let client_id = client_id().ok_or("This build has no Ely.by public client ID.")?;
    let (status, body) = match post_form(
        TOKEN_URL,
        &[
            ("grant_type", &DEVICE_GRANT.to_owned()),
            ("client_id", &client_id),
            ("device_code", &code.device_code),
        ],
    ) {
        Ok(answer) => answer,
        Err(error) => return Ok(DevicePoll::Offline(error)),
    };
    // Server trouble or rate limiting: the code is still good, keep polling.
    if status >= 500 || status == 429 || status == 408 {
        return Ok(DevicePoll::Offline(error_message(
            &body,
            "Ely.by is unavailable right now.",
        )));
    }
    match body.get("error").and_then(Value::as_str) {
        Some("authorization_pending") => return Ok(DevicePoll::Pending),
        Some("slow_down") => return Ok(DevicePoll::SlowDown),
        Some("expired_token") => return Err("The Ely.by sign-in code expired. Start again.".into()),
        Some("access_denied") | Some("authorization_declined") => {
            return Err("Ely.by sign-in was declined.".into())
        }
        _ => {}
    }
    initial_tokens(status, &body).map(DevicePoll::Done)
}

/// Why a refresh failed: Ely.by refused the token (sign in again), or could not be asked
/// (network or server trouble: the sign-in is still good, try later).
#[derive(Clone, PartialEq, Eq)]
pub enum RefreshError {
    Rejected(String),
    Unavailable(String),
    /// The provider rotated the refresh token, but profile lookup failed afterwards.
    /// Keep the replacement so a transient profile outage cannot strand the account.
    UnavailableWithRefresh(String, Option<String>),
}

impl std::fmt::Debug for RefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rejected(message) => f.debug_tuple("Rejected").field(message).finish(),
            Self::Unavailable(message) => f.debug_tuple("Unavailable").field(message).finish(),
            Self::UnavailableWithRefresh(message, refresh) => f
                .debug_tuple("UnavailableWithRefresh")
                .field(message)
                .field(&refresh.as_ref().map(|_| "<hidden>"))
                .finish(),
        }
    }
}

pub fn refresh(refresh_token: &str) -> Result<Tokens, RefreshError> {
    let client_id = client_id().ok_or_else(|| {
        RefreshError::Unavailable("This build has no Ely.by public client ID.".into())
    })?;
    let (status, body) = post_form(
        TOKEN_URL,
        &[
            ("client_id", &client_id),
            ("scope", &SCOPES.to_owned()),
            ("refresh_token", &refresh_token.to_owned()),
            ("grant_type", &"refresh_token".to_owned()),
        ],
    )
    .map_err(RefreshError::Unavailable)?;
    if status != 200 {
        return Err(refresh_failure(
            status,
            error_message(&body, "Ely.by refused the sign-in."),
        ));
    }
    let rotated_refresh = string_field(&body, "refresh_token");
    tokens(status, &body)
        .map_err(|error| RefreshError::UnavailableWithRefresh(error, rotated_refresh))
}

/// A refresh that did not give tokens: only a real refusal (4xx other than "too many requests"
/// or "timed out") means the sign-in is gone. A 5xx, a 429 or a 408 is Ely.by being busy.
fn refresh_failure(status: u16, message: String) -> RefreshError {
    if status >= 500 || status == 429 || status == 408 {
        RefreshError::Unavailable(message)
    } else {
        RefreshError::Rejected(message)
    }
}

fn tokens(status: u16, body: &Value) -> Result<Tokens, String> {
    if status != 200 {
        return Err(error_message(body, "Ely.by refused the sign-in."));
    }
    let access_token =
        string_field(body, "access_token").ok_or("Ely.by returned no access token.")?;
    let profile = profile(&access_token)?;
    Ok(Tokens {
        access_token,
        refresh_token: string_field(body, "refresh_token"),
        expires_at: now().saturating_add(
            body.get("expires_in")
                .and_then(Value::as_u64)
                .unwrap_or(86400),
        ),
        profile,
    })
}

fn profile(token: &str) -> Result<Profile, String> {
    let response = crate::proxy::agent(
        ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(20))
            .user_agent(&format!("horizon-launcher/{}", crate::backend::VERSION)),
        INFO_URL,
    )
    .build()
    .get(INFO_URL)
    .set("Authorization", &format!("Bearer {token}"))
    .call();
    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(_, response)) => response,
        Err(ureq::Error::Transport(_)) => {
            return Err("Could not reach Ely.by. Check your internet connection.".into())
        }
    };
    let status = response.status();
    let body: Value =
        serde_json::from_str(&response.into_string().unwrap_or_default()).unwrap_or(Value::Null);
    if status != 200 {
        return Err(error_message(
            &body,
            "Ely.by could not provide the account profile.",
        ));
    }
    parse_profile(&body)
}

fn parse_profile(body: &Value) -> Result<Profile, String> {
    let uuid = string_field(body, "uuid")
        .ok_or("Ely.by returned no account UUID.")?
        .replace('-', "")
        .to_lowercase();
    if uuid.len() != 32 || !uuid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Ely.by returned an invalid account UUID.".into());
    }
    let name = string_field(body, "username").ok_or("Ely.by returned no username.")?;
    if !(3..=16).contains(&name.chars().count())
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err("Ely.by returned an invalid Minecraft profile name.".into());
    }
    Ok(Profile { uuid, name })
}

/// A loopback callback for installations that explicitly configure a
/// confidential Ely.by application. Device Code never calls this function.
pub struct Callback {
    listener: TcpListener,
    redirect_uri: String,
    state: String,
}

pub fn begin() -> Result<(String, Callback), String> {
    let client_id = client_id().ok_or("This build has no Ely.by public client ID.")?;
    client_secret().ok_or(
        "Authorization Code fallback is not configured. Use Ely.by Device Code sign-in instead.",
    )?;
    let listener = TcpListener::bind(("127.0.0.1", callback_port()))
        .map_err(|_| "Could not open the Ely.by localhost callback.".to_string())?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Could not prepare the Ely.by callback: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Could not read the Ely.by callback address: {e}"))?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let state = random_state()?;
    let url = format!(
        "{AUTHORIZATION_URL}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
        encode(&client_id),
        encode(&redirect_uri),
        encode(SCOPES),
        encode(&state)
    );
    Ok((
        url,
        Callback {
            listener,
            redirect_uri,
            state,
        },
    ))
}

impl Callback {
    pub fn wait(self, cancelled: &dyn Fn() -> bool) -> Result<Tokens, String> {
        let deadline = Instant::now() + Duration::from_secs(600);
        loop {
            if cancelled() {
                return Err("Sign-in cancelled.".into());
            }
            if Instant::now() >= deadline {
                return Err("The Ely.by sign-in expired. Start again.".into());
            }
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    // The accepted socket may inherit the listener's non-blocking mode.
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                    let Ok(request) = read_callback(&mut stream) else {
                        // A stray or slow local connection must not end the sign-in.
                        continue;
                    };
                    if request.method != "GET" || request.path != "/callback" {
                        respond(&mut stream, "Invalid callback request.");
                        continue;
                    }
                    let query = request.query;
                    if query.get("state").map(String::as_str) != Some(self.state.as_str()) {
                        // Not our request (another local program, a stale tab): keep waiting.
                        respond(&mut stream, "The sign-in state did not match.");
                        continue;
                    }
                    let code = match query.get("code") {
                        Some(code) if !code.is_empty() => code,
                        _ => {
                            respond(&mut stream, "Ely.by did not return an authorization code.");
                            return Err(query_error(&query));
                        }
                    };
                    respond(
                        &mut stream,
                        "Ely.by sign-in completed. You can return to Horizon.",
                    );
                    return exchange(&self.redirect_uri, code);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(format!("The Ely.by callback failed: {e}")),
            }
        }
    }
}

fn exchange(redirect_uri: &str, code: &str) -> Result<Tokens, String> {
    let client_id = client_id().ok_or("This build has no Ely.by public client ID.")?;
    let secret = client_secret().ok_or(
        "Authorization Code fallback is not configured. Use Ely.by Device Code sign-in instead.",
    )?;
    let (status, body) = post_form(
        TOKEN_URL,
        &[
            ("client_id", &client_id),
            ("client_secret", &secret),
            ("redirect_uri", &redirect_uri.to_owned()),
            ("grant_type", &"authorization_code".to_owned()),
            ("code", &code.to_owned()),
        ],
    )?;
    initial_tokens(status, &body)
}

/// Initial authorization must grant a refresh token: without offline_access the account would
/// work only until the first access-token expiry and then silently become unusable.
fn initial_tokens(status: u16, body: &Value) -> Result<Tokens, String> {
    if status != 200 {
        return Err(error_message(body, "Ely.by refused the sign-in."));
    }
    if !string_field(body, "refresh_token").is_some_and(|token| !token.is_empty()) {
        return Err(
            "Ely.by did not return a refresh token. Check that offline_access is enabled.".into(),
        );
    }
    tokens(status, body)
}

fn post_form(url: &str, fields: &[(&str, &String)]) -> Result<(u16, Value), String> {
    let form = fields
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    let response = crate::proxy::agent(
        ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(20))
            .user_agent(&format!("horizon-launcher/{}", crate::backend::VERSION)),
        url,
    )
    .build()
    .post(url)
    .set("Content-Type", "application/x-www-form-urlencoded")
    .send_string(&form);
    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(_, response)) => response,
        Err(ureq::Error::Transport(_)) => {
            return Err("Could not reach Ely.by. Check your internet connection.".into())
        }
    };
    let status = response.status();
    let body =
        serde_json::from_str(&response.into_string().unwrap_or_default()).unwrap_or(Value::Null);
    Ok((status, body))
}

struct CallbackRequest {
    method: String,
    path: String,
    query: HashMap<String, String>,
}

/// The longest request line and headers accepted from the loopback callback.
const MAX_CALLBACK_REQUEST: usize = 8192;

fn read_callback(stream: &mut TcpStream) -> Result<CallbackRequest, String> {
    use std::io::Read;
    // A request can arrive in several pieces: read until the end of the headers.
    let mut bytes = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
        if bytes.len() >= MAX_CALLBACK_REQUEST {
            return Err("The Ely.by callback request was too large.".into());
        }
        let n = stream
            .read(&mut chunk)
            .map_err(|e| format!("Could not read the Ely.by callback: {e}"))?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
    let request = String::from_utf8_lossy(&bytes);
    let line = request
        .lines()
        .next()
        .ok_or("Ely.by sent an invalid callback.")?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let target = parts.next().ok_or("Ely.by sent an invalid callback.")?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let query = query
        .split('&')
        .filter(|part| !part.is_empty())
        .filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            Some((decode(key)?, decode(value)?))
        })
        .collect();
    Ok(CallbackRequest {
        method,
        path: path.to_owned(),
        query,
    })
}

fn respond(stream: &mut TcpStream, message: &str) {
    use std::io::Write;
    // Callback error descriptions originate outside the launcher. Render them as text,
    // never as HTML, even though the callback is bound to loopback.
    let message = message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;");
    let body = format!("<!doctype html><title>Horizon</title><p>{message}</p>");
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; style-src 'none'\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
}

fn query_error(query: &HashMap<String, String>) -> String {
    query
        .get("error_message")
        .or_else(|| query.get("error_description"))
        .or_else(|| query.get("error"))
        .cloned()
        .unwrap_or_else(|| "Ely.by did not return an authorization code.".into())
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn error_message(value: &Value, fallback: &str) -> String {
    value
        .get("error_description")
        .or_else(|| value.get("errorMessage"))
        .or_else(|| value.get("error"))
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_owned()
}

fn callback_port() -> u16 {
    std::env::var("HORIZON_ELY_CALLBACK_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|port| *port != 0)
        .unwrap_or(DEFAULT_PORT)
}

fn random_state() -> Result<String, String> {
    let mut random = [0u8; 24];
    getrandom::fill(&mut random).map_err(|e| e.to_string())?;
    Ok(random.iter().map(|b| format!("{b:02x}")).collect())
}

fn encode(text: &str) -> String {
    text.bytes()
        .flat_map(|byte| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![char::from(byte)]
            }
            byte => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn decode(text: &str) -> Option<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn empty_runtime_client_id_falls_back_to_the_embedded_public_id() {
        assert_eq!(
            choose_client_id(Some("  "), Some(" embedded-id "), "built-in-id"),
            Some("embedded-id".into())
        );
        assert_eq!(
            choose_client_id(Some(" runtime-id "), Some("embedded-id"), "built-in-id"),
            Some("runtime-id".into())
        );
        assert_eq!(
            choose_client_id(None, None, "embedded-id"),
            Some("embedded-id".into())
        );
        // An empty built-in id (no embedded default) yields nothing to use.
        assert_eq!(choose_client_id(Some("  "), None, ""), None);
    }

    #[test]
    fn device_poll_interval_is_bounded_and_backed_off() {
        assert_eq!(bounded_poll_interval(0), 1);
        assert_eq!(bounded_poll_interval(5), 5);
        assert_eq!(bounded_poll_interval(u64::MAX), 60);
        assert_eq!(slowed_poll_interval(5), 10);
        assert_eq!(slowed_poll_interval(60), 60);
        assert_eq!(backoff_poll_interval(5), 10);
        assert_eq!(backoff_poll_interval(40), 60);
        assert_eq!(backoff_poll_interval(60), 60);
    }

    #[test]
    fn initial_authorization_requires_a_refresh_token() {
        let body = serde_json::json!({"access_token": "access-only"});
        assert_eq!(
            initial_tokens(200, &body).unwrap_err(),
            "Ely.by did not return a refresh token. Check that offline_access is enabled."
        );
        assert!(initial_tokens(401, &body).is_err());
    }

    #[test]
    fn profile_parser_rejects_invalid_minecraft_identity_fields() {
        let valid = serde_json::json!({
            "uuid": "11111111-1111-1111-1111-111111111111",
            "username": "Valid_Player"
        });
        let parsed = parse_profile(&valid).unwrap();
        assert_eq!(parsed.uuid, "11111111111111111111111111111111");
        assert_eq!(parsed.name, "Valid_Player");

        let bad_uuid = serde_json::json!({"uuid": "not-a-uuid", "username": "Valid_Player"});
        assert!(parse_profile(&bad_uuid).is_err());
        let bad_name = serde_json::json!({
            "uuid": "11111111111111111111111111111111",
            "username": "name with spaces"
        });
        assert!(parse_profile(&bad_name).is_err());
        let too_long = serde_json::json!({
            "uuid": "11111111111111111111111111111111",
            "username": "ThisNameIsMuchTooLong"
        });
        assert!(parse_profile(&too_long).is_err());
    }

    #[test]
    fn callback_response_escapes_untrusted_error_text() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let client = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            response
        });
        let (mut stream, _) = listener.accept().unwrap();
        respond(&mut stream, "<script>alert(1)</script> & \"quoted\"");
        drop(stream);
        let response = client.join().unwrap();
        assert!(response.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(response.contains("&amp; &quot;quoted&quot;"));
        assert!(!response.contains("<script>"));
        assert!(response.contains("Content-Security-Policy: default-src 'none'"));
    }

    #[test]
    fn encode_and_decode_round_trip() {
        let text = "account_info offline_access/ü &=+%";
        assert_eq!(decode(&encode(text)).as_deref(), Some(text));
        assert_eq!(encode("a b"), "a%20b");
        assert_eq!(decode("a+b%21").as_deref(), Some("a b!"));
        assert_eq!(decode("%zz"), None);
        assert_eq!(decode("%4"), None);
    }

    #[test]
    fn debug_output_redacts_access_and_refresh_tokens() {
        let tokens = Tokens {
            access_token: "access-secret-fixture".into(),
            refresh_token: Some("refresh-secret-fixture".into()),
            expires_at: 1,
            profile: Profile {
                uuid: "11111111111111111111111111111111".into(),
                name: "ElyPlayer".into(),
            },
        };
        let debug = format!("{tokens:?}");
        assert!(!debug.contains("access-secret-fixture"));
        assert!(!debug.contains("refresh-secret-fixture"));
        assert!(debug.contains("<hidden>"));
    }

    #[test]
    fn error_message_prefers_the_description() {
        let v: Value =
            serde_json::json!({"error": "invalid_grant", "error_description": "Expired"});
        assert_eq!(error_message(&v, "x"), "Expired");
        assert_eq!(error_message(&Value::Null, "fallback"), "fallback");
    }

    #[test]
    fn the_callback_request_is_read() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let client = std::thread::spawn(move || {
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            s.write_all(b"GET /callback?code=abc%20d&state=s1 HTTP/1.1\r\nHost: x\r\n\r\n")
                .unwrap();
            let mut out = String::new();
            let _ = s.read_to_string(&mut out);
        });
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_callback(&mut stream).unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/callback");
        assert_eq!(request.query.get("code").map(String::as_str), Some("abc d"));
        assert_eq!(request.query.get("state").map(String::as_str), Some("s1"));
        respond(&mut stream, "ok");
        drop(stream);
        client.join().unwrap();
    }

    #[test]
    fn refresh_error_redacts_a_rotated_refresh_token() {
        let error = RefreshError::UnavailableWithRefresh(
            "profile lookup timed out".into(),
            Some("rotated-refresh-secret".into()),
        );
        let debug = format!("{error:?}");
        assert!(debug.contains("profile lookup timed out"));
        assert!(debug.contains("<hidden>"));
        assert!(!debug.contains("rotated-refresh-secret"));
        assert!(matches!(
            error,
            RefreshError::UnavailableWithRefresh(message, Some(token))
                if message == "profile lookup timed out" && token == "rotated-refresh-secret"
        ));
    }

    #[test]
    fn only_a_refusal_ends_the_sign_in() {
        for status in [500, 502, 503, 429, 408] {
            assert!(matches!(
                refresh_failure(status, "x".into()),
                RefreshError::Unavailable(_)
            ));
        }
        for status in [400, 401, 403] {
            assert!(matches!(
                refresh_failure(status, "x".into()),
                RefreshError::Rejected(_)
            ));
        }
    }

    #[test]
    fn a_callback_request_in_pieces_is_read() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let client = std::thread::spawn(move || {
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            s.set_nodelay(true).unwrap();
            s.write_all(b"GET /callback?code=abc").unwrap();
            std::thread::sleep(Duration::from_millis(150));
            s.write_all(b"&state=s1 HTTP/1.1\r\nHost: x\r\n\r\n")
                .unwrap();
            let mut out = String::new();
            let _ = s.read_to_string(&mut out);
        });
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_callback(&mut stream).unwrap();
        assert_eq!(request.query.get("code").map(String::as_str), Some("abc"));
        assert_eq!(request.query.get("state").map(String::as_str), Some("s1"));
        respond(&mut stream, "ok");
        drop(stream);
        client.join().unwrap();
    }

    #[test]
    fn an_oversized_callback_is_refused() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let client = std::thread::spawn(move || {
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            let _ = s.write_all(&vec![b'a'; MAX_CALLBACK_REQUEST + 2048]);
        });
        let (mut stream, _) = listener.accept().unwrap();
        assert!(read_callback(&mut stream).is_err());
        client.join().unwrap();
    }

    #[test]
    fn the_device_code_is_not_printed() {
        let code = DeviceCode {
            user_code: "ABCD".into(),
            verification_uri: "https://x".into(),
            interval: 5,
            expires_at: 1,
            device_code: "SECRET-DEVICE-CODE".into(),
        };
        let shown = format!("{code:?}");
        assert!(shown.contains("ABCD") && !shown.contains("SECRET-DEVICE-CODE"));
    }
}
