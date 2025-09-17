use crate::CopilotV2;
use anyhow::Result;
use edit_prediction::{Direction, EditPrediction, EditPredictionProvider};
use gpui::{App, Context, Entity, EntityId, Task};
use language::{Buffer, OffsetRangeExt, ToOffset, language_settings::AllLanguageSettings, Anchor};
use project::Project;
use settings::Settings;
use std::{path::Path, time::Duration};

pub const COPILOTV2_DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(75);

pub struct CopilotV2Provider {
    cycled: bool,
    buffer_id: Option<EntityId>,
    completions: Vec<MockCompletion>,
    active_completion_index: usize,
    file_extension: Option<String>,
    pending_refresh: Option<Task<Result<()>>>,
    pending_cycling_refresh: Option<Task<Result<()>>>,
    copilotv2: Entity<CopilotV2>,
}

// Mock completion structure for testing
#[derive(Clone, Debug)]
struct MockCompletion {
    text: String,
    range: std::ops::Range<Anchor>,
}

impl CopilotV2Provider {
    pub fn new(copilotv2: Entity<CopilotV2>) -> Self {
        log::debug!("CopilotV2 Provider: Creating new CopilotV2Provider");

        Self {
            cycled: false,
            buffer_id: None,
            completions: Vec::new(),
            active_completion_index: 0,
            file_extension: None,
            pending_refresh: None,
            pending_cycling_refresh: None,
            copilotv2,
        }
    }

    fn active_completion(&self) -> Option<&MockCompletion> {
        self.completions.get(self.active_completion_index)
    }

    fn push_completion(&mut self, new_completion: MockCompletion) {
        for completion in &self.completions {
            if completion.text == new_completion.text && completion.range == new_completion.range {
                return;
            }
        }
        log::debug!("CopilotV2 Provider: Added new completion: '{}'", new_completion.text);
        self.completions.push(new_completion);
    }

    // Generate mock completions for testing
    fn generate_mock_completions(&self, buffer: &Entity<Buffer>, cursor_position: Anchor, cx: &App) -> Vec<MockCompletion> {
        log::debug!("CopilotV2 Provider: Generating mock completions");

        let buffer_read = buffer.read(cx);
        let cursor_offset = cursor_position.to_offset(&buffer_read);

        // Validate cursor position to prevent rope overflow
        let text = buffer_read.text();
        if cursor_offset > text.len() {
            log::warn!("CopilotV2 Provider: Invalid cursor position, skipping completions");
            return Vec::new();
        }

        // Get safe context around the cursor
        let line_start = text[..cursor_offset].rfind('\n').map_or(0, |pos| pos + 1);
        let current_line = &text[line_start..cursor_offset];

        log::debug!("CopilotV2 Provider: Current line: '{}'", current_line);

        // Create a safe cursor anchor for insertion
        let safe_cursor = buffer_read.anchor_at(cursor_offset, language::Bias::Right);

        // Generate ONE primary completion to avoid cycling issues
        let completion_text = if current_line.trim_end().ends_with("function ") {
            "myFunction() {\n    // TODO: Implement function\n}".to_string()
        } else if current_line.trim_end().ends_with("//") {
            " TODO: Add implementation here".to_string()
        } else if current_line.trim_end().ends_with("console.") {
            "log('Debug message')".to_string()
        } else {
            // Default completion
            "// CopilotV2 suggestion".to_string()
        };

        let completion = MockCompletion {
            text: completion_text,
            range: safe_cursor..safe_cursor, // Use safe anchor for both start and end
        };

        log::debug!("CopilotV2 Provider: Generated 1 mock completion: '{}'", completion.text);
        vec![completion]
    }
}

impl EditPredictionProvider for CopilotV2Provider {
    fn name() -> &'static str {
        "copilotv2"
    }

    fn display_name() -> &'static str {
        "Copilot V2"
    }

    fn show_completions_in_menu() -> bool {
        true
    }

    fn show_tab_accept_marker() -> bool {
        true
    }

    fn supports_jump_to_edit() -> bool {
        false
    }

    fn is_refreshing(&self) -> bool {
        self.pending_refresh.is_some() && self.completions.is_empty()
    }

    fn is_enabled(
        &self,
        _buffer: &Entity<Buffer>,
        _cursor_position: language::Anchor,
        cx: &App,
    ) -> bool {
        // For now, always enable the mock provider for testing
        let copilot_status = self.copilotv2.read(cx).status();
        log::info!("CopilotV2 Provider: is_enabled called, status = {:?}", copilot_status);

        // Enable for testing - in production this would check actual auth status
        let is_enabled = true; // copilot_status.is_authorized();
        log::info!("CopilotV2 Provider: is_enabled = {}", is_enabled);
        is_enabled
    }

    fn refresh(
        &mut self,
        _project: Option<Entity<Project>>,
        buffer: Entity<Buffer>,
        cursor_position: language::Anchor,
        debounce: bool,
        cx: &mut Context<Self>,
    ) {
        log::info!("CopilotV2 Provider: refresh() called, debounce = {}", debounce);
        log::info!("CopilotV2 Provider: buffer_id = {:?}, cursor = {:?}", buffer.entity_id(), cursor_position);

        let _copilotv2 = self.copilotv2.clone();
        self.pending_refresh = Some(cx.spawn(async move |this, cx| {
            if debounce {
                log::debug!("CopilotV2 Provider: Applying debounce timeout");
                cx.background_executor()
                    .timer(COPILOTV2_DEBOUNCE_TIMEOUT)
                    .await;
            }

            // For now, generate mock completions instead of calling LSP
            // In the future, this will call: copilotv2.update(cx, |copilotv2, cx| copilotv2.completions(&buffer, cursor_position, cx))

            this.update(cx, |this, cx| {
                log::debug!("CopilotV2 Provider: Generating completions");

                // Generate mock completions
                let mock_completions = this.generate_mock_completions(&buffer, cursor_position, cx);

                if !mock_completions.is_empty() {
                    this.cycled = false;
                    this.pending_refresh = None;
                    this.pending_cycling_refresh = None;
                    this.completions.clear();
                    this.active_completion_index = 0;
                    this.buffer_id = Some(buffer.entity_id());
                    this.file_extension = buffer.read(cx).file().and_then(|file| {
                        Some(
                            Path::new(file.file_name(cx))
                                .extension()?
                                .to_str()?
                                .to_string(),
                        )
                    });

                    for completion in mock_completions {
                        this.push_completion(completion);
                    }

                    log::debug!("CopilotV2 Provider: Refresh complete, {} completions available", this.completions.len());
                    cx.notify();
                } else {
                    log::debug!("CopilotV2 Provider: No completions generated");
                }
            })?;

            Ok(())
        }));
    }

    fn cycle(
        &mut self,
        _buffer: Entity<Buffer>,
        _cursor_position: language::Anchor,
        direction: Direction,
        cx: &mut Context<Self>,
    ) {
        log::info!("CopilotV2 Provider: cycle() called, direction = {}", match direction {
            Direction::Prev => "Prev",
            Direction::Next => "Next",
        });

        // For now, disable cycling to prevent multiple completion issues
        // Just generate a new single completion instead
        if !self.completions.is_empty() {
            log::info!("CopilotV2 Provider: Cycling disabled for stability - keeping current completion");
            // Don't change the current completion, just notify
            cx.notify();
        }
    }

    fn accept(&mut self, _cx: &mut Context<Self>) {
        log::debug!("CopilotV2 Provider: accept() called");

        if let Some(completion) = self.active_completion() {
            log::info!("CopilotV2 Provider: Accepting completion: '{}'", completion.text);

            // In the future, notify the LSP server about acceptance
            // self.copilotv2.update(cx, |copilotv2, cx| copilotv2.accept_completion(completion, cx))
        } else {
            log::debug!("CopilotV2 Provider: No active completion to accept");
        }
    }

    fn discard(&mut self, cx: &mut Context<Self>) {
        log::debug!("CopilotV2 Provider: discard() called");

        let settings = AllLanguageSettings::get_global(cx);
        let copilotv2_enabled = settings.show_edit_predictions(None, cx);

        if !copilotv2_enabled {
            log::debug!("CopilotV2 Provider: Edit predictions disabled, skipping discard");
            return;
        }

        if !self.completions.is_empty() {
            log::info!("CopilotV2 Provider: Discarding {} completions", self.completions.len());

            // In the future, notify the LSP server about rejection
            // self.copilotv2.update(cx, |copilotv2, cx| copilotv2.discard_completions(&self.completions, cx))
        }
    }

    fn suggest(
        &mut self,
        buffer: &Entity<Buffer>,
        cursor_position: language::Anchor,
        cx: &mut Context<Self>,
    ) -> Option<EditPrediction> {
        log::info!("CopilotV2 Provider: suggest() called");
        log::info!("CopilotV2 Provider: Have {} completions available", self.completions.len());

        let buffer_id = buffer.entity_id();
        let buffer_read = buffer.read(cx);
        let completion = self.active_completion()?;

        if Some(buffer_id) != self.buffer_id
            || !completion.range.start.is_valid(&buffer_read)
            || !completion.range.end.is_valid(&buffer_read)
        {
            log::debug!("CopilotV2 Provider: Completion invalid for current buffer");
            return None;
        }

        let completion_range = completion.range.to_offset(&buffer_read);
        let cursor_offset = cursor_position.to_offset(&buffer_read);

        log::debug!("CopilotV2 Provider: Completion range: {:?}, cursor offset: {}", completion_range, cursor_offset);

        // For simple insertions at cursor position
        if completion_range.is_empty() && completion_range.start == cursor_offset {
            let completion_text = &completion.text;
            if !completion_text.trim().is_empty() {
                let position = cursor_position.bias_right(&buffer_read);

                log::debug!("CopilotV2 Provider: Suggesting insertion: '{}'", completion_text);

                return Some(EditPrediction {
                    id: Some("copilotv2-mock".into()),
                    edits: vec![(position..position, completion_text.clone())],
                    edit_preview: None,
                });
            }
        }

        log::debug!("CopilotV2 Provider: No suitable completion found");
        None
    }
}