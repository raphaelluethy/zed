use crate::output::CompletionResult;
use crate::CommonArgs;
use anyhow::{Context as _, Result};
use client::Client;
use futures::StreamExt;
use gpui::{App, AsyncApp, Entity};
use language::{Anchor, Buffer, Point};
use project::{Project, ProjectPath, Worktree};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use supermaven::Supermaven;
use util::rel_path::RelPath;

pub fn test_supermaven(args: CommonArgs) -> Result<()> {
    let result = test_supermaven_internal(args.clone())?;
    crate::output::print_single_result(&result, args.output_format)?;
    Ok(())
}

pub fn test_supermaven_internal(args: CommonArgs) -> Result<CompletionResult> {
    let start = Instant::now();
    let app = gpui::App::new()?;
    let result = app.run(|cx| async move {
        let client = Arc::new(Client::production(cx.clone()));
        let http = Arc::new(reqwest_client::ReqwestClient::new());
        let fs = Arc::new(::fs::RealFs::default());
        let node_runtime = node_runtime::RealNodeRuntime::new();

        // Initialize supermaven
        let supermaven = cx.new(|_cx| Supermaven::Starting);
        Supermaven::set_global(supermaven.clone(), cx);
        supermaven.update(cx, |supermaven, cx| {
            supermaven.start(client.clone(), cx);
        });

        // Wait for supermaven to be ready
        let mut is_enabled = supermaven.read_with(cx, |supermaven, _cx| supermaven.is_enabled())?;
        let mut attempts = 0;
        while !is_enabled && attempts < 50 {
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
            is_enabled = supermaven.read_with(cx, |supermaven, _cx| supermaven.is_enabled())?;
            attempts += 1;
        }

        if !is_enabled {
            return Ok(CompletionResult {
                provider: "supermaven".to_string(),
                completion_type: None,
                range: None,
                text: None,
                jump_target: None,
                supports_jump: false,
                error: Some("Supermaven not enabled or not ready".to_string()),
                duration: start.elapsed(),
            });
        }

        // Create project and open buffer
        let user_store = cx.new(|cx| client::UserStore::global(client.clone(), cx));
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

        // Request completion
        let mut completion = supermaven
            .update(cx, |supermaven, cx| supermaven.complete(&buffer, cursor_anchor, cx))
            .ok_or_else(|| anyhow::anyhow!("Failed to request completion"))?;

        // Wait for completion updates
        let mut completion_text = String::new();
        let timeout = cx.background_executor().timer(Duration::from_secs(5));
        let mut updates = completion.updates;

        loop {
            futures::select! {
                update = updates.next() => {
                    match update {
                        Some(()) => {
                            // Check for completion text
                            if let Some(text) = supermaven.read_with(cx, |supermaven, cx| {
                                supermaven.completion(&buffer, cursor_anchor, cx)
                            })? {
                                completion_text = text.to_string();
                            }
                        }
                        None => break,
                    }
                }
                _ = timeout.fuse() => {
                    break;
                }
            }
        }

        let duration = start.elapsed();

        if completion_text.is_empty() {
            return Ok(CompletionResult {
                provider: "supermaven".to_string(),
                completion_type: None,
                range: None,
                text: None,
                jump_target: None,
                supports_jump: false,
                error: Some("No completion text returned".to_string()),
                duration,
            });
        }

        // Calculate range (from cursor to end of line)
        let cursor_point = cursor_anchor.to_point(&snapshot);
        let end_of_line = snapshot.anchor_after(language::Point::new(
            cursor_point.row,
            snapshot.line_len(cursor_point.row),
        ));

        Ok(CompletionResult {
            provider: "supermaven".to_string(),
            completion_type: Some("Local".to_string()),
            range: Some(format!(
                "({},{})..({},{})",
                cursor_point.row,
                cursor_point.column,
                end_of_line.to_point(&snapshot).row,
                end_of_line.to_point(&snapshot).column
            )),
            text: Some(completion_text),
            jump_target: None,
            supports_jump: false,
            error: None,
            duration,
        })
    })?;

    result
}

