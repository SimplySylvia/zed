//! Comment mutations over a plan (F3.x). Pure functions on `&mut Plan` shared by
//! the UI (which writes plan.json directly) and the plan server; each bumps
//! `rev` and stamps `history[]`.

use crate::Plan;
use crate::schema::{Anchor, Comment, HistoryEntry, ThreadEntry};

fn bump(plan: &mut Plan, by: &str, kind: &str, summary: String) {
    plan.rev += 1;
    plan.history.push(HistoryEntry {
        rev: Some(plan.rev),
        by: Some(by.to_string()),
        kind: Some(kind.to_string()),
        summary: Some(summary),
        at: None,
        extra: Default::default(),
    });
}

fn message(author: &str, text: &str) -> ThreadEntry {
    ThreadEntry {
        author: Some(author.to_string()),
        text: Some(text.to_string()),
        action: None,
        rev: None,
        cites: vec![],
        pushback: None,
        extra: Default::default(),
    }
}

/// Add a `comment`/`flag` with a severity, anchored to a block, opening a thread.
pub fn add_comment(
    plan: &mut Plan,
    id: &str,
    kind: &str,
    author: &str,
    severity: Option<&str>,
    anchor: Anchor,
    text: &str,
) {
    plan.comments.push(Comment {
        id: id.to_string(),
        kind: Some(kind.to_string()),
        author: Some(author.to_string()),
        severity: severity.map(String::from),
        anchor: Some(anchor),
        outdated: false,
        suggestion: None,
        alternatives: None,
        thread: vec![message(author, text)],
        state: Some("open".to_string()),
        caused_changes: vec![],
        extra: Default::default(),
    });
    bump(plan, author, "comment", format!("added comment {id}"));
}

/// Add a suggested-edit comment carrying the original + replacement text.
pub fn add_suggestion(
    plan: &mut Plan,
    id: &str,
    author: &str,
    anchor: Anchor,
    original: &str,
    replacement: &str,
) {
    plan.comments.push(Comment {
        id: id.to_string(),
        kind: Some("suggestion".to_string()),
        author: Some(author.to_string()),
        severity: None,
        anchor: Some(anchor),
        outdated: false,
        suggestion: Some(serde_json::json!({
            "original": original, "replacement": replacement, "applied": null
        })),
        alternatives: None,
        thread: vec![],
        state: Some("open".to_string()),
        caused_changes: vec![],
        extra: Default::default(),
    });
    bump(plan, author, "suggestion", format!("added suggestion {id}"));
}

/// Append a reply to a comment's thread (agent replies carry an `action`).
pub fn reply(plan: &mut Plan, id: &str, author: &str, text: &str, action: Option<&str>) -> bool {
    let mut entry = message(author, text);
    entry.action = action.map(String::from);
    entry.rev = Some(plan.rev + 1);
    let found = plan
        .comments
        .iter_mut()
        .find(|comment| comment.id == id)
        .map(|comment| comment.thread.push(entry))
        .is_some();
    if found {
        bump(plan, author, "reply", format!("reply on {id}"));
    }
    found
}

/// Set a comment's state (open|sent|addressed|resolved|reopened). Resolution is
/// the user's to give (F3.4d); callers enforce that policy.
pub fn set_comment_state(plan: &mut Plan, id: &str, state: &str) -> bool {
    let found = plan
        .comments
        .iter_mut()
        .find(|comment| comment.id == id)
        .map(|comment| comment.state = Some(state.to_string()))
        .is_some();
    if found {
        bump(plan, "user", "comment_state", format!("{id} → {state}"));
    }
    found
}

/// Mark every open comment as sent (the "Send for revision · n" batch, F3.4).
/// Returns how many were sent.
pub fn mark_sent_batch(plan: &mut Plan) -> usize {
    let mut count = 0;
    for comment in plan.comments.iter_mut() {
        if comment.state.as_deref() == Some("open") {
            comment.state = Some("sent".to_string());
            count += 1;
        }
    }
    if count > 0 {
        bump(
            plan,
            "user",
            "batch_sent",
            format!("sent {count} comments for revision"),
        );
    }
    count
}

/// Apply a suggestion's replacement to its anchored block (F3.2b), marking it
/// `applied: verbatim` and the comment `addressed`.
pub fn apply_suggestion(plan: &mut Plan, id: &str) -> bool {
    let target = plan
        .comments
        .iter()
        .find(|comment| comment.id == id)
        .and_then(|comment| {
            let block = comment.anchor.as_ref()?.block.clone()?;
            let replacement = comment
                .suggestion
                .as_ref()?
                .get("replacement")?
                .as_str()?
                .to_string();
            Some((block, replacement))
        });
    let Some((block, replacement)) = target else {
        return false;
    };
    if !crate::anchor::set_block_text(plan, &block, &replacement) {
        return false;
    }
    if let Some(comment) = plan.comments.iter_mut().find(|comment| comment.id == id) {
        if let Some(suggestion) = comment.suggestion.as_mut().and_then(|s| s.as_object_mut()) {
            suggestion.insert(
                "applied".to_string(),
                serde_json::Value::String("verbatim".to_string()),
            );
        }
        comment.state = Some("addressed".to_string());
    }
    bump(plan, "agent", "apply_suggestion", format!("applied suggestion {id}"));
    true
}
