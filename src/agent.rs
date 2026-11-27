//! The agent loop: request -> streamed deltas -> text/tool-call outcome.
//! M2 adds tool execution to the outcome; the loop itself stays provider-agnostic.

use crate::model::{ChatRequest, Delta, Message, ModelError, ModelProvider};
use futures_util::StreamExt;
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ToolCall {
    pub index: usize,
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Default, PartialEq)]
pub struct TurnOutcome {
    pub text: String,
    pub reasoning_chars: usize,
    pub tool_calls: Vec<ToolCall>,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub errors: Vec<String>,
}

pub async fn run_turn(
    provider: &dyn ModelProvider,
    request: &ChatRequest,
    cancel: CancellationToken,
    mut on_delta: impl FnMut(&Delta),
) -> Result<TurnOutcome, ModelError> {
    let mut stream = provider.stream_chat(request.clone(), cancel).await?;
    let mut outcome = TurnOutcome::default();
    let mut calls: BTreeMap<usize, ToolCall> = BTreeMap::new();

    while let Some(item) = stream.next().await {
        let delta = item?;
        on_delta(&delta);
        match delta {
            Delta::Text(text) => outcome.text.push_str(&text),
            Delta::Reasoning(text) => outcome.reasoning_chars += text.chars().count(),
            Delta::ToolCall { index, id, name, arguments } => {
                let call = calls.entry(index).or_insert_with(|| ToolCall { index, ..Default::default() });
                if let Some(id) = id {
                    call.id = id;
                }
                if let Some(name) = name {
                    call.name = name;
                }
                call.arguments.push_str(&arguments);
            }
            Delta::Usage { prompt_tokens, completion_tokens } => {
                outcome.prompt_tokens = prompt_tokens;
                outcome.completion_tokens = completion_tokens;
            }
            Delta::Error(message) => outcome.errors.push(message),
        }
    }

    outcome.tool_calls = calls.into_values().collect();
    Ok(outcome)
}

/// Keep a bounded conversation window. Typed and testable; no hidden state.
pub fn push_history(history: &mut Vec<Message>, message: Message, max: usize) {
    let max = max.max(1);
    history.push(message);
    if history.len() > max {
        let excess = history.len() - max;
        history.drain(0..excess);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChatRequest, Delta, Message, ModelError};
    use crate::providers::mock::MockProvider;

    #[tokio::test]
    async fn assembles_streamed_text_and_tool_calls() {
        let script = vec![
            Delta::Text("Hel".into()),
            Delta::Reasoning("thinking".into()),
            Delta::Text("lo".into()),
            Delta::ToolCall { index: 0, id: Some("call-1".into()), name: Some("read_file".into()), arguments: "{\"path\":".into() },
            Delta::ToolCall { index: 0, id: None, name: None, arguments: "\"a.txt\"}".into() },
            Delta::Usage { prompt_tokens: Some(12), completion_tokens: Some(7) },
        ];
        let provider = MockProvider::new(script);
        let request = ChatRequest::new("mock", vec![Message::user("hi")], 64);
        let mut seen = Vec::new();
        let outcome = run_turn(&provider, &request, CancellationToken::new(), |d| seen.push(d.clone()))
            .await
            .unwrap();

        assert_eq!(outcome.text, "Hello");
        assert_eq!(outcome.reasoning_chars, 8);
        assert_eq!(outcome.prompt_tokens, Some(12));
        assert_eq!(outcome.completion_tokens, Some(7));
        assert_eq!(outcome.tool_calls.len(), 1);
        assert_eq!(outcome.tool_calls[0].id, "call-1");
        assert_eq!(outcome.tool_calls[0].name, "read_file");
        assert_eq!(outcome.tool_calls[0].arguments, "{\"path\":\"a.txt\"}");
        assert_eq!(seen.len(), 6);
    }

    #[tokio::test]
    async fn cancellation_stops_the_stream() {
        let mut script = Vec::new();
        for i in 0..100 {
            script.push(Delta::Text(format!("{i}")));
        }
        let provider = MockProvider::new(script);
        let request = ChatRequest::new("mock", vec![Message::user("hi")], 64);
        let cancel = CancellationToken::new();
        cancel.cancel();
        let outcome = run_turn(&provider, &request, cancel, |_| {}).await.unwrap();
        assert!(outcome.text.is_empty());
    }

    #[test]
    fn history_is_bounded_and_tool_messages_keep_their_id() {
        let mut history = vec![Message::user("1"), Message::user("2"), Message::user("3")];
        push_history(&mut history, Message::assistant("4"), 2);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, "3");
        assert_eq!(history[1].content, "4");

        let tool = Message::tool("call-1", "hello");
        assert_eq!(tool.role, "tool");
        assert_eq!(tool.tool_call_id.as_deref(), Some("call-1"));
        assert_eq!(tool.content, "hello");
        assert!(ModelError::Protocol("bad".into()).to_string().contains("bad"));
        assert!(ModelError::Cancelled.to_string().contains("cancelled"));
    }
}
