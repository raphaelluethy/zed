use crate::output::CompletionResult;
use crate::CommonArgs;
use anyhow::{Context as _, Result};
use client::{Client, UserStore};
use edit_prediction::EditPrediction;
use gpui::{App, AsyncApp, Entity};
use language::{Anchor, Buffer, Point};
use project::{Project, ProjectPath, Worktree};
use reqwest_client::ReqwestClient;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use util::rel_path::RelPath;
use zeta2::{Zeta, ZetaEditPredictionProvider};

pub fn test_zed(args: CommonArgs) -> Result<()> {
    let result = test_zed_internal(args.clone())?;
    crate::output::print_single_result(&result, args.output_format)?;
    Ok(())
}

pub fn test_zed_internal(args: CommonArgs) -> Result<CompletionResult> {
    let start = Instant::now();
    let app = gpui::App::new()?;
    let result = app.run(|cx| async move {
        let client = Arc::new(Client::production(cx.clone()));
        let user_store = cx.new(|cx| UserStore::global(client.clone(), cx));
        let http = Arc::new(ReqwestClient::new());
        let fs = Arc::new(::fs::RealFs::default());
        let node_runtime = node_runtime::RealNodeRuntime::new();

        let project = cx.update(|cx| {
            Project::local(
                client.clone(),
                node_runtime.clone(),
                user_store.clone(),
                languages::default_languages(),
                fs.clone(),
                None,
                cx,
            )
        })?;

        let worktree_path = args.file.parent().unwrap_or(std::path::Path::new("."));
        let worktree = project
            .update(cx, |project, cx| {
                project.create_worktree(worktree_path, true, cx)
            })?
            .await?;

        let file_name = args.file.file_name().unwrap().to_string_lossy().to_string();
        let rel_path = Arc::new(RelPath::from_relative_path(&file_name));
        let project_path = worktree.read_with(cx, |worktree, _cx| ProjectPath {
            worktree_id: worktree.id(),
            path: rel_path.clone(),
        })?;

        let buffer = project
            .update(cx, |project, cx| project.open_buffer(project_path, cx))?
            .await?;

        // Wait for buffer to be ready
        let mut parse_status = buffer.read_with(cx, |buffer, _cx| buffer.parse_status())?;
        while *parse_status.borrow() != language::ParseStatus::Idle {
            parse_status.changed().await?;
        }

        let snapshot = cx.update(|cx| buffer.read(cx).snapshot())?;
        let cursor_point = Point::new(args.line, args.column);
        let cursor_anchor = snapshot.anchor_before(cursor_point);

        // Initialize zeta2
        let zeta = cx.new(|cx| Zeta::new(client.clone(), user_store.clone(), cx));
        zeta.update(cx, |zeta, cx| {
            zeta.register_project(&project, cx);
        });

        // Create provider
        let provider = cx.new(|cx| {
            ZetaEditPredictionProvider::new(project.clone(), &client, &user_store, cx)
        });

        // Refresh prediction
        provider.update(cx, |provider, cx| {
            provider.refresh(buffer.clone(), cursor_anchor, false, cx);
        });

        // Wait a bit for prediction to come in
        cx.background_executor()
            .timer(Duration::from_millis(2000))
            .await;

        // Get suggestion
        let prediction = provider.update(cx, |provider, cx| {
            provider.suggest(&buffer, cursor_anchor, cx)
        })?;

        let duration = start.elapsed();

        let result = match prediction {
            Some(EditPrediction::Local { id, edits, .. }) => {
                let mut text_parts = Vec::new();
                let mut ranges = Vec::new();

                for (range, text) in &edits {
                    let start_point = range.start.to_point(&snapshot);
                    let end_point = range.end.to_point(&snapshot);
                    ranges.push(format!(
                        "({},{})..({},{})",
                        start_point.row, start_point.column, end_point.row, end_point.column
                    ));
                    text_parts.push(text.as_ref());
                }

                CompletionResult {
                    provider: "zed".to_string(),
                    completion_type: Some("Local".to_string()),
                    range: Some(ranges.join(", ")),
                    text: Some(text_parts.join("")),
                    jump_target: None,
                    supports_jump: true,
                    error: None,
                    duration,
                }
            }
            Some(EditPrediction::Jump { id, snapshot, target }) => {
                let target_point = target.to_point(&snapshot);
                let file_name = snapshot
                    .file()
                    .map(|f| {
                        // Note: We can't access cx here, so we'll use a placeholder
                        "different_file".to_string()
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                CompletionResult {
                    provider: "zed".to_string(),
                    completion_type: Some("Jump".to_string()),
                    range: None,
                    text: None,
                    jump_target: Some(format!(
                        "{}:({},{})",
                        file_name, target_point.row, target_point.column
                    )),
                    supports_jump: true,
                    error: None,
                    duration,
                }
            }
            None => CompletionResult {
                provider: "zed".to_string(),
                completion_type: None,
                range: None,
                text: None,
                jump_target: None,
                supports_jump: true,
                error: Some("No completion returned".to_string()),
                duration,
            },
        };

        Ok(result)
    })?;

    result
}

