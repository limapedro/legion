//! Typed configuration. No global mutable config, no flag parser sprawl.

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub provider: ProviderSettings,
    pub agent: AgentSettings,
    pub swarm: SwarmSettings,
    pub ui: UiSettings,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProviderSettings {
    pub kind: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub reasoning_effort: String,
    pub request_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AgentSettings {
    pub max_tokens: u32,
    pub temperature: f32,
    pub max_history_messages: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SwarmSettings {
    pub max_workers: usize,
    pub max_model_requests: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    pub theme: String,
    pub reduced_motion: bool,
    pub no_color: bool,
    pub no_unicode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            provider: ProviderSettings::default(),
            agent: AgentSettings::default(),
            swarm: SwarmSettings::default(),
            ui: UiSettings::default(),
        }
    }
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            kind: "openai-compatible".into(),
            base_url: "http://localhost:1234/v1".into(),
            model: "qwen3.5-2b".into(),
            api_key: String::new(),
            // exp_001: reasoning_effort "none" is what makes Qwen3.5 emit
            // answer text instead of consuming max_tokens on thinking.
            reasoning_effort: "none".into(),
            request_timeout_secs: 120,
        }
    }
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            temperature: 0.0,
            max_history_messages: 40,
        }
    }
}

impl Default for SwarmSettings {
    fn default() -> Self {
        Self {
            max_workers: 3,
            max_model_requests: 2,
        }
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: "amber-cyan".into(),
            reduced_motion: false,
            no_color: false,
            no_unicode: false,
        }
    }
}

impl Settings {
    /// Load `legion.toml` if present; malformed config fails loudly rather than
    /// silently falling back to defaults.
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("invalid {}: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_use_the_measured_dev_model() {
        let s = Settings::default();
        assert_eq!(s.provider.model, "qwen3.5-2b");
        assert_eq!(s.provider.reasoning_effort, "none");
        assert_eq!(s.swarm.max_workers, 3);
        assert_eq!(s.swarm.max_model_requests, 2);
    }

    #[test]
    fn partial_toml_merges_with_defaults() {
        let parsed: Settings = toml::from_str("[provider]\nmodel = 'mock-small'\n").unwrap();
        assert_eq!(parsed.provider.model, "mock-small");
        assert_eq!(parsed.provider.base_url, "http://localhost:1234/v1");
        assert_eq!(parsed.agent.max_tokens, 512);
    }
}
