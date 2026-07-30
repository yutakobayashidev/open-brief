use std::path::Path;

use openbrief_core::{
    BriefProposal, CuriosityCapture, ObservationBatch, ReturnAnchor, TriageConfirmation,
    TriageProposal, UserDecision,
};
use openbrief_store::Store;
pub use openbrief_store::{ConfirmedTriage, IngestOutcome, StoredTriageProposal};
use time::OffsetDateTime;

#[derive(Debug, thiserror::Error)]
pub enum AttentionError {
    #[error(transparent)]
    Store(#[from] openbrief_store::StoreError),
}

pub struct AttentionService {
    store: Store,
}

impl AttentionService {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AttentionError> {
        Ok(Self {
            store: Store::open(path)?,
        })
    }

    pub fn from_store(store: Store) -> Self {
        Self { store }
    }

    pub fn ingest(&mut self, batch: &ObservationBatch) -> Result<IngestOutcome, AttentionError> {
        Ok(self.store.ingest_observation_batch(batch)?)
    }

    pub fn latest_observation_batch(&self) -> Result<Option<ObservationBatch>, AttentionError> {
        Ok(self.store.latest_observation_batch()?)
    }

    /// Saves an Agent-produced Brief as a proposal. This never creates a
    /// `UserDecision`.
    pub fn propose_brief(&mut self, proposal: &BriefProposal) -> Result<(), AttentionError> {
        Ok(self.store.create_brief_proposal(proposal)?)
    }

    pub fn brief_proposals(&self) -> Result<Vec<BriefProposal>, AttentionError> {
        Ok(self.store.list_brief_proposals()?)
    }

    /// Saves the Agent's interpretation of natural-language triage. The
    /// proposal remains inert until `confirm_triage` is called by the UI.
    pub fn propose_triage(&mut self, proposal: &TriageProposal) -> Result<(), AttentionError> {
        Ok(self.store.create_triage_proposal(proposal)?)
    }

    pub fn triage_proposals(&self) -> Result<Vec<StoredTriageProposal>, AttentionError> {
        Ok(self.store.list_triage_proposals()?)
    }

    pub fn user_decisions(&self) -> Result<Vec<UserDecision>, AttentionError> {
        Ok(self.store.list_user_decisions()?)
    }

    pub fn curiosity_captures(&self) -> Result<Vec<CuriosityCapture>, AttentionError> {
        Ok(self.store.list_curiosity_captures()?)
    }

    pub fn return_anchors(&self) -> Result<Vec<ReturnAnchor>, AttentionError> {
        Ok(self.store.list_return_anchors()?)
    }

    pub fn confirm_triage(
        &mut self,
        proposal_id: &str,
        confirmation: &TriageConfirmation,
        confirmed_at: OffsetDateTime,
    ) -> Result<ConfirmedTriage, AttentionError> {
        Ok(self
            .store
            .confirm_triage_proposal(proposal_id, confirmation, confirmed_at)?)
    }
}
