use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::CoreError;

const MAX_BRIEF_ITEMS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Ready,
    Partial,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCoverage {
    pub source: String,
    pub status: SourceStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub last_synced_at: OffsetDateTime,
    pub raw_item_count: u32,
    pub observation_count: u32,
    #[serde(default)]
    pub omitted_item_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentTrust {
    Untrusted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub id: String,
    pub source: String,
    pub kind: String,
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    pub summary: String,
    #[serde(default)]
    pub facts: Vec<String>,
    pub source_ref: String,
    pub content_trust: ContentTrust,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationBatch {
    pub id: String,
    pub producer: String,
    #[serde(with = "time::serde::rfc3339")]
    pub generated_at: OffsetDateTime,
    pub source_coverage: Vec<SourceCoverage>,
    pub observations: Vec<Observation>,
}

impl ObservationBatch {
    pub fn validate(&self) -> Result<(), CoreError> {
        require_text("observation batch id", &self.id)?;
        require_text("producer", &self.producer)?;
        require_unique_ids(
            "observation",
            self.observations.iter().map(|value| value.id.as_str()),
        )?;
        require_unique_ids(
            "source coverage",
            self.source_coverage
                .iter()
                .map(|value| value.source.as_str()),
        )?;

        for coverage in &self.source_coverage {
            require_text("source", &coverage.source)?;
        }
        for observation in &self.observations {
            require_text("observation id", &observation.id)?;
            require_text("observation source", &observation.source)?;
            require_text("observation kind", &observation.kind)?;
            require_text("observation summary", &observation.summary)?;
            require_text("observation source ref", &observation.source_ref)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BriefDisposition {
    Protect,
    Explore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BriefItem {
    pub id: String,
    pub title: String,
    pub reason: String,
    pub disposition: BriefDisposition,
    pub exploration_minutes: Option<u16>,
    pub evidence_ids: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BriefProposal {
    pub id: String,
    pub batch_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub items: Vec<BriefItem>,
}

impl BriefProposal {
    pub fn validate(&self) -> Result<(), CoreError> {
        require_text("brief proposal id", &self.id)?;
        require_text("observation batch id", &self.batch_id)?;
        if self.items.is_empty() || self.items.len() > MAX_BRIEF_ITEMS {
            return Err(CoreError::InvalidBriefItemCount {
                count: self.items.len(),
                max: MAX_BRIEF_ITEMS,
            });
        }
        require_unique_ids(
            "brief item",
            self.items.iter().map(|value| value.id.as_str()),
        )?;
        for item in &self.items {
            require_text("brief item id", &item.id)?;
            require_text("brief item title", &item.title)?;
            require_text("brief item reason", &item.reason)?;
            if matches!(item.disposition, BriefDisposition::Explore)
                && item.exploration_minutes.is_none_or(|minutes| minutes == 0)
            {
                return Err(CoreError::InvalidExplorationMinutes {
                    item_id: item.id.clone(),
                });
            }
            if item.evidence_ids.is_empty() {
                return Err(CoreError::MissingEvidence {
                    item_id: item.id.clone(),
                });
            }
            require_unique_ids("evidence", item.evidence_ids.iter().map(String::as_str))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedDecision {
    pub id: String,
    pub candidate_id: String,
    pub decision: String,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedCuriosityCapture {
    pub id: String,
    pub question: String,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedReturnAnchor {
    pub id: String,
    pub label: String,
    pub resume_point: String,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriageProposal {
    pub id: String,
    pub brief_proposal_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(default)]
    pub decisions: Vec<ProposedDecision>,
    #[serde(default)]
    pub curiosity_captures: Vec<ProposedCuriosityCapture>,
    pub return_anchor: Option<ProposedReturnAnchor>,
}

impl TriageProposal {
    pub fn validate(&self) -> Result<(), CoreError> {
        require_text("triage proposal id", &self.id)?;
        require_text("brief proposal id", &self.brief_proposal_id)?;
        require_unique_ids(
            "proposed decision",
            self.decisions.iter().map(|value| value.id.as_str()),
        )?;
        require_unique_ids(
            "proposed curiosity capture",
            self.curiosity_captures
                .iter()
                .map(|value| value.id.as_str()),
        )?;

        for decision in &self.decisions {
            require_text("proposed decision id", &decision.id)?;
            require_text("decision candidate id", &decision.candidate_id)?;
            require_text("decision", &decision.decision)?;
            if decision.evidence_ids.is_empty() {
                return Err(CoreError::MissingEvidence {
                    item_id: decision.id.clone(),
                });
            }
        }
        for capture in &self.curiosity_captures {
            require_text("proposed curiosity capture id", &capture.id)?;
            require_text("curiosity question", &capture.question)?;
            if capture.evidence_ids.is_empty() {
                return Err(CoreError::MissingEvidence {
                    item_id: capture.id.clone(),
                });
            }
        }
        if let Some(anchor) = &self.return_anchor {
            require_text("proposed return anchor id", &anchor.id)?;
            require_text("return anchor label", &anchor.label)?;
            require_text("return anchor resume point", &anchor.resume_point)?;
            require_text("return anchor next action", &anchor.next_action)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriageConfirmation {
    #[serde(default)]
    pub decision_ids: Vec<String>,
    #[serde(default)]
    pub curiosity_capture_ids: Vec<String>,
    #[serde(default)]
    pub accept_return_anchor: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserDecision {
    pub id: String,
    pub proposal_id: String,
    pub candidate_id: String,
    pub decision: String,
    pub evidence_ids: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub decided_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuriosityCapture {
    pub id: String,
    pub proposal_id: String,
    pub question: String,
    pub evidence_ids: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub captured_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnAnchor {
    pub id: String,
    pub proposal_id: String,
    pub label: String,
    pub resume_point: String,
    pub next_action: String,
    #[serde(with = "time::serde::rfc3339")]
    pub saved_at: OffsetDateTime,
}

fn require_text(field: &'static str, value: &str) -> Result<(), CoreError> {
    if value.trim().is_empty() {
        return Err(CoreError::EmptyField(field));
    }
    Ok(())
}

fn require_unique_ids<'a>(
    entity: &'static str,
    values: impl IntoIterator<Item = &'a str>,
) -> Result<(), CoreError> {
    let mut unique = BTreeSet::new();
    for value in values {
        if !unique.insert(value) {
            return Err(CoreError::DuplicateId {
                entity,
                id: value.to_owned(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn item(id: &str) -> BriefItem {
        BriefItem {
            id: id.to_owned(),
            title: "Reply".to_owned(),
            reason: "A direct question needs attention".to_owned(),
            disposition: BriefDisposition::Protect,
            exploration_minutes: None,
            evidence_ids: vec!["obs-mail-1".to_owned()],
            unknowns: Vec::new(),
        }
    }

    #[test]
    fn brief_is_finite_and_evidence_backed() {
        let proposal = BriefProposal {
            id: "brief-1".to_owned(),
            batch_id: "batch-1".to_owned(),
            created_at: datetime!(2026-07-30 10:00:00 +09:00),
            items: vec![item("one"), item("two"), item("three"), item("four")],
        };

        assert_eq!(
            proposal.validate(),
            Err(CoreError::InvalidBriefItemCount { count: 4, max: 3 })
        );

        let mut proposal = proposal;
        proposal.items.truncate(1);
        proposal.items[0].evidence_ids.clear();
        assert_eq!(
            proposal.validate(),
            Err(CoreError::MissingEvidence {
                item_id: "one".to_owned()
            })
        );
    }

    #[test]
    fn exploration_requires_a_positive_timebox() {
        let mut explore = item("explore");
        explore.disposition = BriefDisposition::Explore;
        let mut proposal = BriefProposal {
            id: "brief-1".to_owned(),
            batch_id: "batch-1".to_owned(),
            created_at: datetime!(2026-07-30 10:00:00 +09:00),
            items: vec![explore],
        };

        assert_eq!(
            proposal.validate(),
            Err(CoreError::InvalidExplorationMinutes {
                item_id: "explore".to_owned()
            })
        );

        proposal.items[0].exploration_minutes = Some(10);
        assert_eq!(proposal.validate(), Ok(()));
    }
}
