//! Provider-agnostic model interface. Nothing above this module knows about
//! LM Studio, OpenAI wire formats, or Qwen3.5 reasoning tokens.

use futures_util::Stream;
use serde::Serialize;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

pub type ProviderStream = Pin<Box<dyn Stream<Item = Result<Delta, ModelError>> + Send>>;

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    fn new(role: &str, text: impl Into<String>) -> Self {
        Self { role: role.into(), content: text.into(), tool_call_id: None }
    }
    pub fn user(text: impl Into<String>) -> Self {
        Self::new("user", text)
    }
    pub fn assistant(text: impl Into<String>) -> Self {
        Self::new("assistant", text)
    }
    pub fn tool(tool_call_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: text.into(),
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolSpec {
    pub r#type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    pub max_tokens: u32,
    pub temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolSpec>,
}

impl ChatRequest {
    pub fn new(model: impl Into<String>, messages: Vec<Message>, max_tokens: u32) -> Self {
        Self {
            model: model.into(),
            messages,
            stream: true,
            max_tokens,
            temperature: 0.0,
            reasoning_effort: None,
            tools: Vec::new(),
        }
    }
}

/// A single incremental event from a model. Reasoning is kept separate from
/// answer text so the HUD can show a "thinking" state (exp_001 finding).
#[derive(Debug, Clone, PartialEq)]
pub enum Delta {
    Text(String),
    Reasoning(String),
    ToolCall {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments: String,
    },
    Usage {
        prompt_tokens: Option<u64>,
        completion_tokens: Option<u64>,
    },
    Error(String),
}

#[derive(Debug)]
pub enum ModelError {
    Transport(String),
    Http(String),
    Protocol(String),
    Cancelled,
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "transport error: {e}"),
            Self::Http(e) => write!(f, "http error: {e}"),
            Self::Protocol(e) => write!(f, "protocol error: {e}"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::error::Error for ModelError {}

/// Implemented by LM Studio / any OpenAI-compatible endpoint, and by the
/// deterministic mock model used in tests.
pub trait ModelProvider: Send + Sync {
    fn stream_chat(
        &self,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<ProviderStream, ModelError>> + Send + '_>>;
}
