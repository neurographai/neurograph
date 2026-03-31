// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Interactive chat REPL for the terminal.

use super::history::ConversationHistory;
use super::{ChatConfig, ChatResponse};
use std::io::{self, Write};

/// Run the interactive chat REPL.
pub async fn run_repl(
    config: ChatConfig,
    answer_fn: impl for<'a> Fn(&'a str, &'a ConversationHistory) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<ChatResponse>> + Send + 'a>>,
    paper_count: usize,
    entity_count: usize,
) -> anyhow::Result<()> {
    let mut history = ConversationHistory::new();

    println!();
    println!("  ╔══════════════════════════════════════════════════╗");
    println!("  ║  NeuroGraph Chat — {} papers, {} entities       ║", paper_count, entity_count);
    println!("  ╚══════════════════════════════════════════════════╝");
    println!();
    println!("  Model: {}:{}", config.provider, config.model);
    println!("  Commands: /help, /clear, /sources, /history, /quit");
    println!();

    loop {
        print!("  You: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() { continue; }

        match input {
            "/quit" | "/exit" | "/q" => { println!("  Goodbye!"); break; }
            "/help" | "/h" => { print_help(); continue; }
            "/clear" => { history.clear(); println!("  Conversation cleared."); continue; }
            "/history" => { print_history(&history); continue; }
            "/sources" => { print_last_sources(&history); continue; }
            cmd if cmd.starts_with('/') => {
                println!("  Unknown command '{}'. Type /help.", cmd);
                continue;
            }
            _ => {}
        }

        history.add_user_message(input);
        print!("\n  NeuroGraph: ");
        io::stdout().flush()?;

        match answer_fn(input, &history).await {
            Ok(response) => {
                println!("{}", response.answer);
                if !response.sources.is_empty() {
                    println!("\n  📄 Sources:");
                    for (i, source) in response.sources.iter().enumerate() {
                        println!("    [{}] \"{}\" — {} (p.{})", i + 1, source.paper_title, source.section, source.page);
                    }
                }
                println!("\n  ⏱  {}ms | {} tokens | model: {}", response.thinking_time_ms, response.tokens_used, response.model);
                history.add_assistant_message(&response.answer, response.sources);
            }
            Err(e) => {
                println!("Error: {}", e);
                println!("\n  Make sure your LLM is running:");
                println!("    Ollama: ollama serve && ollama pull {}", config.model);
            }
        }
        println!();
    }

    Ok(())
}

fn print_help() {
    println!("\n  Available commands:");
    println!("    /help     — Show this help message");
    println!("    /clear    — Clear conversation history");
    println!("    /sources  — Show sources from last response");
    println!("    /history  — Show conversation history");
    println!("    /quit     — Exit chat\n");
}

fn print_history(history: &ConversationHistory) {
    println!();
    if history.is_empty() {
        println!("  No conversation history.");
    } else {
        for msg in history.all() {
            let role = match msg.role { super::Role::User => "You", super::Role::Assistant => "NG", super::Role::System => "Sys" };
            let preview: String = msg.content.chars().take(80).collect();
            println!("  [{}] {}: {}", msg.timestamp.format("%H:%M"), role, preview);
        }
    }
    println!();
}

fn print_last_sources(history: &ConversationHistory) {
    println!();
    let last_assistant = history.all().iter().rev().find(|m| m.role == super::Role::Assistant);
    match last_assistant {
        Some(msg) if !msg.sources.is_empty() => {
            println!("  Sources from last response:");
            for (i, source) in msg.sources.iter().enumerate() {
                println!("    [{}] \"{}\"", i + 1, source.paper_title);
                println!("        Section: {}, Page: {}", source.section, source.page);
                let preview: String = source.chunk_text.chars().take(100).collect();
                println!("        Preview: {}...\n", preview);
            }
        }
        _ => { println!("  No sources available. Ask a question first."); }
    }
    println!();
}
