use serde::{Deserialize, Serialize};

#[derive(Deserialize, Default, Debug, Clone)]
pub struct AiFilterResponse {
    pub text: Option<String>,
    pub min_level: Option<String>,
    pub time_range: Option<String>,
}

/// Parse an AI response JSON string into an `AiFilterResponse`.
/// Handles markdown code fences (```json ... ```) around the JSON.
pub fn parse_ai_response(raw: &str) -> Result<AiFilterResponse, String> {
    let trimmed = raw.trim();

    // Strip markdown code fences if present
    let json_str = if trimmed.starts_with("```") {
        let without_opening = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        without_opening
            .strip_suffix("```")
            .unwrap_or(without_opening)
            .trim()
    } else {
        trimmed
    };

    serde_json::from_str(json_str).map_err(|e| format!("Failed to parse AI response: {e}"))
}

/// Build a system prompt describing the log file context for the AI model.
pub fn build_system_prompt(
    format_name: &str,
    field_names: &[String],
    time_range_desc: Option<&str>,
) -> String {
    let fields = if field_names.is_empty() {
        "none detected".to_string()
    } else {
        field_names.join(", ")
    };

    let mut prompt = format!(
        "You are a log analysis assistant. The user will describe what they want to find in their logs.\n\
         \n\
         Log format: {format_name}\n\
         Available fields: {fields}\n"
    );

    if let Some(range) = time_range_desc {
        prompt.push_str(&format!("Time range of log: {range}\n"));
    }

    prompt.push_str(
        "\n\
         Respond with a JSON object containing any combination of these optional fields:\n\
         - \"text\": a substring or regex to filter log lines\n\
         - \"min_level\": minimum log level (TRACE, DEBUG, INFO, WARN, ERROR, FATAL)\n\
         - \"time_range\": a time range expression (e.g. \"last_30m\", \"last_1h\")\n\
         \n\
         Only include fields that are relevant to the user's query. Respond with JSON only, no explanation.",
    );

    prompt
}

// ---------------------------------------------------------------------------
// AI backend: provider config and HTTP communication
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiProvider {
    Claude,
    OpenAi,
}

#[derive(Debug, Clone)]
pub struct AiConfig {
    pub provider: AiProvider,
    pub api_key: String,
    pub endpoint: String,
    pub model: String,
}

impl AiConfig {
    pub fn new(
        provider: AiProvider,
        api_key: String,
        endpoint: Option<String>,
        model: Option<String>,
    ) -> Self {
        let (default_endpoint, default_model) = match provider {
            AiProvider::Claude => (
                "https://api.anthropic.com".to_string(),
                "claude-haiku-4-5-20251001".to_string(),
            ),
            AiProvider::OpenAi => (
                "http://localhost:11434/v1".to_string(),
                "llama3.2".to_string(),
            ),
        };
        Self {
            provider,
            api_key,
            endpoint: endpoint.unwrap_or(default_endpoint),
            model: model.unwrap_or(default_model),
        }
    }
}

// -- Claude API request/response shapes ------------------------------------

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeMessage>,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    text: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
}

// -- OpenAI-compatible API request/response shapes -------------------------

#[derive(Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<OpenAiMessage>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiChoiceMessage,
}

#[derive(Deserialize)]
struct OpenAiChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

// -- Public query function -------------------------------------------------

/// Send a query to the configured AI provider and return the response text.
pub fn query_ai(
    config: &AiConfig,
    system_prompt: &str,
    user_query: &str,
) -> Result<String, String> {
    let client = reqwest::blocking::Client::new();

    match config.provider {
        AiProvider::Claude => query_claude(&client, config, system_prompt, user_query),
        AiProvider::OpenAi => query_openai(&client, config, system_prompt, user_query),
    }
}

fn query_claude(
    client: &reqwest::blocking::Client,
    config: &AiConfig,
    system_prompt: &str,
    user_query: &str,
) -> Result<String, String> {
    let url = format!("{}/v1/messages", config.endpoint);
    let body = ClaudeRequest {
        model: config.model.clone(),
        max_tokens: 256,
        system: system_prompt.to_string(),
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: user_query.to_string(),
        }],
    };

    let resp = client
        .post(&url)
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(format!("Claude API error ({status}): {text}"));
    }

    let parsed: ClaudeResponse = resp
        .json()
        .map_err(|e| format!("Failed to parse Claude response: {e}"))?;

    parsed
        .content
        .first()
        .map(|block| block.text.clone())
        .ok_or_else(|| "Empty response from Claude".to_string())
}

fn query_openai(
    client: &reqwest::blocking::Client,
    config: &AiConfig,
    system_prompt: &str,
    user_query: &str,
) -> Result<String, String> {
    let url = format!("{}/chat/completions", config.endpoint);
    let body = OpenAiRequest {
        model: config.model.clone(),
        max_tokens: 256,
        messages: vec![
            OpenAiMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            OpenAiMessage {
                role: "user".to_string(),
                content: user_query.to_string(),
            },
        ],
    };

    let mut req = client.post(&url).header("content-type", "application/json");

    if !config.api_key.is_empty() {
        req = req.header("authorization", format!("Bearer {}", config.api_key));
    }

    let resp = req
        .json(&body)
        .send()
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(format!("OpenAI API error ({status}): {text}"));
    }

    let parsed: OpenAiResponse = resp
        .json()
        .map_err(|e| format!("Failed to parse OpenAI response: {e}"))?;

    parsed
        .choices
        .first()
        .map(|choice| choice.message.content.clone())
        .ok_or_else(|| "Empty response from OpenAI".to_string())
}
