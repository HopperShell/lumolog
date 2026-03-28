use serde::Deserialize;

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
