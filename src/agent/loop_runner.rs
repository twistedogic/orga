use std::future::Future;
use std::sync::Arc;
use std::time::Instant;

use rig_core::completion::{AssistantContent, CompletionModel, CompletionRequest, Message, ToolDefinition};
use rig_core::one_or_many::OneOrMany;

use crate::error::{OrgaError, classify_completion_error};
use crate::metrics::AgentMetrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopOutcome {
    /// LLM returned a response with no tool calls.
    NoToolCalls,
    /// Dispatched tool calls reached `max_steps` without terminal.
    CapReached,
    /// A tool dispatch signalled terminal.
    Terminal,
}

pub fn make_completion_request(history: &[Message], tools: Vec<ToolDefinition>) -> CompletionRequest {
    CompletionRequest {
        model: None,
        preamble: None,
        chat_history: OneOrMany::many(history.to_vec())
            .expect("history must be non-empty"),
        documents: vec![],
        tools,
        temperature: None,
        max_tokens: None,
        tool_choice: None,
        additional_params: None,
        output_schema: None,
    }
}

/// Run the LLM tool-call loop until the LLM returns no tool calls, the dispatch
/// closure signals terminal, or the step cap is reached.
///
/// `dispatch` receives owned `(tool_name, args)` strings and a reference to
/// the full assistant turn's `AssistantContent`s, returning a future that
/// resolves to `(result, is_terminal)`. Using owned strings keeps the closure
/// signature HRTB-friendly so the returned future has a `'static`-ish
/// lifetime relative to the loop body.
///
/// On exit, returns both the [`LoopOutcome`] and any text fragments from the
/// last assistant turn (concatenated with newlines), so callers that need to
/// surface a subagent's final message can do so even when no tool calls
/// occurred.
pub async fn run_llm_loop<M, F, Fut>(
    model: &M,
    history: &mut Vec<Message>,
    tools: Vec<ToolDefinition>,
    max_steps: usize,
    metrics: Arc<AgentMetrics>,
    model_label: &str,
    provider: &str,
    agent: &str,
    mut dispatch: F,
) -> Result<(LoopOutcome, String), OrgaError>
where
    M: CompletionModel,
    F: FnMut(String, String, &[AssistantContent]) -> Fut,
    Fut: Future<Output = (String, bool)>,
{
    let mut last_text = String::new();
    let mut step = 0usize;
    loop {
        if step >= max_steps {
            return Ok((LoopOutcome::CapReached, last_text));
        }

        let req = make_completion_request(history, tools.clone());
        let started = Instant::now();
        let response = model.completion(req).await;
        let elapsed = started.elapsed();
        metrics.record_llm_duration(model_label, provider, agent, elapsed);

        let response = match response {
            Ok(r) => r,
            Err(err) => {
                let (kind, _) = classify_completion_error(&err);
                metrics.record_llm_error(model_label, provider, agent, kind);
                return Err(err.into());
            }
        };

        metrics.record_llm_request(model_label, provider, agent);
        metrics.record_tokens(model_label, provider, agent, &response.usage);

        let choices: Vec<AssistantContent> = response.choice.into_iter().collect();
        last_text = choices
            .iter()
            .filter_map(|c| if let AssistantContent::Text(t) = c { Some(t.text.clone()) } else { None })
            .collect::<Vec<_>>()
            .join("\n");
        let tool_calls: Vec<_> = choices
            .iter()
            .filter_map(|c| if let AssistantContent::ToolCall(tc) = c { Some(tc.clone()) } else { None })
            .collect();

        if let Ok(content) = OneOrMany::many(choices.clone()) {
            history.push(Message::Assistant { id: None, content });
        }

        if tool_calls.is_empty() {
            return Ok((LoopOutcome::NoToolCalls, last_text));
        }

        let mut terminal = false;
        for tc in &tool_calls {
            let name = tc.function.name.clone();
            let args = tc.function.arguments.to_string();
            let (result, is_terminal) = dispatch(name, args, &choices).await;
            history.push(Message::tool_result(tc.id.clone(), result));
            step += 1;
            if is_terminal {
                terminal = true;
                break;
            }
            if step >= max_steps {
                return Ok((LoopOutcome::CapReached, last_text));
            }
        }
        if terminal {
            return Ok((LoopOutcome::Terminal, last_text));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_outcome_variants_distinct() {
        assert_ne!(LoopOutcome::NoToolCalls, LoopOutcome::CapReached);
        assert_ne!(LoopOutcome::CapReached, LoopOutcome::Terminal);
        assert_ne!(LoopOutcome::NoToolCalls, LoopOutcome::Terminal);
    }

    #[test]
    fn make_completion_request_sets_chat_history_and_tools() {
        let history = vec![Message::user("hello".to_string())];
        let tools = vec![ToolDefinition {
            name: "noop".to_string(),
            description: "noop".to_string(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        }];
        let req = make_completion_request(&history, tools.clone());
        assert_eq!(req.tools.len(), 1);
        assert_eq!(req.tools[0].name, "noop");
        assert!(req.model.is_none());
        assert!(req.preamble.is_none());
        assert!(req.temperature.is_none());
        assert!(req.max_tokens.is_none());
        assert!(req.tool_choice.is_none());
        assert!(req.additional_params.is_none());
        assert!(req.output_schema.is_none());
        assert!(req.documents.is_empty());
    }
}
