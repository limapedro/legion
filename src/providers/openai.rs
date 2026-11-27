//! OpenAI-compatible streaming provider. Works with LM Studio's local server
//! and any endpoint speaking Chat Completions + SSE.

use crate::model::{ChatRequest, Delta, ModelError, ModelProvider, ProviderStream};
use crate::sse::SseDecoder;
use bytes::Bytes;
use futures_util::{Stream, StreamExt, stream};
use serde::Deserialize;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub struct OpenAiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl OpenAiProvider {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>, timeout: Duration) -> Self {
        Self {
            client: reqwest::Client::builder().timeout(timeout).build().unwrap_or_else(|_| reqwest::Client::new()),
            base_url: base_url.into(),
            api_key: api_key.into(),
        }
    }
}

#[derive(Deserialize)]
struct ChatChunk {
    #[serde(default)]
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<UsageDto>,
}

#[derive(Deserialize)]
struct Choice {
    #[serde(default)]
    delta: DeltaDto,
}

#[derive(Deserialize, Default)]
struct DeltaDto {
    content: Option<String>,
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<ToolCallDto>>,
}

#[derive(Deserialize)]
struct ToolCallDto {
    index: usize,
    id: Option<String>,
    function: Option<FunctionDto>,
}

#[derive(Deserialize)]
struct FunctionDto {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct UsageDto {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
}

struct SseState {
    stream: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
    decoder: SseDecoder,
    pending: VecDeque<Delta>,
    cancel: CancellationToken,
    debug: bool,
}

fn chunk_to_deltas(chunk: ChatChunk, out: &mut VecDeque<Delta>) {
    for choice in chunk.choices {
        if let Some(text) = choice.delta.content {
            if !text.is_empty() {
                out.push_back(Delta::Text(text));
            }
        }
        if let Some(reasoning) = choice.delta.reasoning_content {
            if !reasoning.is_empty() {
                out.push_back(Delta::Reasoning(reasoning));
            }
        }
        for call in choice.delta.tool_calls.unwrap_or_default() {
            out.push_back(Delta::ToolCall {
                index: call.index,
                id: call.id,
                name: call.function.as_ref().and_then(|f| f.name.clone()),
                arguments: call.function.and_then(|f| f.arguments).unwrap_or_default(),
            });
        }
    }
    if let Some(usage) = chunk.usage {
        out.push_back(Delta::Usage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
        });
    }
}

impl ModelProvider for OpenAiProvider {
    fn stream_chat(
        &self,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<ProviderStream, ModelError>> + Send + '_>> {
        Box::pin(async move {
            let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
            let mut req = self.client.post(url).json(&request);
            if !self.api_key.is_empty() {
                req = req.bearer_auth(&self.api_key);
            }

            let response = req.send()
                .await
                .map_err(|e| ModelError::Transport(format!("{e:?}")))?;
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(ModelError::Http(format!("{status}: {body}")));
            }

            let debug = std::env::var_os("LEGION_DEBUG").is_some();
            if debug {
                eprintln!("[debug] status={} content-type={:?}", response.status(), response.headers().get("content-type"));
            }
            let state = SseState {
                stream: Box::pin(response.bytes_stream()),
                decoder: SseDecoder::new(),
                pending: VecDeque::new(),
                cancel,
                debug,
            };

            let stream = stream::unfold(state, |mut st| async move {
                loop {
                    if st.cancel.is_cancelled() {
                        return None;
                    }
                    if let Some(delta) = st.pending.pop_front() {
                        return Some((Ok(delta), st));
                    }
                    let next = tokio::select! {
                        _ = st.cancel.cancelled() => return None,
                        item = st.stream.next() => item,
                    };
                    match next {
                        Some(Ok(bytes)) => {
                            for data in st.decoder.push(&bytes) {
                                if data.trim() == "[DONE]" {
                                    return None;
                                }
                                if st.debug {
                                    eprintln!("[debug] data={}", &data[..data.len().min(160)]);
                                }
                                match serde_json::from_str::<ChatChunk>(&data) {
                                    Ok(chunk) => chunk_to_deltas(chunk, &mut st.pending),
                                    Err(e) => st.pending.push_back(Delta::Error(format!(
                                        "bad SSE frame: {e}"
                                    ))),
                                }
                            }
                        }
                        Some(Err(e)) => {
                            return Some((Err(ModelError::Transport(e.to_string())), st));
                        }
                        None => return None,
                    }
                }
            });

            Ok(Box::pin(stream) as ProviderStream)
        })
    }
}
