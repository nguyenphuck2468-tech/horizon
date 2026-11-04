//! Ely.by OAuth2 authentication.
//!
//! Device Code is the primary flow because it needs only a public client ID.
//! The loopback Authorization Code flow remains available only when an
//! explicitly configured client secret is present.

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

#[derive(Clone, Debug)]
pub struct Profile {
    pub uuid: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64,
    pub profile: Profile,
}

/// Public information shown while waiting for device authorization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_at: u64,
    device_code: String,
}

pub enum DevicePoll {
    Pending,
    SlowDown,
    Done(Tokens),
}

pub fn client_id() -> Option<String> {
    std::env::var("HORIZON_ELY_CLIENT_ID")
        .ok()
        .or_else(|| option_env!("HORIZON_ELY_CLIENT_ID").map(str::to_owned))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
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
            interval: body
                .get("interval")
                .and_then(Value::as_u64)
                .unwrap_or(5)
                .max(1),
            expires_at: now() + body.get("expires_in").and_then(Value::as_u64).unwrap_or(900),
            device_code,
        }),
        _ => Err(error_message(&body, "Ely.by device sign-in is unavailable.")),
    }
}

pub fn wait_for_device_code(
    code: &DeviceCode,
    cancelled: &dyn Fn() -> bool,
) -> Result<Tokens, String> {
    let mut interval = code.interval;
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
            DevicePoll::Pending => {}
            DevicePoll::SlowDown => interval = interval.saturating_add(5),
            DevicePoll::Done(tokens) => return Ok(tokens),
        }
    }
}

fn poll_device(code: &DeviceCode) -> Result<DevicePoll, String> {
    let client_id = client_id().ok_or("This build has no Ely.by public client ID.")?;
    let (status, body) = post_form(
        TOKEN_URL,
        &[
            ("grant_type", &DEVICE_GRANT.to_owned()),
            ("client_id", &client_id),
            ("device_code", &code.device_code),
        ],
    )?;
    match body.get("error").and_then(Value::as_str) {
        Some("authorization_pending") => return Ok(DevicePoll::Pending),
        Some("slow_down") => return Ok(DevicePoll::SlowDown),
        Some("expired_token") => {
            return Err("The Ely.by sign-in code expired. Start again.".into())
        }
        Some("access_denied") | Some("authorization_declined") => {
            return Err("Ely.by sign-in was declined.".into())
        }
        _ => {}
    }
    tokens(status, &body).map(DevicePoll::Done)
}

pub fn refresh(refresh_token: &str) -> Result<Tokens, String> {
    let client_id = client_id().ok_or("This build has no Ely.by public client ID.")?;
    let (status, body) = post_form(
        TOKEN_URL,
        &[
            ("client_id", &client_id),
            ("scope", &SCOPES.to_owned()),
            ("refresh_token", &refresh_token.to_owned()),
            ("grant_type", &"refresh_token".to_owned()),
        ],
    )?;
    tokens(status, &body)
}

fn tokens(status: u16, body: &Value) -> Result<Tokens, String> {
    if status != 200 {
        return Err(error_message(body, "Ely.by refused the sign-in."));
    }
    let access_token = string_field(body, "access_token")
        .ok_or("Ely.by returned no access token.")?;
    let profile = profile(&access_token)?;
    Ok(Tokens {
        access_token,
        refresh_token: string_field(body, "refresh_token"),
        expires_at: now() + body.get("expires_in").and_then(Value::as_u64).unwrap_or(86400),
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
    let body: Value = serde_json::from_str(&response.into_string().unwrap_or_default())
        .unwrap_or(Value::Null);
    if status != 200 {
        return Err(error_message(
            &body,
            "Ely.by could not provide the account profile.",
        ));
    }
    Ok(Profile {
        uuid: string_field(&body, "uuid")
            .ok_or("Ely.by returned no account UUID.")?
            .replace('-', "")
            .to_lowercase(),
        name: string_field(&body, "username").ok_or("Ely.by returned no username.")?,
    })
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
                    let request = read_callback(&mut stream)?;
                    if request.method != "GET" || request.path != "/callback" {
                        respond(&mut stream, "Invalid callback request.");
                        continue;
                    }
                    let query = request.query;
                    if query.get("state").map(String::as_str) != Some(self.state.as_str()) {
                        respond(&mut stream, "The sign-in state did not match. Start again.");
                        return Err("The Ely.by sign-in state did not match. Start again.".into());
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
    tokens(status, &body)
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

fn read_callback(stream: &mut TcpStream) -> Result<CallbackRequest, String> {
    use std::io::Read;
    let mut bytes = [0u8; 4096];
    let n = stream
        .read(&mut bytes)
        .map_err(|e| format!("Could not read the Ely.by callback: {e}"))?;
    if n == bytes.len() && !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
        return Err("The Ely.by callback request was too large.".into());
    }
    let request = String::from_utf8_lossy(&bytes[..n]);
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
    let body = format!("<!doctype html><title>Horizon</title><p>{message}</p>");
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
