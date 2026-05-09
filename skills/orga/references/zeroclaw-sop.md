# Standard Operating Procedures (SOP)

SOPs are deterministic procedures executed by the `SopEngine`. They provide explicit trigger matching, approval gates, and auditable run state.

Source: https://github.com/zeroclaw-labs/zeroclaw/blob/master/docs/book/src/sop/index.md

## Quick Paths

- **Connect Events:** [Connectivity & Fan-In](https://github.com/zeroclaw-labs/zeroclaw/blob/master/docs/book/src/sop/connectivity.md) — trigger SOPs via MQTT, webhooks, cron, or peripherals.
- **Write SOPs:** [Syntax Reference](https://github.com/zeroclaw-labs/zeroclaw/blob/master/docs/book/src/sop/syntax.md) — required file layout and trigger/step syntax.
- **Monitor:** [Observability & Audit](https://github.com/zeroclaw-labs/zeroclaw/blob/master/docs/book/src/sop/observability.md) — where run state and audit entries are stored.
- **Examples:** [Cookbook](https://github.com/zeroclaw-labs/zeroclaw/blob/master/docs/book/src/sop/cookbook.md) — reusable SOP patterns.

## Runtime Contract

- SOP definitions are loaded from `<workspace>/sops/<sop_name>/SOP.toml` plus optional `SOP.md`.
- CLI `zeroclaw sop` currently manages definitions only: `list`, `validate`, `show`.
- SOP runs are started by event fan-in (MQTT/webhook/cron/peripheral) or by the in-agent tool `sop_execute`.
- Run progression uses tools: `sop_status`, `sop_approve`, `sop_advance`.
- SOP audit records are persisted in the configured Memory backend under category `sop`.

## Event Flow

```
MQTT / POST /sop/* / Scheduler / Peripheral
  → Dispatch
    → SOP Engine
      → SOP Run
        → ExecuteStep → Agent Loop
        → WaitApproval → Operator → sop_approve → Run
```

## Setup

1. (Optional) Override the SOP directory in `config.toml`:

   ```toml
   [sop]
   sops_dir = "sops"  # defaults to <workspace>/sops when omitted
   ```

2. Create a SOP directory:

   ```text
   ~/.zeroclaw/workspace/sops/<sop_name>/SOP.toml
   ~/.zeroclaw/workspace/sops/<sop_name>/SOP.md
   ```

3. Validate and inspect:

   ```bash
   zeroclaw sop list
   zeroclaw sop validate
   zeroclaw sop show <sop_name>
   ```

4. Trigger runs via configured event sources, or manually from an agent turn with `sop_execute`.

## orga SOP Example

A complete SOP that picks up an orga ticket from the `In Progress` column and moves it to `Done` after a supervised step.

**Directory layout:**

```text
~/.zeroclaw/workspace/sops/orga-ticket-done/
  SOP.toml
  SOP.md
```

**`SOP.toml`:**

```toml
[sop]
name = "orga-ticket-done"
description = "Mark an orga ticket as done after agent confirms work is complete"
version = "1.0.0"
priority = "normal"
execution_mode = "supervised"
cooldown_secs = 0
max_concurrent = 1

[[triggers]]
type = "manual"

[[triggers]]
type = "cron"
expression = "*/5 * * * *"   # every 5 minutes — poll for tickets awaiting closure

[[triggers]]
type = "mqtt"
topic = "orga/ticket/done"
condition = "$.ticket_id != \"\""
```

**`SOP.md`:**

```md
## Steps

1. **Verify ticket** — Load the ticket and confirm latest comment is from the agent, not a human.
   - tools: orga_ticket_show

2. **Move to Done** — Move the ticket to the Done column on the orga board.
   - tools: orga_ticket_move
   - requires_confirmation: true
```

**Trigger manually from an agent turn:**

```
sop_execute("orga-ticket-done", {"ticket_id": "<id>"})
```

**Check progress and approve the supervised step:**

```
sop_status("<run_id>")
sop_approve("<run_id>", <step_number>)
```

**Validate the definition:**

```bash
zeroclaw sop validate orga-ticket-done
```

## orga Integration

When working orga tickets that involve SOP setup:

- SOPs live in the zeroclaw workspace, not in the orga board — they are triggered externally and drive agent behavior
- Use `orga artifact commit <ticket_id> <name> --file <path>` to persist generated `SOP.toml` / `SOP.md` files as ticket artifacts before handing off to the human for deployment
- Report the SOP name and trigger conditions in a ticket comment so the human can verify routing before enabling live triggers
- If a ticket asks you to *run* a SOP, use the `sop_execute` in-agent tool; use `sop_status` to check progress and `sop_approve` to ungate approval steps
