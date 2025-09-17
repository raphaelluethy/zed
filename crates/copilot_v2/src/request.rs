use anyhow::Result;
use lsp::{request::Request, LanguageServer, Position, Range};
use serde::{Deserialize, Serialize};

// Authentication requests

pub enum CheckStatus {}

#[derive(Serialize, Deserialize)]
pub struct CheckStatusParams {}

#[derive(Serialize, Deserialize)]
pub struct CheckStatusResult {
    pub status: String,
    pub user: Option<String>,
}

impl Request for CheckStatus {
    type Params = CheckStatusParams;
    type Result = CheckStatusResult;
    const METHOD: &'static str = "checkStatus";
}

pub enum SignInInitiate {}

#[derive(Serialize, Deserialize)]
pub struct SignInInitiateParams {}

#[derive(Serialize, Deserialize)]
pub struct SignInInitiateResult {
    pub status: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: Option<u64>,
    pub interval: Option<u64>,
}

impl Request for SignInInitiate {
    type Params = SignInInitiateParams;
    type Result = SignInInitiateResult;
    const METHOD: &'static str = "signInInitiate";
}

pub enum SignInConfirm {}

#[derive(Serialize, Deserialize)]
pub struct SignInConfirmParams {
    pub user_code: String,
}

#[derive(Serialize, Deserialize)]
pub struct SignInConfirmResult {
    pub status: String,
    pub user: String,
}

impl Request for SignInConfirm {
    type Params = SignInConfirmParams;
    type Result = SignInConfirmResult;
    const METHOD: &'static str = "signInConfirm";
}

pub enum SignOut {}

#[derive(Serialize, Deserialize)]
pub struct SignOutParams {}

#[derive(Serialize, Deserialize)]
pub struct SignOutResult {
    pub status: String,
}

impl Request for SignOut {
    type Params = SignOutParams;
    type Result = SignOutResult;
    const METHOD: &'static str = "signOut";
}

// Completion requests

pub enum GetCompletions {}

#[derive(Serialize, Deserialize)]
pub struct GetCompletionsParams {
    pub doc: GetCompletionsDocument,
}

#[derive(Serialize, Deserialize)]
pub struct GetCompletionsDocument {
    pub uri: String,
    pub version: i32,
    pub position: Position,
    pub insert_spaces: bool,
    pub tab_size: u32,
    pub language_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Completion {
    pub text: String,
    pub range: Range,
    pub display_text: Option<String>,
}

impl Default for Completion {
    fn default() -> Self {
        Self {
            text: String::new(),
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            display_text: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GetCompletionsResult {
    pub completions: Vec<Completion>,
}

impl Request for GetCompletions {
    type Params = GetCompletionsParams;
    type Result = GetCompletionsResult;
    const METHOD: &'static str = "getCompletions";
}

pub enum GetCompletionsCycling {}

#[derive(Serialize, Deserialize)]
pub struct GetCompletionsCyclingParams {
    pub doc: GetCompletionsDocument,
}

impl Request for GetCompletionsCycling {
    type Params = GetCompletionsCyclingParams;
    type Result = GetCompletionsResult;
    const METHOD: &'static str = "getCompletionsCycling";
}

// Feedback requests

pub enum NotifyAccepted {}

#[derive(Serialize, Deserialize)]
pub struct NotifyAcceptedParams {
    pub uuid: String,
}

impl Request for NotifyAccepted {
    type Params = NotifyAcceptedParams;
    type Result = ();
    const METHOD: &'static str = "notifyAccepted";
}

pub enum NotifyRejected {}

#[derive(Serialize, Deserialize)]
pub struct NotifyRejectedParams {
    pub uuids: Vec<String>,
}

impl Request for NotifyRejected {
    type Params = NotifyRejectedParams;
    type Result = ();
    const METHOD: &'static str = "notifyRejected";
}

// Configuration requests

pub enum SetEditorInfo {}

#[derive(Serialize, Deserialize)]
pub struct SetEditorInfoParams {
    pub editor_info: EditorInfo,
}

#[derive(Serialize, Deserialize)]
pub struct EditorInfo {
    pub name: String,
    pub version: String,
}

impl Request for SetEditorInfo {
    type Params = SetEditorInfoParams;
    type Result = ();
    const METHOD: &'static str = "setEditorInfo";
}

// Helper functions for LSP communication with debug logging

pub async fn check_status(server: &LanguageServer) -> Result<CheckStatusResult> {
    log::debug!("CopilotV2 Request: Checking authentication status");

    let result = server.request::<CheckStatus>(CheckStatusParams {}).await
        .into_response()?;

    log::debug!("CopilotV2 Response: CheckStatus = {:?}", result.status);
    if let Some(ref user) = result.user {
        log::debug!("CopilotV2: Authenticated user: {}", user);
    }

    Ok(result)
}

pub async fn sign_in_initiate(server: &LanguageServer) -> Result<SignInInitiateResult> {
    log::debug!("CopilotV2 Request: Initiating sign-in");

    let result = server.request::<SignInInitiate>(SignInInitiateParams {}).await
        .into_response()?;

    log::debug!("CopilotV2 Response: SignInInitiate status = {}", result.status);
    log::debug!("CopilotV2: Device code = {}", result.user_code);
    log::debug!("CopilotV2: Verification URI = {}", result.verification_uri);

    Ok(result)
}

pub async fn sign_in_confirm(server: &LanguageServer, user_code: String) -> Result<SignInConfirmResult> {
    log::debug!("CopilotV2 Request: Confirming sign-in with user code: {}", user_code);

    let result = server.request::<SignInConfirm>(SignInConfirmParams { user_code }).await
        .into_response()?;

    log::debug!("CopilotV2 Response: SignInConfirm status = {}", result.status);
    log::debug!("CopilotV2: Authenticated user = {}", result.user);

    Ok(result)
}

pub async fn sign_out(server: &LanguageServer) -> Result<SignOutResult> {
    log::debug!("CopilotV2 Request: Signing out");

    let result = server.request::<SignOut>(SignOutParams {}).await
        .into_response()?;

    log::debug!("CopilotV2 Response: SignOut status = {}", result.status);

    Ok(result)
}

pub async fn get_completions(
    server: &LanguageServer,
    doc: GetCompletionsDocument,
) -> Result<GetCompletionsResult> {
    log::debug!("CopilotV2 Request: Getting completions for {} at {:?}", doc.uri, doc.position);

    let result = server.request::<GetCompletions>(GetCompletionsParams { doc }).await
        .into_response()?;

    log::debug!("CopilotV2 Response: Received {} completions", result.completions.len());
    for (i, completion) in result.completions.iter().enumerate() {
        log::debug!("CopilotV2: Completion {}: '{}' at {:?}", i, completion.text, completion.range);
    }

    Ok(result)
}

pub async fn get_completions_cycling(
    server: &LanguageServer,
    doc: GetCompletionsDocument,
) -> Result<GetCompletionsResult> {
    log::debug!("CopilotV2 Request: Getting cycling completions for {} at {:?}", doc.uri, doc.position);

    let result = server.request::<GetCompletionsCycling>(GetCompletionsCyclingParams { doc }).await
        .into_response()?;

    log::debug!("CopilotV2 Response: Received {} cycling completions", result.completions.len());
    for (i, completion) in result.completions.iter().enumerate() {
        log::debug!("CopilotV2: Cycling completion {}: '{}' at {:?}", i, completion.text, completion.range);
    }

    Ok(result)
}

pub async fn notify_accepted(server: &LanguageServer, uuid: String) -> Result<()> {
    log::debug!("CopilotV2 Request: Notifying completion accepted: {}", uuid);

    server.request::<NotifyAccepted>(NotifyAcceptedParams { uuid }).await
        .into_response()?;

    log::debug!("CopilotV2 Response: Acceptance notification sent");

    Ok(())
}

pub async fn notify_rejected(server: &LanguageServer, uuids: Vec<String>) -> Result<()> {
    log::debug!("CopilotV2 Request: Notifying completions rejected: {:?}", uuids);

    server.request::<NotifyRejected>(NotifyRejectedParams { uuids }).await
        .into_response()?;

    log::debug!("CopilotV2 Response: Rejection notification sent");

    Ok(())
}

pub async fn set_editor_info(server: &LanguageServer, name: String, version: String) -> Result<()> {
    log::debug!("CopilotV2 Request: Setting editor info: {} v{}", name, version);

    let editor_info = EditorInfo { name, version };
    server.request::<SetEditorInfo>(SetEditorInfoParams { editor_info }).await
        .into_response()?;

    log::debug!("CopilotV2 Response: Editor info set");

    Ok(())
}