//! Minimal incremental Server-Sent Events decoder for OpenAI-compatible
//! `text/event-stream` responses. Testable without a network.

#[derive(Debug, Default)]
pub struct SseDecoder {
    buf: String,
}

impl SseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed an arbitrary byte chunk (may split a line in half). Returns every
    /// complete `data:` payload found so far, in order.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buf.push_str(&String::from_utf8_lossy(chunk));
        let mut payloads = Vec::new();
        while let Some(pos) = self.buf.find('\n') {
            let line = self.buf[..pos].trim_end_matches('\r').to_string();
            self.buf = self.buf[pos + 1..].to_string();
            if let Some(data) = line.strip_prefix("data:") {
                payloads.push(data.trim_start().to_string());
            }
        }
        payloads
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_complete_lines() {
        let mut d = SseDecoder::new();
        let out = d.push(b"data: {\"a\":1}\n\ndata: [DONE]\n\n");
        assert_eq!(out, vec!["{\"a\":1}", "[DONE]"]);
    }

    #[test]
    fn survives_chunks_split_mid_line() {
        let mut d = SseDecoder::new();
        assert!(d.push(b"data: hel").is_empty());
        assert_eq!(d.push(b"lo\n\ndata: [DO"), vec!["hello"]);
        assert_eq!(d.push(b"NE]\n"), vec!["[DONE]"]);
    }

    #[test]
    fn ignores_comments_and_other_fields() {
        let mut d = SseDecoder::new();
        let out = d.push(b": keepalive\nevent: message\ndata: x\n\n");
        assert_eq!(out, vec!["x"]);
    }
}
