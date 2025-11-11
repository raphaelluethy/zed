use crate::output::CompletionResult;
use crate::CommonArgs;
use anyhow::{Context as _, Result};
use copilot::Copilot;
use fs::RealFs;
use gpui::{App, AsyncApp, Entity};
use language::{Buffer, Point};
use node_runtime::RealNodeRuntime;
use project::{Project, ProjectPath, Worktree};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use util::rel_path::RelPath;

pub fn test_copilot(args: CommonArgs) -> Result<()> {
    let result = test_copilot_internal(args.clone())?;
    crate::output::print_single_result(&result, args.output_format)?;
    Ok(())
}

pub fn test_copilot_internal(args: CommonArgs) -> Result<CompletionResult> {
    let start = Instant::now();
    let app = gpui::App::new()?;
    let result = app.run(|cx| async move {
        let fs = Arc::new(RealFs::default());
        let http = Arc::new(reqwest_client::ReqwestClient::new());
        let node_runtime = RealNodeRuntime::new();
        let server_id = lsp::LanguageServerId(0);

        // Initialize copilot
        let copilot = cx.new(|cx| {
            Copilot::start(server_id, fs.clone(), node_runtime.clone(), cx);
            Copilot::global(cx).unwrap()
        });

        // Wait for copilot to be ready
        let mut status = copilot.read_with(cx, |copilot, _cx| copilot.status())?;
        let mut attempts = 0;
        while !status.is_authorized() && attempts < 50 {
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
            status = copilot.read_with(cx, |copilot, _cx| copilot.status())?;
            attempts += 1;
        }

        if !status.is_authorized() {
            return Ok(CompletionResult {
                provider: "copilot".to_string(),
                completion_type: None,
                range: None,
                text: None,
                jump_target: None,
                supports_jump: false,
                error: Some("Copilot not authorized or not ready".to_string()),
                duration: start.elapsed(),
            });
        }

        // Create project and open buffer
        let client = Arc::new(client::Client::production(cx.clone()));
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

        // Request completions
        let completions = copilot
            .update(cx, |copilot, cx| copilot.completions(&buffer, cursor_point, cx))?
            .await?;

        let duration = start.elapsed();

        if completions.is_empty() {
            return Ok(CompletionResult {
                provider: "copilot".to_string(),
                completion_type: None,
                range: None,
                text: None,
                jump_target: None,
                supports_jump: false,
                error: Some("No completions returned".to_string()),
                duration,
            });
        }

        // Use first completion
        let completion = &completions[0];
        let start_point = completion.range.start.to_point(&snapshot);
        let end_point = completion.range.end.to_point(&snapshot);

        Ok(CompletionResult {
            provider: "copilot".to_string(),
            completion_type: Some("Local".to_string()),
            range: Some(format!(
                "({},{})..({},{})",
                start_point.row, start_point.column, end_point.row, end_point.column
            )),
            text: Some(completion.text.clone()),
            jump_target: None,
            supports_jump: false,
            error: None,
            duration,
        })
    })?;

    result
}

