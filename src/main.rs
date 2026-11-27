//! Legion M1: minimal harness. Plain line UI, streaming output, typed config.

mod agent;
mod config;
mod model;
mod providers;
mod sse;

use config::Settings;
use model::{ChatRequest, Delta, Message, ModelProvider};
use providers::mock::MockProvider;
use providers::openai::OpenAiProvider;
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::load(Path::new("legion.toml"))?;
    let openai = OpenAiProvider::new(
        &settings.provider.base_url,
        &settings.provider.api_key,
        Duration::from_secs(settings.provider.request_timeout_secs),
    );
    let mock = MockProvider::echo();
    let provider: &dyn ModelProvider = if settings.provider.kind == "mock" {
        &mock
    } else {
        &openai
    };

    println!("LEGION :: ambient assistant");
    println!("model: {}   endpoint: {}", settings.provider.model, settings.provider.base_url);
    println!("(M1 plain UI - type 'exit' to quit)");

    let mut history: Vec<Message> = Vec::new();
    let mut ctrl_c_task: Option<tokio::task::JoinHandle<()>> = None;
    let stdin = io::stdin();
    loop {
        if let Some(task) = ctrl_c_task.take() {
            task.abort();
        }
        let cancel = CancellationToken::new();
        let ctrl_c_cancel = cancel.clone();
        ctrl_c_task = Some(tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            ctrl_c_cancel.cancel();
        }));

        print!("legion > ");
        io::stdout().flush()?;
        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.eq_ignore_ascii_case("exit") || line.eq_ignore_ascii_case("quit") {
            break;
        }

        agent::push_history(&mut history, Message::user(line), settings.agent.max_history_messages);
        let mut request = ChatRequest::new(
            &settings.provider.model,
            history.clone(),
            settings.agent.max_tokens,
        );
        request.temperature = settings.agent.temperature;
        if !settings.provider.reasoning_effort.is_empty() {
            request.reasoning_effort = Some(settings.provider.reasoning_effort.clone());
        }

        println!();
        let outcome = agent::run_turn(provider, &request, cancel.clone(), |delta| match delta {
            Delta::Text(text) => {
                print!("{text}");
                let _ = io::stdout().flush();
            }
            Delta::Reasoning(text) => {
                print!("[thinking] {text}");
                let _ = io::stdout().flush();
            }
            Delta::ToolCall { name, arguments, .. } => {
                println!("\n> tool call: {}({})", name.as_deref().unwrap_or("?"), arguments);
            }
            Delta::Usage { .. } => {}
            Delta::Error(error) => eprintln!("\n! {error}"),
        })
        .await;

        match outcome {
            Ok(outcome) => {
                println!();
                if !outcome.tool_calls.is_empty() {
                    println!("[M1] {} tool call(s) recorded; execution arrives in M2.", outcome.tool_calls.len());
                }
                if let (Some(p), Some(c)) = (outcome.prompt_tokens, outcome.completion_tokens) {
                    println!("[tokens] prompt {p} + completion {c}");
                }
                if !outcome.text.is_empty() {
                    agent::push_history(
                        &mut history,
                        Message::assistant(outcome.text),
                        settings.agent.max_history_messages,
                    );
                }
            }
            Err(error) => eprintln!("model error: {error}"),
        }
    }

    println!("bye.");
    Ok(())
}
