use anyhow::{Context as _, Result};
use collections::HashMap;
use gpui::{
    actions, App, AppContext, Context, Entity, EventEmitter, Global, Task,
};
use http_client::HttpClient;
use language::Buffer;
use lsp::{LanguageServer, LanguageServerBinary};
use node_runtime::NodeRuntime;
use paths::copilot_dir;
use std::{
    env,
    fmt::Display,
    sync::Arc,
    time::Duration,
};

pub mod auth;
pub mod completion_diff_element;
pub mod copilotv2_provider;
pub mod request;

// Re-export main types for external use
pub use copilotv2_provider::CopilotV2Provider;

// use auth::{SignInModal, SignInStatus};
// use request::*;

actions!(
    copilotv2,
    [SignIn, SignOut, ToggleDataCollection, Suggest, NextCompletion, PrevCompletion]
);

pub fn init(
    http_client: Arc<dyn HttpClient>,
    node_runtime: Arc<NodeRuntime>,
    cx: &mut App,
) {
    log::info!("CopilotV2: Initializing...");

    let copilotv2 = cx.new(|cx| CopilotV2::start(http_client, node_runtime, cx));

    cx.set_global(CopilotV2Global { copilotv2 });
    log::info!("CopilotV2: Initialization complete");
}

#[derive(Clone)]
pub struct CopilotV2Global {
    pub copilotv2: Entity<CopilotV2>,
}

impl Global for CopilotV2Global {}

impl CopilotV2Global {
    pub fn global(cx: &App) -> Option<&Self> {
        cx.try_global::<Self>()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CopilotV2Status {
    /// CopilotV2 is starting up
    Starting,
    /// CopilotV2 server is downloading
    Downloading,
    /// CopilotV2 server failed to start
    Error(String),
    /// CopilotV2 is signed out
    SignedOut,
    /// CopilotV2 is signing in
    SigningIn {
        prompt: String,
    },
    /// CopilotV2 is signed in and ready
    SignedIn {
        username: String,
    },
    /// CopilotV2 authentication failed
    Unauthorized,
}

impl Display for CopilotV2Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CopilotV2Status::Starting => write!(f, "starting"),
            CopilotV2Status::Downloading => write!(f, "downloading"),
            CopilotV2Status::Error(e) => write!(f, "error: {}", e),
            CopilotV2Status::SignedOut => write!(f, "signed out"),
            CopilotV2Status::SigningIn { .. } => write!(f, "signing in"),
            CopilotV2Status::SignedIn { username } => write!(f, "signed in as {}", username),
            CopilotV2Status::Unauthorized => write!(f, "unauthorized"),
        }
    }
}

impl CopilotV2Status {
    pub fn is_authorized(&self) -> bool {
        matches!(self, CopilotV2Status::SignedIn { .. })
    }

    pub fn is_signing_in(&self) -> bool {
        matches!(self, CopilotV2Status::SigningIn { .. })
    }
}

#[allow(dead_code)]
enum CopilotV2Server {
    Running(LanguageServer),
    Error(String),
}

pub struct CopilotV2 {
    #[allow(dead_code)]
    server: Option<CopilotV2Server>,
    status: CopilotV2Status,
    http_client: Arc<dyn HttpClient>,
    node_runtime: Arc<NodeRuntime>,
    #[allow(dead_code)]
    registered_buffers: HashMap<u64, RegisteredBuffer>,
    _maintain_server: Task<()>,
}

struct RegisteredBuffer {
    #[allow(dead_code)]
    buffer: Entity<Buffer>,
    #[allow(dead_code)]
    language_server_id: u64,
}

impl EventEmitter<()> for CopilotV2 {}

impl CopilotV2 {
    pub fn start(
        http_client: Arc<dyn HttpClient>,
        node_runtime: Arc<NodeRuntime>,
        cx: &mut App,
    ) -> Self {
        log::info!("CopilotV2: Starting CopilotV2 service");

        let mut this = Self {
            server: None,
            status: CopilotV2Status::SignedIn {
                username: "test-user".to_string(),
            }, // Start as signed in for testing
            http_client,
            node_runtime,
            registered_buffers: Default::default(),
            _maintain_server: Task::ready(()),
        };

        this.start_language_server(cx);
        this
    }

    pub fn status(&self) -> &CopilotV2Status {
        &self.status
    }

    fn start_language_server(&mut self, cx: &mut App) {
        log::info!("CopilotV2: Starting language server");

        let http_client = self.http_client.clone();
        let node_runtime = self.node_runtime.clone();

        self._maintain_server = cx.spawn(async move |_cx| {
            log::debug!("CopilotV2: Background task started for server maintenance");

            // Download and install the LSP server
            match get_copilot_lsp(http_client, node_runtime.clone()).await {
                Ok(_server_binary) => {
                    log::info!("CopilotV2: Successfully obtained LSP server binary");
                    log::info!("CopilotV2: LSP server binary ready for use");
                }
                Err(e) => {
                    let error_msg = format!("Failed to get LSP server: {}", e);
                    log::error!("CopilotV2: {}", error_msg);
                }
            }
        });
    }

    pub fn sign_in(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        log::info!("CopilotV2: Starting sign-in process");

        cx.spawn(async move |_this, cx| {
            log::debug!("CopilotV2: Mock sign-in process started");

            // Mock successful sign-in after delay
            cx.background_executor().timer(Duration::from_secs(2)).await;

            log::info!("CopilotV2: Successfully signed in as test-user");

            Ok(())
        })
    }

    pub fn sign_out(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        log::info!("CopilotV2: Starting sign-out process");

        cx.spawn(async move |_this, _cx| {
            log::debug!("CopilotV2: Mock sign-out process");
            log::info!("CopilotV2: Successfully signed out");
            Ok(())
        })
    }

    pub fn register_buffer(&mut self, buffer: &Entity<Buffer>, _cx: &mut Context<Self>) -> Task<Result<()>> {
        let buffer_id = buffer.entity_id().as_u64();
        log::debug!("CopilotV2: Registering buffer with ID: {}", buffer_id);

        // Mock implementation for now
        Task::ready(Ok(()))
    }
}

async fn get_copilot_lsp(
    _http_client: Arc<dyn HttpClient>,
    node_runtime: Arc<NodeRuntime>,
) -> Result<LanguageServerBinary> {
    log::debug!("CopilotV2: Getting Copilot LSP server");

    let package_name = "@github/copilot-language-server";
    let server_path = copilot_dir().join("node_modules/@github/copilot-language-server");

    log::debug!("CopilotV2: Installing npm package: {}", package_name);

    node_runtime
        .npm_install_packages(
            &copilot_dir(),
            &[(package_name, "latest")],
        )
        .await
        .context("Failed to install Copilot language server")?;

    let server_script = server_path.join("lib/copilot-language-server.js");

    log::debug!("CopilotV2: Server script path: {:?}", server_script);

    Ok(LanguageServerBinary {
        path: node_runtime.binary_path().await?,
        arguments: vec![server_script.to_string_lossy().to_string().into(), "--stdio".to_string().into()],
        env: build_env(),
    })
}

fn build_env() -> Option<HashMap<String, String>> {
    let mut env: HashMap<String, String> = Default::default();

    // Add proxy configuration if available
    if let Ok(proxy) = env::var("HTTP_PROXY") {
        env.insert("HTTP_PROXY".to_string(), proxy);
        log::debug!("CopilotV2: HTTP_PROXY configured");
    }

    if let Ok(proxy) = env::var("HTTPS_PROXY") {
        env.insert("HTTPS_PROXY".to_string(), proxy);
        log::debug!("CopilotV2: HTTPS_PROXY configured");
    }

    if env.is_empty() {
        None
    } else {
        Some(env)
    }
}