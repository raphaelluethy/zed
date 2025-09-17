use gpui::{
    actions, App, ClipboardItem, Context, DismissEvent, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, Styled, Window, div,
};
use serde::{Deserialize, Serialize};
use ui::{prelude::*, Button, Label};

actions!(copilot_auth, [CopyDeviceCode, SubmitDeviceCode]);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignInStatus {
    SignedOut,
    SigningIn {
        prompt: String,
        user_code: String,
    },
    SignedIn {
        username: String
    },
    Unauthorized,
}

impl SignInStatus {
    pub fn is_authorized(&self) -> bool {
        matches!(self, SignInStatus::SignedIn { .. })
    }

    pub fn is_signing_in(&self) -> bool {
        matches!(self, SignInStatus::SigningIn { .. })
    }
}

#[derive(Serialize, Deserialize)]
pub struct SignInInitiateParams {}

#[derive(Serialize, Deserialize)]
pub struct SignInInitiateResponse {
    pub status: String,
    pub user_code: String,
    pub verification_uri: String,
}

#[derive(Serialize, Deserialize)]
pub struct SignInConfirmParams {
    pub user_code: String,
}

#[derive(Serialize, Deserialize)]
pub struct SignInConfirmResponse {
    pub status: String,
    pub user: String,
}

pub struct SignInModal {
    device_code: String,
    verification_uri: String,
    focus_handle: FocusHandle,
    copied: bool,
}

impl SignInModal {
    pub fn new(device_code: String, cx: &mut App) -> Self {
        log::debug!("CopilotV2 Auth: Creating SignInModal with device code: {}", device_code);

        Self {
            device_code,
            verification_uri: "https://github.com/login/device".to_string(),
            focus_handle: cx.focus_handle(),
            copied: false,
        }
    }

    #[allow(dead_code)]
    fn copy_device_code(&mut self, _: &CopyDeviceCode, _window: &mut Window, cx: &mut Context<Self>) {
        log::debug!("CopilotV2 Auth: Copying device code to clipboard");
        cx.write_to_clipboard(ClipboardItem::new_string(self.device_code.clone()));
        self.copied = true;
        cx.notify();
    }

    #[allow(dead_code)]
    fn submit_device_code(&mut self, _: &SubmitDeviceCode, _window: &mut Window, cx: &mut Context<Self>) {
        log::debug!("CopilotV2 Auth: Submitting device code for confirmation");
        cx.emit(DismissEvent);
    }
}

impl Focusable for SignInModal {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for SignInModal {}

impl Render for SignInModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        log::debug!("CopilotV2 Auth: Rendering SignInModal");

        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .max_w_96()
            .bg(cx.theme().colors().panel_background)
            .border_1()
            .border_color(cx.theme().colors().border)
            .rounded_lg()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new("Sign in to GitHub Copilot V2"))
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Label::new("1. Visit the verification URL:"))
                    .child(
                        div()
                            .p_2()
                            .border_1()
                            .border_color(cx.theme().colors().border)
                            .rounded_md()
                            .bg(cx.theme().colors().surface_background)
                            .child(Label::new(self.verification_uri.clone()))
                    )
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Label::new("2. Enter this device code:"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .p_2()
                                    .border_1()
                                    .border_color(cx.theme().colors().border)
                                    .rounded_md()
                                    .bg(cx.theme().colors().surface_background)
                                    .child(Label::new(self.device_code.clone()))
                            )
                            .child(
                                Button::new("copy-code", if self.copied { "Copied!" } else { "Copy" })
                                    .on_click({
                                        let device_code = self.device_code.clone();
                                        move |_, _, cx| {
                                            cx.write_to_clipboard(ClipboardItem::new_string(device_code.clone()));
                                            log::debug!("CopilotV2 Auth: Device code copied to clipboard");
                                        }
                                    })
                            )
                    )
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel", "Cancel")
                            .on_click(cx.listener(|_this, _, _, cx| {
                                log::debug!("CopilotV2 Auth: Cancel button clicked");
                                cx.emit(DismissEvent);
                            }))
                    )
                    .child(
                        Button::new("continue", "Continue")
                            .on_click(cx.listener(|_this, _, _, cx| {
                                log::debug!("CopilotV2 Auth: Continue button clicked");
                                cx.emit(DismissEvent);
                            }))
                    )
            )
    }
}