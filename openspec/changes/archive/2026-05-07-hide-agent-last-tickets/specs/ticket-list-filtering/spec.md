# ticket-list-filtering Specification

## Purpose

Defines the filtering behavior of `ticket list` with respect to agent-last comments. The default view of assigned tickets SHALL reflect only tickets that need attention — i.e. the latest comment was not posted by an agent.

## Requirements

### Requirement: Hide agent-last tickets by default

`ticket list` (with no flags) SHALL exclude tickets where the most recent comment was posted by an agent (identified by a non-null `agent_name` on the comment). Tickets with no comments SHALL always be included.

#### Scenario: Latest comment is from an agent

- **WHEN** a ticket is assigned to the agent AND its most recent comment has `agent_name` set
- **THEN** that ticket SHALL NOT appear in the default `ticket list` output

#### Scenario: Latest comment is from a human

- **WHEN** a ticket is assigned to the agent AND its most recent comment has no `agent_name`
- **THEN** that ticket SHALL appear in the default `ticket list` output

#### Scenario: Ticket has no comments

- **WHEN** a ticket is assigned to the agent AND it has no comments
- **THEN** that ticket SHALL appear in the default `ticket list` output

#### Scenario: --all bypasses the filter

- **WHEN** `ticket list --all` is run
- **THEN** all assigned tickets appear regardless of who posted the latest comment

### Requirement: TicketSummary exposes last_commenter_is_agent

`TicketSummary` SHALL include a `last_commenter_is_agent` boolean field. It SHALL be `true` when the most recent comment on the ticket was posted by an agent, `false` otherwise (including when there are no comments).

#### Scenario: JSON output includes the field

- **WHEN** `ticket list --json` is run
- **THEN** each ticket object in the JSON array SHALL include `"last_commenter_is_agent": true` or `"last_commenter_is_agent": false`

#### Scenario: No comments yields false

- **WHEN** a ticket has no comments
- **THEN** `last_commenter_is_agent` SHALL be `false`
