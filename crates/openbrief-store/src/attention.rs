use std::collections::BTreeSet;

use openbrief_core::{
    BriefProposal, CuriosityCapture, ObservationBatch, ReturnAnchor, TriageConfirmation,
    TriageProposal, UserDecision,
};
use rusqlite::{OptionalExtension, Transaction, params};
use time::OffsetDateTime;

use crate::{Result, Store, StoreError, timestamp_nanos};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestOutcome {
    Inserted,
    AlreadyPresent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedTriage {
    pub decisions: Vec<UserDecision>,
    pub curiosity_captures: Vec<CuriosityCapture>,
    pub return_anchor: Option<ReturnAnchor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTriageProposal {
    pub proposal: TriageProposal,
    pub confirmed: bool,
}

impl Store {
    /// Persists a normalized producer batch exactly once.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, serialization, or the transaction
    /// fails, or when the batch ID was previously used for different content.
    pub fn ingest_observation_batch(&mut self, batch: &ObservationBatch) -> Result<IngestOutcome> {
        batch.validate()?;
        let payload = serde_json::to_string(batch)?;
        let transaction = self.connection.transaction()?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT payload_json FROM observation_batches WHERE id = ?1",
                [&batch.id],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(existing) = existing {
            if existing == payload {
                return Ok(IngestOutcome::AlreadyPresent);
            }
            return Err(StoreError::IdempotencyConflict {
                entity: "observation batch",
                id: batch.id.clone(),
            });
        }

        transaction.execute(
            "INSERT INTO observation_batches (id, generated_at_ns, payload_json)
             VALUES (?1, ?2, ?3)",
            params![batch.id, timestamp_nanos(batch.generated_at)?, payload],
        )?;
        for observation in &batch.observations {
            transaction.execute(
                "INSERT INTO observations (
                    id, batch_id, source, occurred_at_ns, payload_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    observation.id,
                    batch.id,
                    observation.source,
                    timestamp_nanos(observation.occurred_at)?,
                    serde_json::to_string(observation)?
                ],
            )?;
        }
        for coverage in &batch.source_coverage {
            transaction.execute(
                "INSERT INTO source_coverages (batch_id, source, payload_json)
                 VALUES (?1, ?2, ?3)",
                params![batch.id, coverage.source, serde_json::to_string(coverage)?],
            )?;
        }
        transaction.commit()?;
        Ok(IngestOutcome::Inserted)
    }

    /// Returns the most recently generated observation batch.
    ///
    /// # Errors
    ///
    /// Returns an error when the query or stored JSON decoding fails.
    pub fn latest_observation_batch(&self) -> Result<Option<ObservationBatch>> {
        self.connection
            .query_row(
                "SELECT payload_json FROM observation_batches
                 ORDER BY generated_at_ns DESC, id DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map(|payload| serde_json::from_str(&payload).map_err(StoreError::from))
            .transpose()
    }

    /// Persists an evidence-backed Agent Brief without accepting it.
    ///
    /// # Errors
    ///
    /// Returns an error when validation or persistence fails, or when its
    /// batch or cited observations do not exist.
    pub fn create_brief_proposal(&mut self, proposal: &BriefProposal) -> Result<()> {
        proposal.validate()?;
        let transaction = self.connection.transaction()?;
        require_exists(
            &transaction,
            "observation batch",
            "SELECT EXISTS(SELECT 1 FROM observation_batches WHERE id = ?1)",
            &proposal.batch_id,
        )?;

        for evidence_id in proposal
            .items
            .iter()
            .flat_map(|item| item.evidence_ids.iter())
        {
            let belongs: bool = transaction.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM observations WHERE id = ?1 AND batch_id = ?2
                 )",
                params![evidence_id, proposal.batch_id],
                |row| row.get(0),
            )?;
            if !belongs {
                return Err(StoreError::NotFound {
                    entity: "observation evidence",
                    id: evidence_id.clone(),
                });
            }
        }

        let payload = serde_json::to_string(proposal)?;
        insert_idempotent_json(
            &transaction,
            "brief proposal",
            &proposal.id,
            "SELECT payload_json FROM brief_proposals WHERE id = ?1",
            &payload,
            |transaction, payload| {
                transaction.execute(
                    "INSERT INTO brief_proposals (
                        id, batch_id, created_at_ns, payload_json
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        proposal.id,
                        proposal.batch_id,
                        timestamp_nanos(proposal.created_at)?,
                        payload
                    ],
                )?;
                Ok(())
            },
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Lists Brief proposals newest first.
    ///
    /// # Errors
    ///
    /// Returns an error when the query or stored JSON decoding fails.
    pub fn list_brief_proposals(&self) -> Result<Vec<BriefProposal>> {
        list_json(
            &self.connection,
            "SELECT payload_json FROM brief_proposals
             ORDER BY created_at_ns DESC, id DESC",
        )
    }

    /// Persists an Agent triage interpretation without creating decisions.
    ///
    /// # Errors
    ///
    /// Returns an error when validation or persistence fails, or when cited
    /// Brief candidates or observations do not exist.
    pub fn create_triage_proposal(&mut self, proposal: &TriageProposal) -> Result<()> {
        proposal.validate()?;
        let transaction = self.connection.transaction()?;
        let brief_payload: Option<String> = transaction
            .query_row(
                "SELECT payload_json FROM brief_proposals WHERE id = ?1",
                [&proposal.brief_proposal_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(brief_payload) = brief_payload else {
            return Err(StoreError::NotFound {
                entity: "brief proposal",
                id: proposal.brief_proposal_id.clone(),
            });
        };
        let brief: BriefProposal = serde_json::from_str(&brief_payload)?;
        let candidate_ids = brief
            .items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<BTreeSet<_>>();
        for decision in &proposal.decisions {
            if !candidate_ids.contains(decision.candidate_id.as_str()) {
                return Err(StoreError::NotFound {
                    entity: "brief candidate",
                    id: decision.candidate_id.clone(),
                });
            }
        }
        for evidence_id in proposal
            .decisions
            .iter()
            .flat_map(|value| value.evidence_ids.iter())
            .chain(
                proposal
                    .curiosity_captures
                    .iter()
                    .flat_map(|value| value.evidence_ids.iter()),
            )
        {
            let belongs: bool = transaction.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM observations WHERE id = ?1 AND batch_id = ?2
                 )",
                params![evidence_id, brief.batch_id],
                |row| row.get(0),
            )?;
            if !belongs {
                return Err(StoreError::NotFound {
                    entity: "observation evidence",
                    id: evidence_id.clone(),
                });
            }
        }
        let payload = serde_json::to_string(proposal)?;
        insert_idempotent_json(
            &transaction,
            "triage proposal",
            &proposal.id,
            "SELECT payload_json FROM triage_proposals WHERE id = ?1",
            &payload,
            |transaction, payload| {
                transaction.execute(
                    "INSERT INTO triage_proposals (
                        id, brief_proposal_id, created_at_ns, confirmed_at_ns, payload_json
                     ) VALUES (?1, ?2, ?3, NULL, ?4)",
                    params![
                        proposal.id,
                        proposal.brief_proposal_id,
                        timestamp_nanos(proposal.created_at)?,
                        payload
                    ],
                )?;
                Ok(())
            },
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Lists triage proposals newest first.
    ///
    /// # Errors
    ///
    /// Returns an error when the query or stored JSON decoding fails.
    pub fn list_triage_proposals(&self) -> Result<Vec<StoredTriageProposal>> {
        let mut statement = self.connection.prepare(
            "SELECT payload_json, confirmed_at_ns IS NOT NULL
             FROM triage_proposals
             ORDER BY created_at_ns DESC, id DESC",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(payload, confirmed)| {
                Ok(StoredTriageProposal {
                    proposal: serde_json::from_str(&payload)?,
                    confirmed,
                })
            })
            .collect()
    }

    /// Materializes only the proposal elements explicitly selected by a user.
    ///
    /// # Errors
    ///
    /// Returns an error when the proposal or selection does not exist, the
    /// proposal was already confirmed, or the atomic transaction fails.
    #[allow(clippy::too_many_lines)]
    pub fn confirm_triage_proposal(
        &mut self,
        proposal_id: &str,
        confirmation: &TriageConfirmation,
        confirmed_at: OffsetDateTime,
    ) -> Result<ConfirmedTriage> {
        let transaction = self.connection.transaction()?;
        let row: Option<(String, Option<i64>)> = transaction
            .query_row(
                "SELECT payload_json, confirmed_at_ns
                 FROM triage_proposals WHERE id = ?1",
                [proposal_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((payload, previous_confirmation)) = row else {
            return Err(StoreError::NotFound {
                entity: "triage proposal",
                id: proposal_id.to_owned(),
            });
        };
        if previous_confirmation.is_some() {
            return Err(StoreError::AlreadyConfirmed(proposal_id.to_owned()));
        }
        let proposal: TriageProposal = serde_json::from_str(&payload)?;
        let decision_ids = selected_ids(
            "proposed decision",
            &confirmation.decision_ids,
            proposal.decisions.iter().map(|value| value.id.as_str()),
        )?;
        let capture_ids = selected_ids(
            "proposed curiosity capture",
            &confirmation.curiosity_capture_ids,
            proposal
                .curiosity_captures
                .iter()
                .map(|value| value.id.as_str()),
        )?;
        if confirmation.accept_return_anchor && proposal.return_anchor.is_none() {
            return Err(StoreError::UnknownSelection {
                entity: "proposed return anchor",
                id: proposal.id.clone(),
            });
        }

        let decisions = proposal
            .decisions
            .iter()
            .filter(|value| decision_ids.contains(value.id.as_str()))
            .map(|value| UserDecision {
                id: value.id.clone(),
                proposal_id: proposal.id.clone(),
                candidate_id: value.candidate_id.clone(),
                decision: value.decision.clone(),
                evidence_ids: value.evidence_ids.clone(),
                decided_at: confirmed_at,
            })
            .collect::<Vec<_>>();
        let curiosity_captures = proposal
            .curiosity_captures
            .iter()
            .filter(|value| capture_ids.contains(value.id.as_str()))
            .map(|value| CuriosityCapture {
                id: value.id.clone(),
                proposal_id: proposal.id.clone(),
                question: value.question.clone(),
                evidence_ids: value.evidence_ids.clone(),
                captured_at: confirmed_at,
            })
            .collect::<Vec<_>>();
        let return_anchor = confirmation
            .accept_return_anchor
            .then(|| {
                proposal.return_anchor.as_ref().map(|value| ReturnAnchor {
                    id: value.id.clone(),
                    proposal_id: proposal.id.clone(),
                    label: value.label.clone(),
                    resume_point: value.resume_point.clone(),
                    next_action: value.next_action.clone(),
                    saved_at: confirmed_at,
                })
            })
            .flatten();

        for decision in &decisions {
            insert_entity(
                &transaction,
                "user_decisions",
                &decision.id,
                &proposal.id,
                &serde_json::to_string(decision)?,
            )?;
        }
        for capture in &curiosity_captures {
            insert_entity(
                &transaction,
                "curiosity_captures",
                &capture.id,
                &proposal.id,
                &serde_json::to_string(capture)?,
            )?;
        }
        if let Some(anchor) = &return_anchor {
            insert_entity(
                &transaction,
                "return_anchors",
                &anchor.id,
                &proposal.id,
                &serde_json::to_string(anchor)?,
            )?;
        }
        transaction.execute(
            "UPDATE triage_proposals SET confirmed_at_ns = ?1 WHERE id = ?2",
            params![timestamp_nanos(confirmed_at)?, proposal.id],
        )?;
        transaction.commit()?;

        Ok(ConfirmedTriage {
            decisions,
            curiosity_captures,
            return_anchor,
        })
    }

    /// Lists confirmed user decisions in insertion order.
    ///
    /// # Errors
    ///
    /// Returns an error when the query or stored JSON decoding fails.
    pub fn list_user_decisions(&self) -> Result<Vec<UserDecision>> {
        list_json(
            &self.connection,
            "SELECT payload_json FROM user_decisions ORDER BY rowid",
        )
    }

    /// Lists confirmed curiosity captures in insertion order.
    ///
    /// # Errors
    ///
    /// Returns an error when the query or stored JSON decoding fails.
    pub fn list_curiosity_captures(&self) -> Result<Vec<CuriosityCapture>> {
        list_json(
            &self.connection,
            "SELECT payload_json FROM curiosity_captures ORDER BY rowid",
        )
    }

    /// Lists confirmed return anchors in insertion order.
    ///
    /// # Errors
    ///
    /// Returns an error when the query or stored JSON decoding fails.
    pub fn list_return_anchors(&self) -> Result<Vec<ReturnAnchor>> {
        list_json(
            &self.connection,
            "SELECT payload_json FROM return_anchors ORDER BY rowid",
        )
    }
}

fn selected_ids<'a>(
    entity: &'static str,
    selected: &'a [String],
    available: impl Iterator<Item = &'a str>,
) -> Result<BTreeSet<&'a str>> {
    let available = available.collect::<BTreeSet<_>>();
    let selected = selected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for id in &selected {
        if !available.contains(id) {
            return Err(StoreError::UnknownSelection {
                entity,
                id: (*id).to_owned(),
            });
        }
    }
    Ok(selected)
}

fn require_exists(
    transaction: &Transaction<'_>,
    entity: &'static str,
    query: &str,
    id: &str,
) -> Result<()> {
    let exists: bool = transaction.query_row(query, [id], |row| row.get(0))?;
    if !exists {
        return Err(StoreError::NotFound {
            entity,
            id: id.to_owned(),
        });
    }
    Ok(())
}

fn insert_idempotent_json(
    transaction: &Transaction<'_>,
    entity: &'static str,
    id: &str,
    lookup: &str,
    payload: &str,
    insert: impl FnOnce(&Transaction<'_>, &str) -> Result<()>,
) -> Result<()> {
    let existing: Option<String> = transaction
        .query_row(lookup, [id], |row| row.get(0))
        .optional()?;
    match existing {
        Some(existing) if existing == payload => Ok(()),
        Some(_) => Err(StoreError::IdempotencyConflict {
            entity,
            id: id.to_owned(),
        }),
        None => insert(transaction, payload),
    }
}

fn insert_entity(
    transaction: &Transaction<'_>,
    table: &str,
    id: &str,
    proposal_id: &str,
    payload: &str,
) -> Result<()> {
    let query = format!("INSERT INTO {table} (id, proposal_id, payload_json) VALUES (?1, ?2, ?3)");
    transaction.execute(&query, params![id, proposal_id, payload])?;
    Ok(())
}

fn list_json<T: serde::de::DeserializeOwned>(
    connection: &rusqlite::Connection,
    query: &str,
) -> Result<Vec<T>> {
    let mut statement = connection.prepare(query)?;
    let payloads = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    payloads
        .into_iter()
        .map(|payload| serde_json::from_str(&payload).map_err(StoreError::from))
        .collect()
}

#[cfg(test)]
mod tests {
    use openbrief_core::{
        BriefDisposition, BriefItem, ContentTrust, Observation, ProposedCuriosityCapture,
        ProposedDecision, ProposedReturnAnchor, SourceCoverage, SourceStatus,
    };
    use time::macros::datetime;

    use super::*;

    fn batch() -> ObservationBatch {
        ObservationBatch {
            id: "batch-1".to_owned(),
            producer: "fixture".to_owned(),
            generated_at: datetime!(2026-07-30 10:00:00 +09:00),
            source_coverage: vec![SourceCoverage {
                source: "gmail".to_owned(),
                status: SourceStatus::Ready,
                last_synced_at: datetime!(2026-07-30 09:59:00 +09:00),
                raw_item_count: 1,
                observation_count: 1,
                omitted_item_count: 0,
            }],
            observations: vec![Observation {
                id: "obs-1".to_owned(),
                source: "gmail".to_owned(),
                kind: "message".to_owned(),
                occurred_at: datetime!(2026-07-30 09:58:00 +09:00),
                summary: "A direct question".to_owned(),
                facts: vec!["The user is the recipient".to_owned()],
                source_ref: "fixture-message-1".to_owned(),
                content_trust: ContentTrust::Untrusted,
            }],
        }
    }

    fn brief() -> BriefProposal {
        BriefProposal {
            id: "brief-1".to_owned(),
            batch_id: "batch-1".to_owned(),
            created_at: datetime!(2026-07-30 10:01:00 +09:00),
            items: vec![BriefItem {
                id: "candidate-1".to_owned(),
                title: "Reply today".to_owned(),
                reason: "A direct question needs attention".to_owned(),
                disposition: BriefDisposition::Protect,
                exploration_minutes: None,
                evidence_ids: vec!["obs-1".to_owned()],
                unknowns: Vec::new(),
            }],
        }
    }

    fn triage() -> TriageProposal {
        TriageProposal {
            id: "triage-1".to_owned(),
            brief_proposal_id: "brief-1".to_owned(),
            created_at: datetime!(2026-07-30 10:02:00 +09:00),
            decisions: vec![ProposedDecision {
                id: "decision-1".to_owned(),
                candidate_id: "candidate-1".to_owned(),
                decision: "handle_today".to_owned(),
                evidence_ids: vec!["obs-1".to_owned()],
            }],
            curiosity_captures: vec![ProposedCuriosityCapture {
                id: "capture-1".to_owned(),
                question: "How should direct questions be ranked?".to_owned(),
                evidence_ids: vec!["obs-1".to_owned()],
            }],
            return_anchor: Some(ProposedReturnAnchor {
                id: "anchor-1".to_owned(),
                label: "Authentication test".to_owned(),
                resume_point: "Refresh failure case".to_owned(),
                next_action: "Add a 401 assertion".to_owned(),
            }),
        }
    }

    #[test]
    fn observation_ingest_is_idempotent_and_round_trips_coverage() {
        let mut store = Store::open_in_memory().unwrap();
        let batch = batch();

        assert_eq!(
            store.ingest_observation_batch(&batch).unwrap(),
            IngestOutcome::Inserted
        );
        assert_eq!(
            store.ingest_observation_batch(&batch).unwrap(),
            IngestOutcome::AlreadyPresent
        );
        assert_eq!(store.latest_observation_batch().unwrap(), Some(batch));
    }

    #[test]
    fn agent_proposal_is_inert_until_the_user_confirms_it() {
        let mut store = Store::open_in_memory().unwrap();
        store.ingest_observation_batch(&batch()).unwrap();
        store.create_brief_proposal(&brief()).unwrap();
        store.create_triage_proposal(&triage()).unwrap();

        assert!(!store.list_triage_proposals().unwrap()[0].confirmed);
        assert!(store.list_user_decisions().unwrap().is_empty());
        assert!(store.list_curiosity_captures().unwrap().is_empty());
        assert!(store.list_return_anchors().unwrap().is_empty());

        let result = store
            .confirm_triage_proposal(
                "triage-1",
                &TriageConfirmation {
                    decision_ids: vec!["decision-1".to_owned()],
                    curiosity_capture_ids: vec!["capture-1".to_owned()],
                    accept_return_anchor: true,
                },
                datetime!(2026-07-30 10:03:00 +09:00),
            )
            .unwrap();

        assert_eq!(result.decisions.len(), 1);
        assert!(store.list_triage_proposals().unwrap()[0].confirmed);
        assert_eq!(result.curiosity_captures.len(), 1);
        assert_eq!(
            result
                .return_anchor
                .as_ref()
                .map(|anchor| anchor.id.as_str()),
            Some("anchor-1")
        );
        assert_eq!(store.list_user_decisions().unwrap(), result.decisions);
        assert_eq!(
            store.list_curiosity_captures().unwrap(),
            result.curiosity_captures
        );
        assert_eq!(
            store.list_return_anchors().unwrap(),
            vec![result.return_anchor.unwrap()]
        );
    }
}
