use crate::CommonArgs;
use anyhow::Result;
use serde_json::json;
use std::time::Duration;

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum OutputFormat {
    Human,
    Json,
    Table,
}

#[derive(Clone)]
pub struct CompletionResult {
    pub provider: String,
    pub completion_type: Option<String>,
    pub range: Option<String>,
    pub text: Option<String>,
    pub jump_target: Option<String>,
    pub supports_jump: bool,
    pub error: Option<String>,
    pub duration: Duration,
}

pub fn print_comparison(
    results: &[(&str, Result<CompletionResult>, Duration)],
    show_diff: bool,
) -> Result<()> {
    println!("\n=== Completion Comparison ===\n");

    // Print table header
    println!("{:<12} | {:<10} | {:<20} | {:<30} | {:<10}", 
        "Provider", "Type", "Range", "Text Preview", "Jump");
    println!("{}", "-".repeat(100));

    for (provider, result, duration) in results {
        match result {
            Ok(completion) => {
                let text_preview = completion
                    .text
                    .as_ref()
                    .map(|t| {
                        if t.len() > 30 {
                            format!("{}...", &t[..27])
                        } else {
                            t.clone()
                        }
                    })
                    .unwrap_or_else(|| "N/A".to_string());

                let range_str = completion.range.as_ref().unwrap_or(&"N/A".to_string());
                let type_str = completion
                    .completion_type
                    .as_ref()
                    .unwrap_or(&"None".to_string());
                let jump_str = if completion.supports_jump {
                    "Yes"
                } else {
                    "No"
                };

                println!(
                    "{:<12} | {:<10} | {:<20} | {:<30} | {:<10}",
                    provider, type_str, range_str, text_preview, jump_str
                );

                if let Some(jump_target) = &completion.jump_target {
                    println!("  └─ Jump target: {}", jump_target);
                }
            }
            Err(e) => {
                println!(
                    "{:<12} | {:<10} | {:<20} | {:<30} | {:<10}",
                    provider, "Error", "N/A", format!("Error: {}", e), "N/A"
                );
            }
        }
    }

    println!("\nResponse times:");
    for (provider, _, duration) in results {
        println!("  {}: {:?}", provider, duration);
    }

    if show_diff {
        print_differences(results)?;
    }

    Ok(())
}

fn print_differences(results: &[(&str, Result<CompletionResult>, Duration)]) -> Result<()> {
    println!("\n=== Differences ===");

    let mut successful_results: Vec<_> = results
        .iter()
        .filter_map(|(provider, result, _)| {
            result.as_ref().ok().map(|r| (*provider, r))
        })
        .collect();

    if successful_results.len() < 2 {
        println!("Need at least 2 successful results to compare");
        return Ok(());
    }

    // Compare completion types
    let types: Vec<_> = successful_results
        .iter()
        .map(|(_, r)| r.completion_type.as_deref().unwrap_or("None"))
        .collect();
    if types.iter().any(|t| *t != types[0]) {
        println!("\nCompletion types differ:");
        for (provider, result) in &successful_results {
            println!(
                "  {}: {}",
                provider,
                result.completion_type.as_deref().unwrap_or("None")
            );
        }
    }

    // Compare text content
    let texts: Vec<_> = successful_results
        .iter()
        .map(|(_, r)| r.text.as_deref().unwrap_or(""))
        .collect();
    if texts.iter().any(|t| *t != texts[0]) {
        println!("\nText content differs:");
        for (provider, result) in &successful_results {
            if let Some(text) = &result.text {
                println!("  {}: {}", provider, text);
            }
        }
    }

    Ok(())
}

pub fn print_comparison_json(
    results: &[(&str, Result<CompletionResult>, Duration)],
) -> Result<()> {
    let mut json_results = Vec::new();

    for (provider, result, duration) in results {
        match result {
            Ok(completion) => {
                json_results.push(json!({
                    "provider": provider,
                    "success": true,
                    "completion_type": completion.completion_type,
                    "range": completion.range,
                    "text": completion.text,
                    "jump_target": completion.jump_target,
                    "supports_jump": completion.supports_jump,
                    "duration_ms": duration.as_millis(),
                }));
            }
            Err(e) => {
                json_results.push(json!({
                    "provider": provider,
                    "success": false,
                    "error": e.to_string(),
                    "duration_ms": duration.as_millis(),
                }));
            }
        }
    }

    println!("{}", serde_json::to_string_pretty(&json_results)?);
    Ok(())
}

pub fn print_single_result(result: &CompletionResult, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Human => {
            println!("\n=== {} Completion Result ===", result.provider);
            println!("Type: {}", result.completion_type.as_deref().unwrap_or("None"));
            if let Some(range) = &result.range {
                println!("Range: {}", range);
            }
            if let Some(text) = &result.text {
                println!("Text: {}", text);
            }
            if let Some(jump_target) = &result.jump_target {
                println!("Jump Target: {}", jump_target);
            }
            println!("Supports Jump: {}", result.supports_jump);
            println!("Duration: {:?}", result.duration);
            if let Some(error) = &result.error {
                println!("Error: {}", error);
            }
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "provider": result.provider,
                    "completion_type": result.completion_type,
                    "range": result.range,
                    "text": result.text,
                    "jump_target": result.jump_target,
                    "supports_jump": result.supports_jump,
                    "duration_ms": result.duration.as_millis(),
                    "error": result.error,
                }))?
            );
        }
        OutputFormat::Table => {
            print_comparison(&[(result.provider.as_str(), Ok(result.clone()), result.duration)], false)?;
        }
    }
    Ok(())
}

