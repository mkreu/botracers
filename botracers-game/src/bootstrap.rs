#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use base64::Engine;
use bevy::prelude::*;
use botracers_protocol::{
    ArtifactSummary, ServerCapabilities, UpdateArtifactVisibilityRequest, UploadArtifactRequest,
    UploadArtifactResponse,
};
#[cfg(not(target_arch = "wasm32"))]
use botracers_protocol::{LoginRequest, LoginResponse};
#[cfg(not(target_arch = "wasm32"))]
use botracers_server::{AuthMode, ServerConfig};

use crate::game_api::{DriverType, SpawnCarRequest, SpawnResolvedCarRequest, WebApiCommand};
use crate::race_runtime::SimState;

pub struct BootstrapPlugin;

impl Plugin for BootstrapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BootstrapConfig>()
            .init_resource::<WebPortalState>()
            .init_resource::<WebApiQueue>()
            .init_resource::<ArtifactFetchPipeline>()
            .add_systems(
                Startup,
                (initialize_bootstrap, trigger_initial_capability_check).chain(),
            )
            .add_systems(
                Update,
                (
                    handle_web_api_commands,
                    process_web_api_events,
                    handle_spawn_car_request,
                    process_artifact_fetch_results,
                ),
            );
    }
}

#[derive(Resource, Clone, Default)]
pub struct BootstrapConfig {
    pub standalone_mode: bool,
    pub standalone_bind: Option<String>,
}

pub struct CompileResult {
    pub id: u64,
    pub binary: String,
    pub result: Result<Vec<u8>, String>,
}

#[derive(Resource)]
pub struct ArtifactFetchPipeline {
    pub async_results: Arc<Mutex<Vec<CompileResult>>>,
    pub pending: HashMap<u64, DriverType>,
    pub next_request_id: u64,
}

impl Default for ArtifactFetchPipeline {
    fn default() -> Self {
        Self {
            async_results: Arc::new(Mutex::new(Vec::<CompileResult>::new())),
            pending: HashMap::new(),
            next_request_id: 1,
        }
    }
}

#[derive(Debug, Clone)]
enum WebApiEvent {
    Capabilities(Result<ServerCapabilities, String>),
    #[cfg(not(target_arch = "wasm32"))]
    Login(Result<LoginResponse, String>),
    Artifacts(Result<Vec<ArtifactSummary>, String>),
    UploadResult(Result<UploadArtifactResponse, String>),
    DeleteResult {
        artifact_id: i64,
        result: Result<(), String>,
    },
    VisibilityResult {
        artifact_id: i64,
        is_public: bool,
        result: Result<(), String>,
    },
}

#[derive(Resource, Clone)]
pub struct WebApiQueue {
    events: Arc<Mutex<Vec<WebApiEvent>>>,
}

impl Default for WebApiQueue {
    fn default() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[derive(Resource)]
pub struct WebPortalState {
    pub server_url: String,
    pub standalone_mode: bool,
    pub auth_required: Option<bool>,
    #[cfg(not(target_arch = "wasm32"))]
    pub token: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub cli_credentials: Option<(String, String)>,
    pub artifacts: Vec<ArtifactSummary>,
    pub status_message: Option<String>,
}

impl Default for WebPortalState {
    fn default() -> Self {
        Self {
            server_url: {
                #[cfg(target_arch = "wasm32")]
                {
                    String::new()
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    std::env::var("BOTRACERS_URL")
                        .unwrap_or_else(|_| "http://127.0.0.1:8787".to_string())
                }
            },
            standalone_mode: false,
            auth_required: None,
            #[cfg(not(target_arch = "wasm32"))]
            token: None,
            #[cfg(not(target_arch = "wasm32"))]
            cli_credentials: None,
            artifacts: Vec::new(),
            status_message: None,
        }
    }
}

fn initialize_bootstrap(config: Res<BootstrapConfig>, mut web_state: ResMut<WebPortalState>) {
    #[cfg(not(target_arch = "wasm32"))]
    if config.standalone_mode {
        let bind = config
            .standalone_bind
            .clone()
            .unwrap_or_else(|| "127.0.0.1:8787".to_string());
        spawn_embedded_botracers(bind.clone());
        web_state.server_url = format!("http://{bind}");
        web_state.standalone_mode = true;
        web_state.status_message = Some("Standalone mode: auth disabled".to_string());
        return;
    }

    #[cfg(not(target_arch = "wasm32"))]
    match prompt_cli_credentials() {
        Ok(Some((username, password))) => {
            web_state.cli_credentials = Some((username.clone(), password));
            web_state.status_message = Some(format!("Using CLI credentials for '{username}'"));
        }
        Ok(None) => {
            web_state.status_message =
                Some("No CLI credentials provided; remote auth may fail".to_string());
        }
        Err(error) => {
            web_state.status_message = Some(format!("CLI login prompt failed: {error}"));
        }
    }
}

fn trigger_initial_capability_check(mut cmds: MessageWriter<WebApiCommand>) {
    cmds.write(WebApiCommand::RefreshCapabilities);
}

#[cfg(not(target_arch = "wasm32"))]
fn prompt_cli_credentials() -> Result<Option<(String, String)>, String> {
    use std::io::{self, Write};

    print!("BotRacers username (leave empty to skip): ");
    io::stdout()
        .flush()
        .map_err(|e| format!("stdout flush failed: {e}"))?;
    let mut username = String::new();
    io::stdin()
        .read_line(&mut username)
        .map_err(|e| format!("failed to read username: {e}"))?;
    let username = username.trim().to_string();
    if username.is_empty() {
        return Ok(None);
    }

    print!("BotRacers password: ");
    io::stdout()
        .flush()
        .map_err(|e| format!("stdout flush failed: {e}"))?;
    let mut password = String::new();
    io::stdin()
        .read_line(&mut password)
        .map_err(|e| format!("failed to read password: {e}"))?;
    let password = password.trim_end().to_string();
    if password.is_empty() {
        return Ok(None);
    }

    Ok(Some((username, password)))
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_embedded_botracers(bind: String) {
    let mut config = ServerConfig::default();
    config.bind = bind;
    config.auth_mode = AuthMode::Disabled;
    config.db_path = PathBuf::from(
        std::env::var("BOTRACERS_DB_PATH").unwrap_or_else(|_| "botracers.db".to_string()),
    );
    config.artifacts_dir = PathBuf::from(
        std::env::var("BOTRACERS_ARTIFACTS_DIR")
            .unwrap_or_else(|_| "botracers_artifacts".to_string()),
    );
    config.static_dir = None;

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new()
            .expect("failed to create tokio runtime for embedded botracers");
        runtime
            .block_on(botracers_server::run_server(config))
            .expect("embedded botracers crashed");
    });
}

fn web_api_url(base: &str, path: &str) -> String {
    if base.trim().is_empty() {
        path.to_string()
    } else {
        format!("{}{}", base.trim_end_matches('/'), path)
    }
}

fn web_request_with_auth(url: String, _token: Option<&str>) -> ehttp::Request {
    let mut req = ehttp::Request::get(url);
    #[cfg(not(target_arch = "wasm32"))]
    let token = _token;
    #[cfg(target_arch = "wasm32")]
    let token: Option<&str> = None;
    if let Some(token) = token {
        req.headers
            .insert("Authorization", format!("Bearer {token}"));
    }
    req
}

fn push_web_event(queue: &Arc<Mutex<Vec<WebApiEvent>>>, event: WebApiEvent) {
    if let Ok(mut events) = queue.lock() {
        events.push(event);
    }
}

fn response_error(resp: &ehttp::Response) -> String {
    let body = String::from_utf8_lossy(&resp.bytes);
    format!("HTTP {} {}: {}", resp.status, resp.status_text, body.trim())
}

#[cfg(not(target_arch = "wasm32"))]
fn web_fetch_login(
    server_url: &str,
    username: &str,
    password: &str,
    queue: Arc<Mutex<Vec<WebApiEvent>>>,
) {
    let url = web_api_url(server_url, "/api/v1/auth/login");
    let request = match ehttp::Request::json(
        url,
        &LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        },
    ) {
        Ok(req) => req,
        Err(err) => {
            push_web_event(
                &queue,
                WebApiEvent::Login(Err(format!("failed to serialize login request: {err}"))),
            );
            return;
        }
    };

    ehttp::fetch(request, move |result| {
        let event = match result {
            Ok(resp) if resp.ok => WebApiEvent::Login(
                resp.json::<LoginResponse>()
                    .map_err(|err| format!("invalid login response: {err}")),
            ),
            Ok(resp) => WebApiEvent::Login(Err(response_error(&resp))),
            Err(err) => WebApiEvent::Login(Err(format!("network error: {err}"))),
        };
        push_web_event(&queue, event);
    });
}

fn web_fetch_capabilities(server_url: &str, queue: Arc<Mutex<Vec<WebApiEvent>>>) {
    let url = web_api_url(server_url, "/api/v1/capabilities");
    let request = ehttp::Request::get(url);
    ehttp::fetch(request, move |result| {
        let event = match result {
            Ok(resp) if resp.ok => WebApiEvent::Capabilities(
                resp.json::<ServerCapabilities>()
                    .map_err(|err| format!("invalid capabilities response: {err}")),
            ),
            Ok(resp) => WebApiEvent::Capabilities(Err(response_error(&resp))),
            Err(err) => WebApiEvent::Capabilities(Err(format!("network error: {err}"))),
        };
        push_web_event(&queue, event);
    });
}

fn web_fetch_artifacts(server_url: &str, token: Option<&str>, queue: Arc<Mutex<Vec<WebApiEvent>>>) {
    let url = web_api_url(server_url, "/api/v1/artifacts");
    let request = web_request_with_auth(url, token);
    ehttp::fetch(request, move |result| {
        let event = match result {
            Ok(resp) if resp.ok => WebApiEvent::Artifacts(
                resp.json::<Vec<ArtifactSummary>>()
                    .map_err(|err| format!("invalid artifacts response: {err}")),
            ),
            Ok(resp) => WebApiEvent::Artifacts(Err(response_error(&resp))),
            Err(err) => WebApiEvent::Artifacts(Err(format!("network error: {err}"))),
        };
        push_web_event(&queue, event);
    });
}

fn web_upload_artifact(
    server_url: &str,
    _token: Option<&str>,
    name: String,
    note: Option<String>,
    elf: Vec<u8>,
    queue: Arc<Mutex<Vec<WebApiEvent>>>,
) {
    let url = web_api_url(server_url, "/api/v1/artifacts");
    let mut request = match ehttp::Request::json(
        url,
        &UploadArtifactRequest {
            name,
            note,
            target: "riscv32imafc-unknown-none-elf".to_string(),
            elf_base64: base64::engine::general_purpose::STANDARD.encode(elf),
        },
    ) {
        Ok(req) => req,
        Err(err) => {
            push_web_event(
                &queue,
                WebApiEvent::UploadResult(Err(format!(
                    "failed to serialize upload payload: {err}"
                ))),
            );
            return;
        }
    };
    request.method = "POST".to_string();
    #[cfg(not(target_arch = "wasm32"))]
    let token = _token;
    #[cfg(target_arch = "wasm32")]
    let token: Option<&str> = None;
    if let Some(token) = token {
        request
            .headers
            .insert("Authorization", format!("Bearer {token}"));
    }

    ehttp::fetch(request, move |result| {
        let event = match result {
            Ok(resp) if resp.ok => WebApiEvent::UploadResult(
                resp.json::<UploadArtifactResponse>()
                    .map_err(|err| format!("invalid upload response: {err}")),
            ),
            Ok(resp) => WebApiEvent::UploadResult(Err(response_error(&resp))),
            Err(err) => WebApiEvent::UploadResult(Err(format!("network error: {err}"))),
        };
        push_web_event(&queue, event);
    });
}

fn web_delete_artifact(
    server_url: &str,
    _token: Option<&str>,
    artifact_id: i64,
    queue: Arc<Mutex<Vec<WebApiEvent>>>,
) {
    let url = web_api_url(server_url, &format!("/api/v1/artifacts/{artifact_id}"));
    let mut request = ehttp::Request::get(url);
    request.method = "DELETE".to_string();
    #[cfg(not(target_arch = "wasm32"))]
    let token = _token;
    #[cfg(target_arch = "wasm32")]
    let token: Option<&str> = None;
    if let Some(token) = token {
        request
            .headers
            .insert("Authorization", format!("Bearer {token}"));
    }

    ehttp::fetch(request, move |result| {
        let event = match result {
            Ok(resp) if resp.ok => WebApiEvent::DeleteResult {
                artifact_id,
                result: Ok(()),
            },
            Ok(resp) => WebApiEvent::DeleteResult {
                artifact_id,
                result: Err(response_error(&resp)),
            },
            Err(err) => WebApiEvent::DeleteResult {
                artifact_id,
                result: Err(format!("network error: {err}")),
            },
        };
        push_web_event(&queue, event);
    });
}

fn web_set_artifact_visibility(
    server_url: &str,
    _token: Option<&str>,
    artifact_id: i64,
    is_public: bool,
    queue: Arc<Mutex<Vec<WebApiEvent>>>,
) {
    let url = web_api_url(
        server_url,
        &format!("/api/v1/artifacts/{artifact_id}/visibility"),
    );
    let mut request =
        match ehttp::Request::json(url, &UpdateArtifactVisibilityRequest { is_public }) {
            Ok(req) => req,
            Err(err) => {
                push_web_event(
                    &queue,
                    WebApiEvent::VisibilityResult {
                        artifact_id,
                        is_public,
                        result: Err(format!("failed to serialize visibility payload: {err}")),
                    },
                );
                return;
            }
        };
    request.method = "PATCH".to_string();
    #[cfg(not(target_arch = "wasm32"))]
    let token = _token;
    #[cfg(target_arch = "wasm32")]
    let token: Option<&str> = None;
    if let Some(token) = token {
        request
            .headers
            .insert("Authorization", format!("Bearer {token}"));
    }

    ehttp::fetch(request, move |result| {
        let event = match result {
            Ok(resp) if resp.ok => WebApiEvent::VisibilityResult {
                artifact_id,
                is_public,
                result: Ok(()),
            },
            Ok(resp) => WebApiEvent::VisibilityResult {
                artifact_id,
                is_public,
                result: Err(response_error(&resp)),
            },
            Err(err) => WebApiEvent::VisibilityResult {
                artifact_id,
                is_public,
                result: Err(format!("network error: {err}")),
            },
        };
        push_web_event(&queue, event);
    });
}

fn web_fetch_artifact_elf(
    server_url: &str,
    token: Option<&str>,
    artifact_id: i64,
    request_id: u64,
    results_queue: Arc<Mutex<Vec<CompileResult>>>,
) {
    let url = web_api_url(server_url, &format!("/api/v1/artifacts/{artifact_id}"));
    let request = web_request_with_auth(url, token);
    ehttp::fetch(request, move |result| {
        let compile_result = match result {
            Ok(resp) if resp.ok => CompileResult {
                id: request_id,
                binary: format!("artifact_{artifact_id}"),
                result: Ok(resp.bytes),
            },
            Ok(resp) => CompileResult {
                id: request_id,
                binary: format!("artifact_{artifact_id}"),
                result: Err(response_error(&resp)),
            },
            Err(err) => CompileResult {
                id: request_id,
                binary: format!("artifact_{artifact_id}"),
                result: Err(format!("network error: {err}")),
            },
        };
        if let Ok(mut pending) = results_queue.lock() {
            pending.push(compile_result);
        }
    });
}

fn maybe_auth_token(web_state: &WebPortalState) -> Result<Option<String>, String> {
    match web_state.auth_required {
        Some(true) => {
            #[cfg(target_arch = "wasm32")]
            {
                Ok(None)
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                web_state
                    .token
                    .clone()
                    .map(Some)
                    .ok_or_else(|| "[auth] Login required".to_string())
            }
        }
        Some(false) => Ok(None),
        None => Err("[capabilities] Server capabilities not loaded yet".to_string()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn pick_artifact_for_upload_native() -> Result<Option<(String, Vec<u8>)>, String> {
    let Some(path) = rfd::FileDialog::new().pick_file() else {
        return Ok(None);
    };
    let bytes = std::fs::read(&path).map_err(|e| format!("failed to read file: {e}"))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "artifact.elf".to_string());
    Ok(Some((name, bytes)))
}

#[cfg(target_arch = "wasm32")]
fn pick_artifact_for_upload_web(
    server_url: String,
    token: Option<String>,
    queue: Arc<Mutex<Vec<WebApiEvent>>>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let Some(file) = rfd::AsyncFileDialog::new().pick_file().await else {
            return;
        };
        let bytes = file.read().await;
        let name = file.file_name();
        web_upload_artifact(&server_url, token.as_deref(), name, None, bytes, queue);
    });
}

fn handle_web_api_commands(
    mut commands: MessageReader<WebApiCommand>,
    mut web_state: ResMut<WebPortalState>,
    web_queue: Res<WebApiQueue>,
) {
    for command in commands.read() {
        match command {
            WebApiCommand::RefreshCapabilities => {
                web_state.status_message =
                    Some("[capabilities] Loading server capabilities...".to_string());
                web_fetch_capabilities(&web_state.server_url, web_queue.events.clone());
            }
            WebApiCommand::LoadArtifacts => {
                if web_state.auth_required.is_none() {
                    web_state.status_message =
                        Some("[capabilities] Checking server capabilities first...".to_string());
                    web_fetch_capabilities(&web_state.server_url, web_queue.events.clone());
                    continue;
                }
                let token = match maybe_auth_token(&web_state) {
                    Ok(token) => token,
                    Err(error) => {
                        web_state.status_message = Some(error);
                        continue;
                    }
                };
                web_state.status_message = Some("[load] Loading artifacts...".to_string());
                web_fetch_artifacts(
                    &web_state.server_url,
                    token.as_deref(),
                    web_queue.events.clone(),
                );
            }
            WebApiCommand::UploadArtifact => {
                if web_state.auth_required.is_none() {
                    web_state.status_message =
                        Some("[capabilities] Checking server capabilities first...".to_string());
                    web_fetch_capabilities(&web_state.server_url, web_queue.events.clone());
                    continue;
                }
                let token = match maybe_auth_token(&web_state) {
                    Ok(token) => token,
                    Err(error) => {
                        web_state.status_message = Some(error);
                        continue;
                    }
                };
                #[cfg(not(target_arch = "wasm32"))]
                match pick_artifact_for_upload_native() {
                    Ok(Some((name, bytes))) => {
                        web_state.status_message = Some(format!("[upload] Uploading '{name}'..."));
                        web_upload_artifact(
                            &web_state.server_url,
                            token.as_deref(),
                            name,
                            None,
                            bytes,
                            web_queue.events.clone(),
                        );
                    }
                    Ok(None) => {}
                    Err(error) => {
                        web_state.status_message = Some(error);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    web_state.status_message =
                        Some("[upload] Pick artifact to upload...".to_string());
                    pick_artifact_for_upload_web(
                        web_state.server_url.clone(),
                        token,
                        web_queue.events.clone(),
                    );
                }
            }
            WebApiCommand::DeleteArtifact { id } => {
                if web_state.auth_required.is_none() {
                    web_state.status_message =
                        Some("[capabilities] Checking server capabilities first...".to_string());
                    web_fetch_capabilities(&web_state.server_url, web_queue.events.clone());
                    continue;
                }
                let token = match maybe_auth_token(&web_state) {
                    Ok(token) => token,
                    Err(error) => {
                        web_state.status_message = Some(error);
                        continue;
                    }
                };
                web_state.status_message = Some(format!("[delete] Deleting artifact #{id}..."));
                web_delete_artifact(
                    &web_state.server_url,
                    token.as_deref(),
                    *id,
                    web_queue.events.clone(),
                );
            }
            WebApiCommand::SetArtifactVisibility { id, is_public } => {
                if web_state.auth_required.is_none() {
                    web_state.status_message =
                        Some("[capabilities] Checking server capabilities first...".to_string());
                    web_fetch_capabilities(&web_state.server_url, web_queue.events.clone());
                    continue;
                }
                let token = match maybe_auth_token(&web_state) {
                    Ok(token) => token,
                    Err(error) => {
                        web_state.status_message = Some(error);
                        continue;
                    }
                };
                let visibility = if *is_public { "public" } else { "private" };
                web_state.status_message = Some(format!(
                    "[visibility] Setting artifact #{id} to {visibility}..."
                ));
                web_set_artifact_visibility(
                    &web_state.server_url,
                    token.as_deref(),
                    *id,
                    *is_public,
                    web_queue.events.clone(),
                );
            }
        }
    }
}

fn process_web_api_events(mut web_state: ResMut<WebPortalState>, web_queue: Res<WebApiQueue>) {
    let mut events = Vec::new();
    if let Ok(mut queue) = web_queue.events.lock() {
        events.append(&mut *queue);
    }

    for event in events {
        match event {
            WebApiEvent::Capabilities(result) => match result {
                Ok(caps) => {
                    web_state.auth_required = Some(caps.auth_required);
                    web_state.status_message = Some(format!(
                        "[capabilities] Connected: mode={}, auth_required={}, registration_enabled={}",
                        caps.mode, caps.auth_required, caps.registration_enabled
                    ));
                    #[cfg(not(target_arch = "wasm32"))]
                    if caps.auth_required && web_state.token.is_none() {
                        if let Some((username, password)) = web_state.cli_credentials.clone() {
                            web_state.status_message =
                                Some(format!("[auth] Logging in as '{username}'..."));
                            web_fetch_login(
                                &web_state.server_url,
                                &username,
                                &password,
                                web_queue.events.clone(),
                            );
                            continue;
                        }
                    }
                    if let Ok(token) = maybe_auth_token(&web_state) {
                        web_fetch_artifacts(
                            &web_state.server_url,
                            token.as_deref(),
                            web_queue.events.clone(),
                        );
                    }
                }
                Err(error) => {
                    web_state.status_message = Some(format!(
                        "[error][capabilities] Capability check failed: {error}"
                    ));
                }
            },
            #[cfg(not(target_arch = "wasm32"))]
            WebApiEvent::Login(result) => match result {
                Ok(login) => {
                    web_state.token = Some(login.token);
                    web_state.status_message =
                        Some(format!("[auth] Logged in as {}", login.user.username));
                    web_fetch_artifacts(
                        &web_state.server_url,
                        web_state.token.as_deref(),
                        web_queue.events.clone(),
                    );
                }
                Err(error) => {
                    web_state.status_message = Some(format!("[error][auth] Login failed: {error}"));
                }
            },
            WebApiEvent::Artifacts(result) => match result {
                Ok(artifacts) => {
                    web_state.artifacts = artifacts;
                    web_state.status_message = Some(format!(
                        "[load] Loaded {} artifacts",
                        web_state.artifacts.len()
                    ));
                }
                Err(error) => {
                    web_state.status_message =
                        Some(format!("[error][load] Loading artifacts failed: {error}"));
                }
            },
            WebApiEvent::UploadResult(result) => match result {
                Ok(upload) => {
                    web_state.status_message = Some(format!(
                        "[upload] Uploaded artifact #{}",
                        upload.artifact_id
                    ));
                    if let Ok(token) = maybe_auth_token(&web_state) {
                        web_fetch_artifacts(
                            &web_state.server_url,
                            token.as_deref(),
                            web_queue.events.clone(),
                        );
                    }
                }
                Err(error) => {
                    web_state.status_message =
                        Some(format!("[error][upload] Upload failed: {error}"));
                }
            },
            WebApiEvent::DeleteResult {
                artifact_id,
                result,
            } => match result {
                Ok(()) => {
                    web_state.status_message =
                        Some(format!("[delete] Deleted artifact #{artifact_id}"));
                    if let Ok(token) = maybe_auth_token(&web_state) {
                        web_fetch_artifacts(
                            &web_state.server_url,
                            token.as_deref(),
                            web_queue.events.clone(),
                        );
                    }
                }
                Err(error) => {
                    web_state.status_message = Some(format!(
                        "[error][delete] Failed to delete artifact #{artifact_id}: {error}"
                    ));
                }
            },
            WebApiEvent::VisibilityResult {
                artifact_id,
                is_public,
                result,
            } => match result {
                Ok(()) => {
                    let visibility = if is_public { "public" } else { "private" };
                    web_state.status_message = Some(format!(
                        "[visibility] Set artifact #{artifact_id} to {visibility}"
                    ));
                    if let Ok(token) = maybe_auth_token(&web_state) {
                        web_fetch_artifacts(
                            &web_state.server_url,
                            token.as_deref(),
                            web_queue.events.clone(),
                        );
                    }
                }
                Err(error) => {
                    web_state.status_message = Some(format!(
                        "[error][visibility] Failed to update artifact #{artifact_id}: {error}"
                    ));
                }
            },
        }
    }
}

fn handle_spawn_car_request(
    mut events: MessageReader<SpawnCarRequest>,
    mut fetch_pipeline: ResMut<ArtifactFetchPipeline>,
    mut web_state: ResMut<WebPortalState>,
    state: Res<State<SimState>>,
) {
    for event in events.read() {
        if *state.get() != SimState::PreRace {
            continue;
        }

        let request_id = fetch_pipeline.next_request_id;
        fetch_pipeline.next_request_id += 1;
        fetch_pipeline
            .pending
            .insert(request_id, event.driver.clone());

        match &event.driver {
            DriverType::RemoteArtifact { id } => {
                let token = match maybe_auth_token(&web_state) {
                    Ok(token) => token,
                    Err(error) => {
                        fetch_pipeline.pending.remove(&request_id);
                        web_state.status_message = Some(error);
                        continue;
                    }
                };
                web_state.status_message = Some(format!("Downloading artifact #{id}..."));
                web_fetch_artifact_elf(
                    &web_state.server_url,
                    token.as_deref(),
                    *id,
                    request_id,
                    fetch_pipeline.async_results.clone(),
                );
            }
        }
    }
}

fn process_artifact_fetch_results(
    mut fetch_pipeline: ResMut<ArtifactFetchPipeline>,
    mut resolved_events: MessageWriter<SpawnResolvedCarRequest>,
    mut web_state: ResMut<WebPortalState>,
    state: Res<State<SimState>>,
) {
    let mut results = Vec::new();
    if let Ok(mut async_results) = fetch_pipeline.async_results.lock() {
        results.append(&mut *async_results);
    }

    for result in results {
        let Some(driver) = fetch_pipeline.pending.remove(&result.id) else {
            continue;
        };

        match result.result {
            Ok(elf_bytes) => {
                if *state.get() != SimState::PreRace {
                    web_state.status_message = Some(format!(
                        "Discarded compiled '{}' result (race already started)",
                        result.binary
                    ));
                    continue;
                }

                resolved_events.write(SpawnResolvedCarRequest {
                    driver,
                    elf_bytes,
                    binary_name: result.binary.clone(),
                });
                web_state.status_message = Some(format!("Loaded and spawned '{}'", result.binary));
            }
            Err(error) => {
                web_state.status_message = Some(format!(
                    "Artifact load failed for '{}': {}",
                    result.binary, error
                ));
            }
        }
    }
}
