use std::time::Duration;

use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder,
};
use rig_core::completion::Usage;

use crate::error::LlmErrorKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolScope {
    Main,
    Subagent,
    Sleep,
}

impl ToolScope {
    pub fn as_label(self) -> &'static str {
        match self {
            ToolScope::Main => "main",
            ToolScope::Subagent => "subagent",
            ToolScope::Sleep => "sleep",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolOutcome {
    Ok,
    Error,
}

impl ToolOutcome {
    pub fn as_label(self) -> &'static str {
        match self {
            ToolOutcome::Ok => "ok",
            ToolOutcome::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketOutcome {
    Success,
    Error,
    Skipped,
    CapReached,
}

impl TicketOutcome {
    pub fn as_label(self) -> &'static str {
        match self {
            TicketOutcome::Success => "success",
            TicketOutcome::Error => "error",
            TicketOutcome::Skipped => "skipped",
            TicketOutcome::CapReached => "cap_reached",
        }
    }
}

const LLM_DURATION_BUCKETS: &[f64] = &[
    0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 30.0, 60.0,
];

const TICKET_DURATION_BUCKETS: &[f64] = &[
    0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0,
];

pub struct AgentMetrics {
    pub registry: Registry,
    pub llm_requests: IntCounterVec,
    pub llm_duration: HistogramVec,
    pub llm_tokens: IntCounterVec,
    pub tool_calls: IntCounterVec,
    pub ticket_duration: HistogramVec,
}

impl AgentMetrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        let llm_requests = IntCounterVec::new(
            Opts::new(
                "orga_llm_requests_total",
                "LLM completion calls, classified by outcome kind (ok | network | rate_limit | auth | parse | backend | other)",
            ),
            &["model", "provider", "agent", "kind"],
        )
        .expect("metric definition is valid");

        let llm_duration = HistogramVec::new(
            HistogramOpts::new(
                "orga_llm_request_duration_seconds",
                "Wall-clock duration of LLM completion calls in seconds",
            )
            .buckets(LLM_DURATION_BUCKETS.to_vec()),
            &["model", "provider", "agent"],
        )
        .expect("metric definition is valid");

        let llm_tokens = IntCounterVec::new(
            Opts::new(
                "orga_llm_tokens_total",
                "Token usage by kind: input, output, cached (input + cache_creation), reasoning, total",
            ),
            &["model", "provider", "agent", "kind"],
        )
        .expect("metric definition is valid");

        let tool_calls = IntCounterVec::new(
            Opts::new(
                "orga_agent_tool_calls_total",
                "Tool dispatches across all agent scopes",
            ),
            &["tool", "scope", "outcome"],
        )
        .expect("metric definition is valid");

        let ticket_duration = HistogramVec::new(
            HistogramOpts::new(
                "orga_agent_ticket_processing_duration_seconds",
                "Wall-clock duration of processing a single ticket, by outcome",
            )
            .buckets(TICKET_DURATION_BUCKETS.to_vec()),
            &["outcome"],
        )
        .expect("metric definition is valid");

        registry
            .register(Box::new(llm_requests.clone()))
            .expect("register llm_requests");
        registry
            .register(Box::new(llm_duration.clone()))
            .expect("register llm_duration");
        registry
            .register(Box::new(llm_tokens.clone()))
            .expect("register llm_tokens");
        registry
            .register(Box::new(tool_calls.clone()))
            .expect("register tool_calls");
        registry
            .register(Box::new(ticket_duration.clone()))
            .expect("register ticket_duration");

        Self {
            registry,
            llm_requests,
            llm_duration,
            llm_tokens,
            tool_calls,
            ticket_duration,
        }
    }

    pub fn record_llm_request(&self, model: &str, provider: &str, agent: &str) {
        self.llm_requests
            .with_label_values(&[model, provider, agent, "ok"])
            .inc();
    }

    pub fn record_llm_error(
        &self,
        model: &str,
        provider: &str,
        agent: &str,
        kind: LlmErrorKind,
    ) {
        self.llm_requests
            .with_label_values(&[model, provider, agent, kind.as_str()])
            .inc();
    }

    pub fn record_llm_duration(
        &self,
        model: &str,
        provider: &str,
        agent: &str,
        elapsed: Duration,
    ) {
        self.llm_duration
            .with_label_values(&[model, provider, agent])
            .observe(elapsed.as_secs_f64());
    }

    pub fn record_tokens(
        &self,
        model: &str,
        provider: &str,
        agent: &str,
        usage: &Usage,
    ) {
        let input = usage.input_tokens;
        let output = usage.output_tokens;
        let cached = usage.cached_input_tokens + usage.cache_creation_input_tokens;
        let reasoning = usage.reasoning_tokens;
        let total = usage.total_tokens;
        self.llm_tokens
            .with_label_values(&[model, provider, agent, "input"])
            .inc_by(input);
        self.llm_tokens
            .with_label_values(&[model, provider, agent, "output"])
            .inc_by(output);
        self.llm_tokens
            .with_label_values(&[model, provider, agent, "cached"])
            .inc_by(cached);
        self.llm_tokens
            .with_label_values(&[model, provider, agent, "reasoning"])
            .inc_by(reasoning);
        self.llm_tokens
            .with_label_values(&[model, provider, agent, "total"])
            .inc_by(total);
    }

    pub fn record_tool_call(&self, tool: &str, scope: ToolScope, outcome: ToolOutcome) {
        self.tool_calls
            .with_label_values(&[tool, scope.as_label(), outcome.as_label()])
            .inc();
    }

    pub fn record_ticket(&self, outcome: TicketOutcome, elapsed: Duration) {
        self.ticket_duration
            .with_label_values(&[outcome.as_label()])
            .observe(elapsed.as_secs_f64());
    }

    pub fn encode(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let mut buf = Vec::new();
        encoder.encode(&self.registry.gather(), &mut buf)?;
        Ok(String::from_utf8(buf).map_err(|e| {
            prometheus::Error::Msg(format!("metrics text is not valid utf8: {e}"))
        })?)
    }
}

impl Default for AgentMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_registers_all_metrics() {
        let m = AgentMetrics::new();
        // Touch each metric so it appears in the encode output.
        m.record_llm_request("m", "p", "a");
        m.record_llm_error("m", "p", "a", LlmErrorKind::Other);
        m.record_llm_duration("m", "p", "a", Duration::from_millis(1));
        m.record_tokens("m", "p", "a", &Usage::new());
        m.record_tool_call("t", ToolScope::Main, ToolOutcome::Ok);
        m.record_ticket(TicketOutcome::Success, Duration::from_millis(1));
        let text = m.encode().unwrap();
        assert!(text.contains("orga_llm_requests_total"));
        assert!(text.contains("orga_llm_request_duration_seconds"));
        assert!(text.contains("orga_llm_tokens_total"));
        assert!(text.contains("orga_agent_tool_calls_total"));
        assert!(text.contains("orga_agent_ticket_processing_duration_seconds"));
    }

    #[test]
    fn record_llm_request_increments_ok_kind() {
        let m = AgentMetrics::new();
        m.record_llm_request("claude-opus-4-7", "anthropic", "main");
        m.record_llm_request("claude-opus-4-7", "anthropic", "main");
        let v = m
            .llm_requests
            .with_label_values(&["claude-opus-4-7", "anthropic", "main", "ok"])
            .get();
        assert_eq!(v, 2);
    }

    #[test]
    fn record_llm_error_uses_kind_label() {
        let m = AgentMetrics::new();
        m.record_llm_error("m", "p", "a", LlmErrorKind::RateLimited);
        m.record_llm_error("m", "p", "a", LlmErrorKind::Network);
        let rate = m
            .llm_requests
            .with_label_values(&["m", "p", "a", "rate_limit"])
            .get();
        let net = m
            .llm_requests
            .with_label_values(&["m", "p", "a", "network"])
            .get();
        assert_eq!(rate, 1);
        assert_eq!(net, 1);
    }

    #[test]
    fn record_tokens_aggregates_usage() {
        let m = AgentMetrics::new();
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 180,
            cached_input_tokens: 20,
            cache_creation_input_tokens: 5,
            reasoning_tokens: 10,
        };
        m.record_tokens("m", "p", "a", &usage);
        assert_eq!(m.llm_tokens.with_label_values(&["m", "p", "a", "input"]).get(), 100);
        assert_eq!(m.llm_tokens.with_label_values(&["m", "p", "a", "output"]).get(), 50);
        assert_eq!(m.llm_tokens.with_label_values(&["m", "p", "a", "cached"]).get(), 25);
        assert_eq!(m.llm_tokens.with_label_values(&["m", "p", "a", "reasoning"]).get(), 10);
        assert_eq!(m.llm_tokens.with_label_values(&["m", "p", "a", "total"]).get(), 180);
    }

    #[test]
    fn record_tokens_zero_usage_is_noop() {
        let m = AgentMetrics::new();
        m.record_tokens("m", "p", "a", &Usage::new());
        for kind in ["input", "output", "cached", "reasoning", "total"] {
            let v = m.llm_tokens.with_label_values(&["m", "p", "a", kind]).get();
            assert_eq!(v, 0, "kind={kind} should be 0");
        }
    }

    #[test]
    fn record_tool_call_with_scope_and_outcome() {
        let m = AgentMetrics::new();
        m.record_tool_call("comment", ToolScope::Main, ToolOutcome::Ok);
        m.record_tool_call("comment", ToolScope::Subagent, ToolOutcome::Error);
        let ok = m.tool_calls.with_label_values(&["comment", "main", "ok"]).get();
        let err = m.tool_calls.with_label_values(&["comment", "subagent", "error"]).get();
        assert_eq!(ok, 1);
        assert_eq!(err, 1);
    }

    #[test]
    fn record_ticket_observes_duration() {
        let m = AgentMetrics::new();
        m.record_ticket(TicketOutcome::Success, Duration::from_millis(500));
        m.record_ticket(TicketOutcome::Error, Duration::from_secs(2));
        let text = m.encode().unwrap();
        assert!(text.contains("orga_agent_ticket_processing_duration_seconds_count{outcome=\"success\"} 1"));
        assert!(text.contains("orga_agent_ticket_processing_duration_seconds_count{outcome=\"error\"} 1"));
    }

    #[test]
    fn encode_renders_help_and_type_lines() {
        let m = AgentMetrics::new();
        m.record_llm_request("m", "p", "a");
        let text = m.encode().unwrap();
        assert!(text.contains("# HELP orga_llm_requests_total"));
        assert!(text.contains("# TYPE orga_llm_requests_total counter"));
        assert!(text.contains("orga_llm_requests_total{agent=\"a\",kind=\"ok\",model=\"m\",provider=\"p\"} 1"));
    }
}
