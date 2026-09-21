//! Turning a conversation into an HTTP request, and an HTTP response back into a reply.
//!
//! Nothing here performs I/O. Every provider's wire format is built and parsed by pure
//! functions so the awkward parts - which field holds the text, which header carries the
//! credential, what an error body looks like - are covered by tests instead of being
//! discovered against a live endpoint.

use serde_json::{Value, json};

use crate::{AiProvider, AiSettings, AssistanceStyle, Persona, UserProfile};

/// API version pinned for Anthropic's Messages API.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Output ceiling for one reply. Generous enough not to truncate a normal answer, small
/// enough to stay well inside the request timeout without streaming.
const MAX_OUTPUT_TOKENS: u32 = 16_000;

/// The shape of a provider's HTTP API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireFormat {
    /// `POST {endpoint}/chat/completions`, the format `OpenAI` defined and others adopted.
    OpenAiCompatible,
    /// `POST {endpoint}/v1/messages`, Anthropic's Messages API.
    Anthropic,
    /// `POST {endpoint}/v1beta/models/{model}:generateContent`.
    Gemini,
}

impl AiProvider {
    /// How this provider expects to be spoken to, or `None` when it is not a provider.
    #[must_use]
    pub const fn wire_format(self) -> Option<WireFormat> {
        match self {
            Self::Disabled => None,
            Self::Anthropic => Some(WireFormat::Anthropic),
            Self::Gemini => Some(WireFormat::Gemini),
            // Ollama and LM Studio both expose an OpenAI-compatible surface.
            Self::OpenAi
            | Self::CustomCloud
            | Self::Ollama
            | Self::LmStudio
            | Self::CustomLocal => Some(WireFormat::OpenAiCompatible),
        }
    }

    /// Where to reach the provider when the user has not named an endpoint.
    ///
    /// The two custom providers have none on purpose: naming the endpoint is the whole
    /// point of choosing them.
    #[must_use]
    pub const fn default_endpoint(self) -> Option<&'static str> {
        match self {
            Self::OpenAi => Some("https://api.openai.com/v1"),
            Self::Anthropic => Some("https://api.anthropic.com"),
            Self::Gemini => Some("https://generativelanguage.googleapis.com"),
            Self::Ollama => Some("http://127.0.0.1:11434/v1"),
            Self::LmStudio => Some("http://127.0.0.1:1234/v1"),
            Self::Disabled | Self::CustomCloud | Self::CustomLocal => None,
        }
    }

    /// The model used when the user has not named one. Always editable in settings.
    #[must_use]
    pub const fn default_model(self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("claude-opus-5"),
            Self::OpenAi => Some("gpt-4o-mini"),
            Self::Gemini => Some("gemini-2.0-flash"),
            Self::Ollama => Some("llama3.2"),
            Self::LmStudio => Some("local-model"),
            Self::Disabled | Self::CustomCloud | Self::CustomLocal => None,
        }
    }
}

/// Who said a line of the conversation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChatRole {
    User,
    Assistant,
}

impl ChatRole {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

/// One line of the conversation sent to the model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatTurn {
    pub role: ChatRole,
    pub text: String,
}

impl ChatTurn {
    #[must_use]
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            text: text.into(),
        }
    }

    #[must_use]
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            text: text.into(),
        }
    }
}

/// A request ready to be put on the wire, with nothing provider-specific left to decide.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub provider: AiProvider,
}

impl ChatRequest {
    /// Header names only - used to assert in tests that a credential never leaks into a log.
    #[must_use]
    pub fn header_names(&self) -> Vec<&str> {
        self.headers.iter().map(|(name, _)| name.as_str()).collect()
    }
}

/// Everything that can go wrong between asking and answering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InferenceError {
    /// No provider is selected.
    NotConfigured,
    /// The provider needs an endpoint the user has not given.
    MissingEndpoint,
    /// The provider needs a model name the user has not given.
    MissingModel,
    /// A cloud provider has no stored credential.
    MissingApiKey,
    /// The request never reached the provider.
    Transport(String),
    /// The provider answered, with a refusal or an error.
    Status { code: u16, message: String },
    /// The provider answered in a shape this client does not understand.
    Malformed(String),
}

impl InferenceError {
    /// A line safe to show in the interface: never contains the credential or the payload.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::NotConfigured => "Elige un proveedor en Ajustes antes de hablar".to_owned(),
            Self::MissingEndpoint => "Configura el endpoint del proveedor en Ajustes".to_owned(),
            Self::MissingModel => "Escribe el modelo a usar en Ajustes".to_owned(),
            Self::MissingApiKey => {
                "Este proveedor necesita una clave guardada en Ajustes".to_owned()
            }
            Self::Transport(detail) => format!("No pudimos contactar al proveedor: {detail}"),
            Self::Status { code, message } if message.is_empty() => {
                format!("El proveedor respondio con error {code}")
            }
            Self::Status { code, message } => format!("Error {code} del proveedor: {message}"),
            Self::Malformed(detail) => format!("Respuesta inesperada del proveedor: {detail}"),
        }
    }
}

/// The instructions Tom gives the model about itself, built from the user's own profile.
#[must_use]
pub fn system_prompt(profile: &UserProfile) -> String {
    let name = profile.persona.display_name();
    let voice = match profile.persona {
        Persona::Tom => "Hablas de forma precisa, serena y directa, con lenguaje profesional.",
        Persona::Tomy => "Hablas de forma cercana, calida y relajada, sin perder criterio.",
    };
    let initiative = match profile.assistance_style {
        AssistanceStyle::Focused => {
            "Responde solo lo que se te pregunta y no propongas trabajo extra."
        }
        AssistanceStyle::Balanced => {
            "Sugiere un siguiente paso solo cuando el patron sea claro, sin insistir."
        }
        AssistanceStyle::Proactive => {
            "Puedes anticipar necesidades y proponer acciones, siempre pidiendo aprobacion."
        }
    };

    format!(
        "Eres {name}, un asistente de escritorio local que acompaña a alguien cuyo trabajo \
principal es: {work}. {voice} {initiative} Nunca ejecutas acciones sin permiso y admites \
cuando no sabes algo en vez de inventarlo. Responde en el idioma del usuario.",
        work = profile.professional_profile.label(),
        name = name,
        voice = voice,
        initiative = initiative,
    )
}

/// Builds the one request that asks the configured provider for the next reply.
///
/// # Errors
///
/// Returns the specific piece of configuration that is missing, so the interface can say
/// what to fix instead of reporting a generic failure.
pub fn build_chat_request(
    settings: &AiSettings,
    api_key: Option<&str>,
    system: &str,
    history: &[ChatTurn],
) -> Result<ChatRequest, InferenceError> {
    let provider = settings.provider;
    let format = provider
        .wire_format()
        .ok_or(InferenceError::NotConfigured)?;
    let endpoint = resolve_endpoint(settings)?;
    let model = resolve_model(settings)?;
    let api_key = api_key.map(str::trim).filter(|key| !key.is_empty());

    // A cloud provider cannot answer without a credential; a local one usually can.
    if provider.is_cloud() && api_key.is_none() {
        return Err(InferenceError::MissingApiKey);
    }

    let mut headers = vec![("content-type".to_owned(), "application/json".to_owned())];
    let (url, body) = match format {
        WireFormat::OpenAiCompatible => {
            if let Some(key) = api_key {
                headers.push(("authorization".to_owned(), format!("Bearer {key}")));
            }
            (
                format!("{endpoint}/chat/completions"),
                openai_body(&model, system, history),
            )
        }
        WireFormat::Anthropic => {
            let key = api_key.ok_or(InferenceError::MissingApiKey)?;
            headers.push(("x-api-key".to_owned(), key.to_owned()));
            headers.push(("anthropic-version".to_owned(), ANTHROPIC_VERSION.to_owned()));
            (
                format!("{endpoint}/v1/messages"),
                anthropic_body(&model, system, history),
            )
        }
        WireFormat::Gemini => {
            let key = api_key.ok_or(InferenceError::MissingApiKey)?;
            // Sent as a header rather than a query parameter so the key stays out of URLs.
            headers.push(("x-goog-api-key".to_owned(), key.to_owned()));
            (
                format!("{endpoint}/v1beta/models/{model}:generateContent"),
                gemini_body(system, history),
            )
        }
    };

    Ok(ChatRequest {
        url,
        headers,
        body: body.to_string(),
        provider,
    })
}

fn resolve_endpoint(settings: &AiSettings) -> Result<String, InferenceError> {
    let configured = settings.endpoint.trim();
    let endpoint = if configured.is_empty() {
        settings
            .provider
            .default_endpoint()
            .ok_or(InferenceError::MissingEndpoint)?
    } else {
        configured
    };
    Ok(endpoint.trim_end_matches('/').to_owned())
}

fn resolve_model(settings: &AiSettings) -> Result<String, InferenceError> {
    let configured = settings.model_path.trim();
    if !configured.is_empty() {
        return Ok(configured.to_owned());
    }
    settings
        .provider
        .default_model()
        .map(ToOwned::to_owned)
        .ok_or(InferenceError::MissingModel)
}

fn openai_body(model: &str, system: &str, history: &[ChatTurn]) -> Value {
    let mut messages = vec![json!({"role": "system", "content": system})];
    messages.extend(
        history
            .iter()
            .map(|turn| json!({"role": turn.role.stable_id(), "content": turn.text})),
    );
    json!({ "model": model, "messages": messages })
}

fn anthropic_body(model: &str, system: &str, history: &[ChatTurn]) -> Value {
    // Anthropic takes the system prompt as its own field, not as a message.
    let messages = history
        .iter()
        .map(|turn| json!({"role": turn.role.stable_id(), "content": turn.text}))
        .collect::<Vec<_>>();
    json!({
        "model": model,
        "max_tokens": MAX_OUTPUT_TOKENS,
        "system": system,
        "messages": messages,
    })
}

fn gemini_body(system: &str, history: &[ChatTurn]) -> Value {
    let contents = history
        .iter()
        .map(|turn| {
            // Gemini calls the assistant "model".
            let role = match turn.role {
                ChatRole::User => "user",
                ChatRole::Assistant => "model",
            };
            json!({"role": role, "parts": [{"text": turn.text}]})
        })
        .collect::<Vec<_>>();
    json!({
        "systemInstruction": {"parts": [{"text": system}]},
        "contents": contents,
    })
}

/// Reads the reply out of a successful response body.
///
/// # Errors
///
/// Returns [`InferenceError::Malformed`] when the body does not hold text where this
/// provider's format says it should.
pub fn parse_chat_response(provider: AiProvider, body: &str) -> Result<String, InferenceError> {
    let format = provider
        .wire_format()
        .ok_or(InferenceError::NotConfigured)?;
    let value: Value =
        serde_json::from_str(body).map_err(|error| InferenceError::Malformed(error.to_string()))?;

    let text = match format {
        WireFormat::OpenAiCompatible => value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        WireFormat::Anthropic => {
            // A refusal is a 200 with a stop reason, not an error status.
            if value.get("stop_reason").and_then(Value::as_str) == Some("refusal") {
                return Err(InferenceError::Status {
                    code: 200,
                    message: "el proveedor declino responder".to_owned(),
                });
            }
            value
                .get("content")
                .and_then(Value::as_array)
                .map(|blocks| join_text_blocks(blocks, "text", "text"))
        }
        WireFormat::Gemini => value
            .pointer("/candidates/0/content/parts")
            .and_then(Value::as_array)
            .map(|parts| join_text_blocks(parts, "text", "text")),
    };

    match text {
        Some(text) if !text.trim().is_empty() => Ok(text),
        _ => Err(InferenceError::Malformed(
            "no se encontro texto en la respuesta".to_owned(),
        )),
    }
}

/// Collects every text block of a content array into one string.
fn join_text_blocks(blocks: &[Value], type_field_value: &str, text_field: &str) -> String {
    blocks
        .iter()
        .filter(|block| {
            // Anthropic tags each block; Gemini's parts carry only the text field.
            block
                .get("type")
                .and_then(Value::as_str)
                .is_none_or(|kind| kind == type_field_value)
        })
        .filter_map(|block| block.get(text_field).and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("")
}

/// Best-effort extraction of a provider's own error message from a failed response.
///
/// Every provider nests it differently and some answer with plain text, so a miss is
/// normal and the caller falls back to reporting the status code alone.
#[must_use]
pub fn parse_error_message(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    let candidates = [
        "/error/message", // OpenAI, Gemini and Anthropic all use this
        "/message",
        "/error",
        "/detail",
    ];
    candidates
        .into_iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
        .map(|message| message.trim().to_owned())
        .filter(|message| !message.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        ChatRole, ChatTurn, InferenceError, WireFormat, build_chat_request, parse_chat_response,
        parse_error_message, system_prompt,
    };
    use crate::{
        AiProvider, AiSettings, AssistanceStyle, Persona, ProfessionalProfile, UserProfile,
    };
    use serde_json::Value;

    fn settings(provider: AiProvider) -> AiSettings {
        AiSettings {
            provider,
            endpoint: String::new(),
            model_path: String::new(),
        }
    }

    fn history() -> Vec<ChatTurn> {
        vec![
            ChatTurn::user("hola"),
            ChatTurn::assistant("dime"),
            ChatTurn::user("resume mi dia"),
        ]
    }

    #[test]
    fn every_usable_provider_has_a_wire_format_and_only_disabled_lacks_one() {
        for provider in AiProvider::ALL {
            assert_eq!(
                provider.wire_format().is_none(),
                provider == AiProvider::Disabled,
                "{}",
                provider.stable_id()
            );
        }
        assert_eq!(
            AiProvider::Ollama.wire_format(),
            Some(WireFormat::OpenAiCompatible),
            "Ollama is reached through its OpenAI-compatible surface"
        );
    }

    #[test]
    fn an_openai_compatible_request_carries_the_system_prompt_and_the_whole_history() {
        let request = build_chat_request(
            &settings(AiProvider::OpenAi),
            Some("sk-test"),
            "SYS",
            &history(),
        )
        .expect("a cloud provider with a key builds");

        assert_eq!(request.url, "https://api.openai.com/v1/chat/completions");
        assert!(request.header_names().contains(&"authorization"));

        let body: Value = serde_json::from_str(&request.body).expect("valid json");
        assert_eq!(body["model"], "gpt-4o-mini");
        let messages = body["messages"].as_array().expect("messages array");
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "SYS");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[2]["role"], "assistant");
        assert_eq!(messages[3]["content"], "resume mi dia");
    }

    #[test]
    fn an_anthropic_request_uses_the_documented_headers_and_a_separate_system_field() {
        let request = build_chat_request(
            &settings(AiProvider::Anthropic),
            Some("sk-ant"),
            "SYS",
            &history(),
        )
        .expect("a cloud provider with a key builds");

        assert_eq!(request.url, "https://api.anthropic.com/v1/messages");
        let names = request.header_names();
        assert!(names.contains(&"x-api-key"));
        assert!(names.contains(&"anthropic-version"));
        assert!(
            !names.contains(&"authorization"),
            "Anthropic authenticates with x-api-key, not a bearer token"
        );

        let body: Value = serde_json::from_str(&request.body).expect("valid json");
        assert_eq!(body["model"], "claude-opus-5");
        assert_eq!(body["system"], "SYS");
        assert!(body["max_tokens"].as_u64().expect("max_tokens") > 0);
        let messages = body["messages"].as_array().expect("messages array");
        assert_eq!(messages.len(), 3, "the system prompt is not a message here");
        assert_eq!(messages[0]["role"], "user");
    }

    #[test]
    fn a_gemini_request_renames_the_assistant_and_keeps_the_key_out_of_the_url() {
        let request = build_chat_request(
            &settings(AiProvider::Gemini),
            Some("goog-key"),
            "SYS",
            &history(),
        )
        .expect("a cloud provider with a key builds");

        assert!(
            request
                .url
                .ends_with("/v1beta/models/gemini-2.0-flash:generateContent")
        );
        assert!(
            !request.url.contains("goog-key"),
            "the credential must never reach the URL"
        );
        assert!(request.header_names().contains(&"x-goog-api-key"));

        let body: Value = serde_json::from_str(&request.body).expect("valid json");
        assert_eq!(body["systemInstruction"]["parts"][0]["text"], "SYS");
        let contents = body["contents"].as_array().expect("contents array");
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model");
    }

    #[test]
    fn a_local_provider_is_reachable_with_no_credential_at_all() {
        let request = build_chat_request(&settings(AiProvider::Ollama), None, "SYS", &history())
            .expect("a local provider needs no key");

        assert_eq!(request.url, "http://127.0.0.1:11434/v1/chat/completions");
        assert!(
            !request.header_names().contains(&"authorization"),
            "no credential means no authorization header"
        );
    }

    #[test]
    fn a_local_provider_behind_a_proxy_still_sends_the_token() {
        let request = build_chat_request(
            &settings(AiProvider::LmStudio),
            Some("proxy-token"),
            "SYS",
            &history(),
        )
        .expect("an optional token is used when present");

        assert!(request.header_names().contains(&"authorization"));
    }

    #[test]
    fn a_cloud_provider_without_a_credential_is_refused_before_any_request_is_built() {
        assert_eq!(
            build_chat_request(&settings(AiProvider::OpenAi), None, "SYS", &history()),
            Err(InferenceError::MissingApiKey)
        );
        assert_eq!(
            build_chat_request(
                &settings(AiProvider::Anthropic),
                Some("   "),
                "SYS",
                &history()
            ),
            Err(InferenceError::MissingApiKey),
            "a blank key is no key"
        );
    }

    #[test]
    fn missing_configuration_is_reported_precisely_rather_than_generically() {
        assert_eq!(
            build_chat_request(&settings(AiProvider::Disabled), None, "SYS", &history()),
            Err(InferenceError::NotConfigured)
        );
        assert_eq!(
            build_chat_request(&settings(AiProvider::CustomLocal), None, "SYS", &history()),
            Err(InferenceError::MissingEndpoint)
        );

        let named_endpoint = AiSettings {
            provider: AiProvider::CustomLocal,
            endpoint: "http://192.168.1.10:8080/v1/".to_owned(),
            model_path: String::new(),
        };
        assert_eq!(
            build_chat_request(&named_endpoint, None, "SYS", &history()),
            Err(InferenceError::MissingModel)
        );
    }

    #[test]
    fn a_configured_endpoint_wins_over_the_default_and_loses_its_trailing_slash() {
        let custom = AiSettings {
            provider: AiProvider::OpenAi,
            endpoint: "  https://gateway.example.com/v1/  ".to_owned(),
            model_path: " my-model ".to_owned(),
        };

        let request =
            build_chat_request(&custom, Some("k"), "SYS", &history()).expect("fully configured");

        assert_eq!(
            request.url,
            "https://gateway.example.com/v1/chat/completions"
        );
        let body: Value = serde_json::from_str(&request.body).expect("valid json");
        assert_eq!(body["model"], "my-model");
    }

    #[test]
    fn each_format_is_read_out_of_the_place_that_format_puts_it() {
        assert_eq!(
            parse_chat_response(
                AiProvider::OpenAi,
                r#"{"choices":[{"message":{"role":"assistant","content":"hola"}}]}"#
            ),
            Ok("hola".to_owned())
        );
        assert_eq!(
            parse_chat_response(
                AiProvider::Anthropic,
                r#"{"content":[{"type":"text","text":"ho"},{"type":"text","text":"la"}],"stop_reason":"end_turn"}"#
            ),
            Ok("hola".to_owned()),
            "several text blocks join into one reply"
        );
        assert_eq!(
            parse_chat_response(
                AiProvider::Gemini,
                r#"{"candidates":[{"content":{"parts":[{"text":"hola"}]}}]}"#
            ),
            Ok("hola".to_owned())
        );
    }

    #[test]
    fn anthropic_thinking_blocks_are_skipped_rather_than_shown_as_the_reply() {
        let body = r#"{"content":[{"type":"thinking","thinking":""},{"type":"text","text":"la respuesta"}]}"#;

        assert_eq!(
            parse_chat_response(AiProvider::Anthropic, body),
            Ok("la respuesta".to_owned())
        );
    }

    #[test]
    fn a_refusal_is_surfaced_as_a_provider_answer_not_as_a_parse_failure() {
        let body = r#"{"content":[],"stop_reason":"refusal"}"#;

        match parse_chat_response(AiProvider::Anthropic, body) {
            Err(InferenceError::Status { code, .. }) => assert_eq!(code, 200),
            other => panic!("expected a refusal status, got {other:?}"),
        }
    }

    #[test]
    fn an_unreadable_body_is_reported_instead_of_showing_an_empty_reply() {
        assert!(matches!(
            parse_chat_response(AiProvider::OpenAi, "not json"),
            Err(InferenceError::Malformed(_))
        ));
        assert!(matches!(
            parse_chat_response(AiProvider::OpenAi, r#"{"choices":[]}"#),
            Err(InferenceError::Malformed(_))
        ));
        assert!(
            matches!(
                parse_chat_response(
                    AiProvider::OpenAi,
                    r#"{"choices":[{"message":{"content":"   "}}]}"#
                ),
                Err(InferenceError::Malformed(_))
            ),
            "a blank reply is not a reply"
        );
    }

    #[test]
    fn provider_error_messages_are_found_wherever_the_provider_nests_them() {
        assert_eq!(
            parse_error_message(
                r#"{"error":{"type":"invalid_request_error","message":"bad model"}}"#
            ),
            Some("bad model".to_owned())
        );
        assert_eq!(
            parse_error_message(r#"{"message":"unauthorized"}"#),
            Some("unauthorized".to_owned())
        );
        assert_eq!(parse_error_message("<html>502</html>"), None);
        assert_eq!(parse_error_message(r#"{"error":{"message":"  "}}"#), None);
    }

    #[test]
    fn user_facing_errors_name_the_fix_and_never_carry_a_payload() {
        assert!(
            InferenceError::MissingApiKey
                .user_message()
                .contains("clave")
        );
        assert!(
            InferenceError::MissingModel
                .user_message()
                .contains("modelo")
        );
        let status = InferenceError::Status {
            code: 429,
            message: "rate limited".to_owned(),
        };
        assert!(status.user_message().contains("429"));
    }

    #[test]
    fn the_system_prompt_speaks_as_the_chosen_persona_and_respects_its_initiative() {
        let focused = UserProfile {
            persona: Persona::Tom,
            professional_profile: ProfessionalProfile::Developer,
            assistance_style: AssistanceStyle::Focused,
        };
        let prompt = system_prompt(&focused);
        assert!(prompt.starts_with("Eres Tom,"));
        assert!(prompt.contains("Desarrollo"));
        assert!(prompt.contains("solo lo que se te pregunta"));

        let proactive = UserProfile {
            persona: Persona::Tomy,
            professional_profile: ProfessionalProfile::Creator,
            assistance_style: AssistanceStyle::Proactive,
        };
        let prompt = system_prompt(&proactive);
        assert!(prompt.starts_with("Eres Tomy,"));
        assert!(prompt.contains("aprobacion"));
    }

    #[test]
    fn roles_use_the_ids_the_wire_formats_expect() {
        assert_eq!(ChatRole::User.stable_id(), "user");
        assert_eq!(ChatRole::Assistant.stable_id(), "assistant");
    }
}
