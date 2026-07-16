//! Ticket coverage + drift (F2.4f/g). Pure, facts-in: the skill's Jira MCP does
//! the fetching; these helpers only reason over what's stored in `tickets[]` +
//! `spec.acceptance`. Coverage = every ticket AC maps to a plan criterion; drift =
//! a fresh fetch differs from the stored snapshot.

use crate::Plan;
use crate::lint::Severity;
use crate::schema::Ticket;

/// The stable id for a ticket AC: `"KEY#n"`, 1-based (Appendix A `"LED-212#1"`).
pub fn ticket_ac_id(key: &str, index_1based: usize) -> String {
    format!("{key}#{index_1based}")
}

/// A ticket AC's coverage state for the meter (F2.4b): mapped-and-current,
/// mapped-but-the-ticket-drifted, or not mapped at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageState {
    Covered,
    NeedsUpdate,
    Unmapped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketAcCoverage {
    pub ticket_key: String,
    /// 1-based index into the ticket's `ac` list.
    pub index: usize,
    pub text: String,
    pub state: CoverageState,
}

fn is_mapped(plan: &Plan, ticket_ac_id: &str) -> bool {
    plan.spec
        .acceptance
        .iter()
        .any(|acceptance| acceptance.ticket_ac.as_deref() == Some(ticket_ac_id))
}

/// The coverage view over every ticket AC (F2.4b meter data). A mapped AC whose
/// ticket carries drift reads `NeedsUpdate` (drift is stamped by resync in M8b;
/// until then all mapped ACs read `Covered`).
pub fn coverage(plan: &Plan) -> Vec<TicketAcCoverage> {
    let mut coverage = Vec::new();
    for ticket in &plan.tickets {
        let drifted = ticket.drift.as_ref().is_some_and(|drift| !drift.is_null());
        for (offset, text) in ticket.ac.iter().enumerate() {
            let index = offset + 1;
            let id = ticket_ac_id(&ticket.key, index);
            let state = if !is_mapped(plan, &id) {
                CoverageState::Unmapped
            } else if drifted {
                CoverageState::NeedsUpdate
            } else {
                CoverageState::Covered
            };
            coverage.push(TicketAcCoverage {
                ticket_key: ticket.key.clone(),
                index,
                text: text.clone(),
                state,
            });
        }
    }
    coverage
}

/// Ticket ACs with no mapped criterion (F2.4f) — the lint findings + Done gate.
pub fn uncovered_ticket_acs(plan: &Plan) -> Vec<TicketAcCoverage> {
    coverage(plan)
        .into_iter()
        .filter(|coverage| coverage.state == CoverageState::Unmapped)
        .collect()
}

/// What changed between a stored ticket and a fresh fetch (F2.4g). `None` when
/// nothing did. `ac_changed` is the flag that must re-run coverage; a status/scope
/// change is what M8b turns into a drift card / staged revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketDrift {
    pub status_changed: bool,
    pub ac_changed: bool,
    pub fields_changed: Vec<String>,
}

pub fn ticket_drift(stored: &Ticket, fresh: &Ticket) -> Option<TicketDrift> {
    let mut fields_changed = Vec::new();
    if stored.status != fresh.status {
        fields_changed.push("status".to_string());
    }
    if stored.priority != fresh.priority {
        fields_changed.push("priority".to_string());
    }
    if stored.assignee != fresh.assignee {
        fields_changed.push("assignee".to_string());
    }
    let ac_changed = stored.ac != fresh.ac;
    if ac_changed {
        fields_changed.push("ac".to_string());
    }
    if fields_changed.is_empty() {
        return None;
    }
    Some(TicketDrift {
        status_changed: fields_changed.iter().any(|field| field == "status"),
        ac_changed,
        fields_changed,
    })
}

/// Uncovered ticket ACs block Done (F2.4f) when the rule is a blocker. A descope is
/// recorded as a *mapped* (optionally `waived`) criterion, so it counts as covered
/// here — an omission blocks, a recorded decision does not.
pub fn coverage_blocks_done(plan: &Plan, severity: Severity) -> Option<String> {
    if severity != Severity::Blocker {
        return None;
    }
    let uncovered = uncovered_ticket_acs(plan);
    if uncovered.is_empty() {
        return None;
    }
    let list = uncovered
        .iter()
        .map(|coverage| ticket_ac_id(&coverage.ticket_key, coverage.index))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "{} ticket AC(s) uncovered — map or waive before Done: {list}",
        uncovered.len()
    ))
}
