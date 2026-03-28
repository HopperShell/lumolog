use serde::{Deserialize, Serialize};

#[derive(Deserialize, Default, Debug, Clone)]
pub struct AiFilterResponse {
    pub text: Option<String>,
    pub min_level: Option<String>,
    pub time_range: Option<String>,
}

/// Parse an AI response JSON string into an `AiFilterResponse`.
/// Handles markdown code fences, leading/trailing text, and extracts
/// the first JSON object found in the response.
pub fn parse_ai_response(raw: &str) -> Result<AiFilterResponse, String> {
    let trimmed = raw.trim();

    // Strip markdown code fences if present
    let stripped = if trimmed.starts_with("```") {
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

    // Try direct parse first
    if let Ok(resp) = serde_json::from_str::<AiFilterResponse>(stripped) {
        return Ok(resp);
    }

    // Fallback: find the first { ... } block in the response
    if let Some(start) = stripped.find('{')
        && let Some(end) = stripped.rfind('}')
    {
        let json_str = &stripped[start..=end];
        if let Ok(resp) = serde_json::from_str::<AiFilterResponse>(json_str) {
            return Ok(resp);
        }
    }

    Err(format!(
        "Failed to parse AI response — no valid JSON found in: {}",
        &raw[..raw.len().min(100)]
    ))
}

/// Build a system prompt describing the log file context for the AI model.
pub fn build_system_prompt(
    format_name: &str,
    field_names: &[String],
    time_range_desc: Option<&str>,
    sample_lines: &[String],
) -> String {
    let fields = if field_names.is_empty() {
        "none detected".to_string()
    } else {
        field_names.join(", ")
    };

    let mut prompt = format!(
        "You are a log filter assistant. The user will describe what log lines they want to see. \
         Respond with ONLY a JSON object containing filter criteria.\n\
         \n\
         Log format: {format_name}\n\
         Available fields: {fields}\n"
    );

    if let Some(range) = time_range_desc {
        prompt.push_str(&format!("Time range of log: {range}\n"));
    }

    if !sample_lines.is_empty() {
        prompt.push_str("\nHere are sample lines from the log so you can see the actual content and vocabulary:\n\n");
        for line in sample_lines {
            prompt.push_str(line);
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    prompt.push_str(
        "Available filters:\n\
         - \"text\": a substring to search for in log lines (case-insensitive). Pick a short, \
         specific substring that actually appears in the sample lines above.\n\
         - \"min_level\": minimum log level (TRACE, DEBUG, INFO, WARN, ERROR, FATAL)\n\
         - \"time_range\": time window: \"last_Xm\" for minutes, \"last_Xh\" for hours, \"last_Xd\" for days\n\
         \n\
         IMPORTANT: The \"text\" filter does substring matching. Choose a word or phrase that literally \
         appears in the log messages. Look at the sample lines to find the right term.\n\
         \n\
         Omit filters the user didn't mention. Respond with JSON only, no explanation.",
    );

    prompt
}

/// Build prompts for analyze mode — the AI reads actual log content and answers a question.
/// Returns (system_prompt, user_message).
pub fn build_analyze_prompt(user_question: &str, log_lines: &[String]) -> (String, String) {
    let system = "You are a log analysis expert. The user will show you log lines and ask a question.\n\
        Analyze the logs carefully and provide a clear, concise response. Focus on:\n\
        - Patterns and trends you observe\n\
        - Errors, anomalies, or concerning behavior\n\
        - Correlations between events\n\
        - Potential root causes if errors are present\n\
        - A brief summary of what the logs show\n\n\
        Be specific — reference actual log content, timestamps, and error messages.\n\
        Keep your response concise and actionable."
        .to_string();

    let mut user_msg = format!("Here are {} log lines:\n\n", log_lines.len());
    for line in log_lines {
        user_msg.push_str(line);
        user_msg.push('\n');
    }
    user_msg.push_str(&format!("\nQuestion: {user_question}"));

    (system, user_msg)
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
    #[serde(default)]
    content: Option<String>,
    /// Some "thinking" models put the answer here when content is empty
    #[serde(default)]
    reasoning_content: Option<String>,
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
        max_tokens: 2048,
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
        max_tokens: 2048,
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

    let msg = parsed
        .choices
        .first()
        .map(|choice| &choice.message)
        .ok_or_else(|| "Empty response from OpenAI".to_string())?;

    // Prefer content, fall back to reasoning_content (for "thinking" models)
    msg.content
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| msg.reasoning_content.clone())
        .ok_or_else(|| "AI returned empty response".to_string())
}
