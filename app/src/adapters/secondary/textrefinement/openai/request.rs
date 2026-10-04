use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::RefinementCfg;

const MIN_OUTPUT_TOKENS: u32 = 64;
const CHARS_PER_TOKEN: u32 = 2;
const CUT_OFF_FINISH_REASON: &str = "length";
const REASONING_OPEN: &str = "<think>";
const REASONING_CLOSE: &str = "</think>";

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: AnswerMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct AnswerMessage {
    content: String,
}

/// Builds the OpenAI Chat Completions body: the configured prompt is the instruction,
/// each example is a delimited transcript answered with its normalization, and the
/// transcript is delimited data in the last user message.
pub fn chat_body(cfg: &RefinementCfg, transcript: &str) -> String {
    let mut messages = vec![ChatMessage {
        role: "system",
        content: cfg.system_prompt.clone(),
    }];
    for example in &cfg.examples {
        messages.push(transcript_message(&example.transcript));
        messages.push(ChatMessage {
            role: "assistant",
            content: example.normalized.clone(),
        });
    }
    messages.push(transcript_message(transcript));
    let request = ChatRequest {
        model: &cfg.model,
        messages,
        temperature: cfg.temperature,
        max_tokens: output_budget(transcript, cfg.max_tokens),
        stream: false,
    };
    serde_json::to_string(&request).expect("chat request serializes to JSON")
}

fn transcript_message(transcript: &str) -> ChatMessage<'static> {
    ChatMessage {
        role: "user",
        content: format!("<transcript>\n{transcript}\n</transcript>"),
    }
}

/// Output limit in tokens: about one token per two characters of the transcript, which
/// leaves room for a full copy in Russian or English, capped at `max_tokens`.
pub fn output_budget(transcript: &str, max_tokens: u32) -> u32 {
    let chars = u32::try_from(transcript.chars().count()).unwrap_or(u32::MAX);
    chars
        .div_ceil(CHARS_PER_TOKEN)
        .max(MIN_OUTPUT_TOKENS)
        .min(max_tokens)
}

/// Extracts the corrected text from the service's answer. An answer cut off by the
/// output limit or shorter than half the transcript is an error, so the caller falls
/// back to the raw transcript.
pub fn cleaned_answer(json: &str, transcript: &str) -> Result<String> {
    let response: ChatResponse = serde_json::from_str(json)
        .with_context(|| format!("unexpected text refinement response: {json}"))?;
    let choice = response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("text refinement response has no choices: {json}"))?;
    if choice.finish_reason.as_deref() == Some(CUT_OFF_FINISH_REASON) {
        bail!("text refinement stopped at its output limit — the answer is cut off");
    }
    let text = without_reasoning(&choice.message.content).trim().to_owned();
    let (output_chars, input_chars) = (text.chars().count(), transcript.chars().count());
    if output_chars * 2 < input_chars {
        bail!(
            "text refinement answer is shorter than half the transcript \
             ({output_chars} vs {input_chars} chars) — discarding it"
        );
    }
    Ok(text)
}

fn without_reasoning(content: &str) -> &str {
    let trimmed = content.trim_start();
    if !trimmed.starts_with(REASONING_OPEN) {
        return content;
    }
    trimmed
        .find(REASONING_CLOSE)
        .map_or(content, |end| &trimmed[end + REASONING_CLOSE.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RefinementExample;
    use serde_json::Value;

    const TRANSCRIPT: &str = "translate this sentence into russian";

    fn answer(content: &str, finish_reason: &str) -> String {
        serde_json::json!({
            "choices": [{
                "message": {"role": "assistant", "content": content},
                "finish_reason": finish_reason
            }]
        })
        .to_string()
    }

    #[test]
    fn should_send_prompt_as_instruction_and_transcript_as_delimited_data() {
        // Given
        let cfg = RefinementCfg {
            model: "qwen".into(),
            system_prompt: "Fix it.".into(),
            examples: Vec::new(),
            ..RefinementCfg::default()
        };
        // When
        let body: Value = serde_json::from_str(&chat_body(&cfg, TRANSCRIPT)).unwrap();
        // Then
        assert_eq!(body["model"], "qwen");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "Fix it.");
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(
            body["messages"][1]["content"],
            format!("<transcript>\n{TRANSCRIPT}\n</transcript>")
        );
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn should_send_examples_as_turns_between_instruction_and_transcript() {
        // Given
        let cfg = RefinementCfg {
            system_prompt: "Normalize it.".into(),
            examples: vec![RefinementExample {
                transcript: "ну привет".into(),
                normalized: "Ну, привет.".into(),
            }],
            ..RefinementCfg::default()
        };
        // When
        let body: Value = serde_json::from_str(&chat_body(&cfg, TRANSCRIPT)).unwrap();
        // Then
        let roles: Vec<&str> = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["role"].as_str().unwrap())
            .collect();
        assert_eq!(roles, ["system", "user", "assistant", "user"]);
        assert_eq!(
            body["messages"][1]["content"],
            "<transcript>\nну привет\n</transcript>"
        );
        assert_eq!(body["messages"][2]["content"], "Ну, привет.");
        assert_eq!(
            body["messages"][3]["content"],
            format!("<transcript>\n{TRANSCRIPT}\n</transcript>")
        );
    }

    #[test]
    fn should_send_sampling_budget_and_no_streaming() {
        // Given
        let cfg = RefinementCfg {
            temperature: 0.5,
            ..RefinementCfg::default()
        };
        // When
        let body: Value = serde_json::from_str(&chat_body(&cfg, TRANSCRIPT)).unwrap();
        // Then
        assert_eq!(body["temperature"], 0.5);
        assert_eq!(body["max_tokens"], 64);
        assert_eq!(body["stream"], false);
    }

    #[test]
    fn should_budget_at_least_64_tokens_for_short_transcripts() {
        assert_eq!(output_budget("short", 512), 64);
    }

    #[test]
    fn should_budget_one_token_per_two_characters() {
        let transcript = "й".repeat(301);
        assert_eq!(output_budget(&transcript, 512), 151);
    }

    #[test]
    fn should_cap_budget_at_configured_max_tokens() {
        let transcript = "a".repeat(10_000);
        assert_eq!(output_budget(&transcript, 512), 512);
    }

    #[test]
    fn should_return_trimmed_answer_text() {
        let text = cleaned_answer(&answer(" Fixed text here, really. \n", "stop"), TRANSCRIPT);
        assert_eq!(text.unwrap(), "Fixed text here, really.");
    }

    #[test]
    fn should_reject_answer_cut_off_by_output_limit() {
        // Given / When
        let error = cleaned_answer(&answer(TRANSCRIPT, "length"), TRANSCRIPT).unwrap_err();
        // Then
        assert!(error.to_string().contains("output limit"), "{error}");
    }

    #[test]
    fn should_reject_answer_shorter_than_half_the_transcript() {
        // Given / When
        let error = cleaned_answer(&answer("translate", "stop"), TRANSCRIPT).unwrap_err();
        // Then
        assert!(error.to_string().contains("shorter than half"), "{error}");
    }

    #[test]
    fn should_strip_leading_reasoning_block() {
        // Given
        let content = "<think>\nThe user dictated a request.\n</think>\n\nTranslate this sentence into Russian.";
        // When
        let text = cleaned_answer(&answer(content, "stop"), TRANSCRIPT).unwrap();
        // Then
        assert_eq!(text, "Translate this sentence into Russian.");
    }

    #[test]
    fn should_judge_length_after_stripping_reasoning() {
        // Given
        let content = format!("<think>{}</think>ok", "x".repeat(200));
        // When / Then
        assert!(cleaned_answer(&answer(&content, "stop"), TRANSCRIPT).is_err());
    }

    #[test]
    fn should_reject_answer_without_choices() {
        assert!(cleaned_answer(r#"{"choices": []}"#, TRANSCRIPT).is_err());
    }

    #[test]
    fn should_reject_malformed_answer() {
        assert!(cleaned_answer("not json", TRANSCRIPT).is_err());
    }
}
