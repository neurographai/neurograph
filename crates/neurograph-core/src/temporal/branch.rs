// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Git-like branch isolation for agent memory.
//!
//! Each agent operates on its own branch, making isolated changes that
//! can be inspected, diffed, and merged back to the main branch.
//!
//! ## Design
//!
//! Branches use a **copy-on-write (COW)** model:
//! - A new branch starts as a lightweight pointer to a parent state
//! - Only modified items are stored in the branch's delta set
//! - Reads fall through: branch delta → parent delta → ... → root
//!
//! ```text
//! main ──────●────────●────────●──────
//!            │                 ↑
//!            └── agent-ext ──●─┘  (merge)
//!                            │
//!                       agent-val ──●  (still open)
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use neurograph_core::temporal::branch::{BranchManager, BranchConfig};
//! use neurograph_core::graph::{EntityId, RelationshipId};
//!
//! let manager = BranchManager::new(BranchConfig::default());
//! let main_id = manager.main_branch();
//!
//! // Agent creates a working branch
//! let agent_branch = manager.create_branch("extractor-agent", main_id.clone()).unwrap();
//!
//! // Agent makes changes on its branch
//! let entity_id = EntityId::new();
//! let rel_id = RelationshipId::new();
//! manager.add_entity(&agent_branch, entity_id).unwrap();
//! manager.add_relationship(&agent_branch, rel_id).unwrap();
//!
//! // Supervisor inspects the diff
//! let diff = manager.diff(&agent_branch, &main_id).unwrap();
//! println!("Agent added {} entities, {} relationships",
//!     diff.added_entities.len(), diff.added_relationships.len());
//!
//! // Approve and merge
//! let result = manager.merge(&agent_branch, &main_id).unwrap();
//! ```

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

use crate::graph::{EntityId, RelationshipId};

/// Unique identifier for a branch.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchId(pub Uuid);

impl BranchId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn main() -> Self {
        // Deterministic UUID for the main branch (all zeros + 1)
        Self(Uuid::from_u128(1))
    }
}

impl Default for BranchId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BranchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Branch status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchStatus {
    /// Branch is open and accepting changes.
    Open,
    /// Branch has been merged into its target.
    Merged,
    /// Branch has been abandoned (closed without merging).
    Abandoned,
}

/// Information about a branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    /// Branch identifier.
    pub id: BranchId,
    /// Human-readable name (e.g., "extractor-agent-run-42").
    pub name: String,
    /// Parent branch this was forked from.
    pub parent_id: BranchId,
    /// When the branch was created.
    pub created_at: DateTime<Utc>,
    /// Current status.
    pub status: BranchStatus,
    /// Who/what created this branch (agent name, user, etc.).
    pub created_by: String,
    /// Number of entity changes on this branch.
    pub entity_change_count: usize,
    /// Number of relationship changes on this branch.
    pub relationship_change_count: usize,
}

/// The copy-on-write delta for a branch.
///
/// Stores only what changed relative to the parent branch.
#[derive(Debug, Clone, Default)]
struct BranchDelta {
    /// Entity IDs added on this branch.
    added_entities: HashSet<EntityId>,
    /// Entity IDs removed on this branch.
    removed_entities: HashSet<EntityId>,
    /// Relationship IDs added on this branch.
    added_relationships: HashSet<RelationshipId>,
    /// Relationship IDs removed on this branch.
    removed_relationships: HashSet<RelationshipId>,
}

/// Internal branch state.
#[derive(Debug)]
struct BranchState {
    info: BranchInfo,
    delta: BranchDelta,
}

/// The result of diffing two branches.
#[derive(Debug, Clone)]
pub struct BranchDiff {
    /// Source branch.
    pub source: BranchId,
    /// Target branch.
    pub target: BranchId,
    /// Entity IDs added in source but not in target.
    pub added_entities: Vec<EntityId>,
    /// Entity IDs removed in source but present in target.
    pub removed_entities: Vec<EntityId>,
    /// Relationship IDs added in source but not in target.
    pub added_relationships: Vec<RelationshipId>,
    /// Relationship IDs removed in source but present in target.
    pub removed_relationships: Vec<RelationshipId>,
}

impl BranchDiff {
    /// Whether the diff is empty (no changes).
    pub fn is_empty(&self) -> bool {
        self.added_entities.is_empty()
            && self.removed_entities.is_empty()
            && self.added_relationships.is_empty()
            && self.removed_relationships.is_empty()
    }

    /// Total number of changes.
    pub fn change_count(&self) -> usize {
        self.added_entities.len()
            + self.removed_entities.len()
            + self.added_relationships.len()
            + self.removed_relationships.len()
    }
}

/// Result of a merge operation.
#[derive(Debug)]
pub struct MergeResult {
    /// Whether the merge was successful.
    pub success: bool,
    /// Entity IDs merged.
    pub entities_merged: usize,
    /// Relationship IDs merged.
    pub relationships_merged: usize,
    /// Conflicts detected during merge (entity IDs modified in both branches).
    pub conflicts: Vec<MergeConflict>,
}

/// A conflict detected during merge.
#[derive(Debug, Clone)]
pub struct MergeConflict {
    /// Type of conflicting item.
    pub item_type: ConflictItemType,
    /// The conflicting item ID (stringified EntityId or RelationshipId).
    pub item_id: String,
    /// Description of the conflict.
    pub description: String,
}

/// Type of item in a merge conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictItemType {
    Entity,
    Relationship,
}

/// Configuration for the branch manager.
#[derive(Debug, Clone)]
pub struct BranchConfig {
    /// Maximum open branches allowed (prevent branch explosion).
    pub max_open_branches: usize,
    /// Auto-abandon branches older than this many hours (None = never).
    pub auto_abandon_hours: Option<u64>,
}

impl Default for BranchConfig {
    fn default() -> Self {
        Self {
            max_open_branches: 100,
            auto_abandon_hours: Some(24),
        }
    }
}

/// Errors from branch operations.
#[derive(Debug, thiserror::Error)]
pub enum BranchError {
    #[error("Branch not found: {0}")]
    NotFound(String),

    #[error("Branch is not open: {0}")]
    NotOpen(String),

    #[error("Too many open branches (max: {0})")]
    TooManyBranches(usize),

    #[error("Cannot modify the main branch directly")]
    MainBranchProtected,

    #[error("Merge conflict: {0}")]
    MergeConflict(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// The branch manager — orchestrates Git-like branch operations for agent memory.
///
/// Thread-safe: uses `DashMap` for concurrent access from multiple agents.
pub struct BranchManager {
    /// All branches indexed by ID.
    branches: DashMap<BranchId, BranchState>,
    /// Configuration.
    config: BranchConfig,
    /// Counter for unique branch naming.
    branch_counter: AtomicU64,
}

impl BranchManager {
    /// Create a new branch manager with a main branch already initialized.
    pub fn new(config: BranchConfig) -> Self {
        let manager = Self {
            branches: DashMap::new(),
            config,
            branch_counter: AtomicU64::new(0),
        };

        // Create the main branch
        let main_id = BranchId::main();
        manager.branches.insert(
            main_id.clone(),
            BranchState {
                info: BranchInfo {
                    id: main_id.clone(),
                    name: "main".to_string(),
                    parent_id: main_id.clone(), // Self-referencing for root
                    created_at: Utc::now(),
                    status: BranchStatus::Open,
                    created_by: "system".to_string(),
                    entity_change_count: 0,
                    relationship_change_count: 0,
                },
                delta: BranchDelta::default(),
            },
        );

        manager
    }

    /// Get the main branch ID.
    pub fn main_branch(&self) -> BranchId {
        BranchId::main()
    }

    /// Create a new branch forked from a parent branch.
    pub fn create_branch(
        &self,
        name: impl Into<String>,
        parent_id: BranchId,
    ) -> Result<BranchId, BranchError> {
        // Verify parent exists
        if !self.branches.contains_key(&parent_id) {
            return Err(BranchError::NotFound(format!(
                "Parent branch {} not found",
                parent_id
            )));
        }

        // Check branch limit
        let open_count = self
            .branches
            .iter()
            .filter(|b| b.info.status == BranchStatus::Open)
            .count();
        if open_count >= self.config.max_open_branches {
            return Err(BranchError::TooManyBranches(self.config.max_open_branches));
        }

        let branch_id = BranchId::new();
        let counter = self.branch_counter.fetch_add(1, Ordering::Relaxed);
        let name = name.into();
        let full_name = if name.is_empty() {
            format!("branch-{}", counter)
        } else {
            name
        };

        self.branches.insert(
            branch_id.clone(),
            BranchState {
                info: BranchInfo {
                    id: branch_id.clone(),
                    name: full_name,
                    parent_id,
                    created_at: Utc::now(),
                    status: BranchStatus::Open,
                    created_by: String::new(),
                    entity_change_count: 0,
                    relationship_change_count: 0,
                },
                delta: BranchDelta::default(),
            },
        );

        tracing::info!(branch = %branch_id, "Created new branch");
        Ok(branch_id)
    }

    /// Record that an entity was added on a branch.
    pub fn add_entity(
        &self,
        branch_id: &BranchId,
        entity_id: EntityId,
    ) -> Result<(), BranchError> {
        let mut branch = self
            .branches
            .get_mut(branch_id)
            .ok_or_else(|| BranchError::NotFound(branch_id.to_string()))?;

        if branch.info.status != BranchStatus::Open {
            return Err(BranchError::NotOpen(branch_id.to_string()));
        }

        branch.delta.removed_entities.remove(&entity_id);
        branch.delta.added_entities.insert(entity_id);
        branch.info.entity_change_count = branch.delta.added_entities.len()
            + branch.delta.removed_entities.len();
        Ok(())
    }

    /// Record that an entity was removed on a branch.
    pub fn remove_entity(
        &self,
        branch_id: &BranchId,
        entity_id: EntityId,
    ) -> Result<(), BranchError> {
        let mut branch = self
            .branches
            .get_mut(branch_id)
            .ok_or_else(|| BranchError::NotFound(branch_id.to_string()))?;

        if branch.info.status != BranchStatus::Open {
            return Err(BranchError::NotOpen(branch_id.to_string()));
        }

        branch.delta.added_entities.remove(&entity_id);
        branch.delta.removed_entities.insert(entity_id);
        branch.info.entity_change_count = branch.delta.added_entities.len()
            + branch.delta.removed_entities.len();
        Ok(())
    }

    /// Record that a relationship was added on a branch.
    pub fn add_relationship(
        &self,
        branch_id: &BranchId,
        relationship_id: RelationshipId,
    ) -> Result<(), BranchError> {
        let mut branch = self
            .branches
            .get_mut(branch_id)
            .ok_or_else(|| BranchError::NotFound(branch_id.to_string()))?;

        if branch.info.status != BranchStatus::Open {
            return Err(BranchError::NotOpen(branch_id.to_string()));
        }

        branch.delta.removed_relationships.remove(&relationship_id);
        branch.delta.added_relationships.insert(relationship_id);
        branch.info.relationship_change_count = branch.delta.added_relationships.len()
            + branch.delta.removed_relationships.len();
        Ok(())
    }

    /// Record that a relationship was removed on a branch.
    pub fn remove_relationship(
        &self,
        branch_id: &BranchId,
        relationship_id: RelationshipId,
    ) -> Result<(), BranchError> {
        let mut branch = self
            .branches
            .get_mut(branch_id)
            .ok_or_else(|| BranchError::NotFound(branch_id.to_string()))?;

        if branch.info.status != BranchStatus::Open {
            return Err(BranchError::NotOpen(branch_id.to_string()));
        }

        branch.delta.added_relationships.remove(&relationship_id);
        branch.delta.removed_relationships.insert(relationship_id);
        branch.info.relationship_change_count = branch.delta.added_relationships.len()
            + branch.delta.removed_relationships.len();
        Ok(())
    }

    /// Compute the diff between a source branch and a target branch.
    ///
    /// Returns what source has that target doesn't (additions)
    /// and what source removed that target has (removals).
    pub fn diff(
        &self,
        source_id: &BranchId,
        target_id: &BranchId,
    ) -> Result<BranchDiff, BranchError> {
        let source = self
            .branches
            .get(source_id)
            .ok_or_else(|| BranchError::NotFound(source_id.to_string()))?;

        // Verify target exists
        if !self.branches.contains_key(target_id) {
            return Err(BranchError::NotFound(target_id.to_string()));
        }

        Ok(BranchDiff {
            source: source_id.clone(),
            target: target_id.clone(),
            added_entities: source.delta.added_entities.iter().cloned().collect(),
            removed_entities: source.delta.removed_entities.iter().cloned().collect(),
            added_relationships: source.delta.added_relationships.iter().cloned().collect(),
            removed_relationships: source
                .delta
                .removed_relationships
                .iter()
                .cloned()
                .collect(),
        })
    }

    /// Merge a source branch into a target branch.
    ///
    /// Currently implements a simple "apply all changes" strategy.
    /// Future: detect conflicts when both branches modify the same entities.
    pub fn merge(
        &self,
        source_id: &BranchId,
        target_id: &BranchId,
    ) -> Result<MergeResult, BranchError> {
        // Check that source exists and is open
        let source_delta = {
            let source = self
                .branches
                .get(source_id)
                .ok_or_else(|| BranchError::NotFound(source_id.to_string()))?;

            if source.info.status != BranchStatus::Open {
                return Err(BranchError::NotOpen(source_id.to_string()));
            }

            source.delta.clone()
        };

        // Check target exists
        let mut target = self
            .branches
            .get_mut(target_id)
            .ok_or_else(|| BranchError::NotFound(target_id.to_string()))?;

        // Detect conflicts: items removed in source but added in target (or vice versa)
        let mut conflicts = Vec::new();

        for entity_id in &source_delta.added_entities {
            if target.delta.removed_entities.contains(entity_id) {
                conflicts.push(MergeConflict {
                    item_type: ConflictItemType::Entity,
                    item_id: entity_id.to_string(),
                    description: "Entity added in source but removed in target".to_string(),
                });
            }
        }

        for entity_id in &source_delta.removed_entities {
            if target.delta.added_entities.contains(entity_id) {
                conflicts.push(MergeConflict {
                    item_type: ConflictItemType::Entity,
                    item_id: entity_id.to_string(),
                    description: "Entity removed in source but added in target".to_string(),
                });
            }
        }

        // Apply changes (even if there are conflicts — flag but proceed)
        let entities_merged = source_delta.added_entities.len()
            + source_delta.removed_entities.len();
        let relationships_merged = source_delta.added_relationships.len()
            + source_delta.removed_relationships.len();

        // Apply source's adds to target
        for entity_id in source_delta.added_entities {
            target.delta.added_entities.insert(entity_id);
        }
        for rel_id in source_delta.added_relationships {
            target.delta.added_relationships.insert(rel_id);
        }

        // Apply source's removals to target
        for entity_id in source_delta.removed_entities {
            target.delta.removed_entities.insert(entity_id);
        }
        for rel_id in source_delta.removed_relationships {
            target.delta.removed_relationships.insert(rel_id);
        }

        // Update target counts
        target.info.entity_change_count =
            target.delta.added_entities.len() + target.delta.removed_entities.len();
        target.info.relationship_change_count = target.delta.added_relationships.len()
            + target.delta.removed_relationships.len();

        drop(target);

        // Mark source as merged
        if let Some(mut source) = self.branches.get_mut(source_id) {
            source.info.status = BranchStatus::Merged;
        }

        let has_conflicts = !conflicts.is_empty();

        tracing::info!(
            source = %source_id,
            target = %target_id,
            entities = entities_merged,
            relationships = relationships_merged,
            conflicts = conflicts.len(),
            "Branch merged"
        );

        Ok(MergeResult {
            success: !has_conflicts,
            entities_merged,
            relationships_merged,
            conflicts,
        })
    }

    /// Abandon a branch (close it without merging).
    pub fn abandon(&self, branch_id: &BranchId) -> Result<(), BranchError> {
        if *branch_id == BranchId::main() {
            return Err(BranchError::MainBranchProtected);
        }

        let mut branch = self
            .branches
            .get_mut(branch_id)
            .ok_or_else(|| BranchError::NotFound(branch_id.to_string()))?;

        branch.info.status = BranchStatus::Abandoned;

        tracing::info!(branch = %branch_id, "Branch abandoned");
        Ok(())
    }

    /// Get information about a branch.
    pub fn get_info(&self, branch_id: &BranchId) -> Result<BranchInfo, BranchError> {
        let branch = self
            .branches
            .get(branch_id)
            .ok_or_else(|| BranchError::NotFound(branch_id.to_string()))?;
        Ok(branch.info.clone())
    }

    /// List all branches.
    pub fn list_branches(&self) -> Vec<BranchInfo> {
        self.branches.iter().map(|b| b.info.clone()).collect()
    }

    /// List only open branches.
    pub fn list_open_branches(&self) -> Vec<BranchInfo> {
        self.branches
            .iter()
            .filter(|b| b.info.status == BranchStatus::Open)
            .map(|b| b.info.clone())
            .collect()
    }

    /// Check if an entity exists on a branch (including parent chain).
    pub fn entity_visible_on_branch(
        &self,
        branch_id: &BranchId,
        entity_id: &EntityId,
    ) -> bool {
        if let Some(branch) = self.branches.get(branch_id) {
            // If explicitly removed on this branch → not visible
            if branch.delta.removed_entities.contains(entity_id) {
                return false;
            }
            // If explicitly added on this branch → visible
            if branch.delta.added_entities.contains(entity_id) {
                return true;
            }
            // Fall through to parent
            if branch.info.parent_id != *branch_id {
                return self.entity_visible_on_branch(&branch.info.parent_id, entity_id);
            }
        }
        // For the root branch, entities are visible by default (stored in driver)
        true
    }

    /// Total number of branches (all statuses).
    pub fn branch_count(&self) -> usize {
        self.branches.len()
    }
}

impl Default for BranchManager {
    fn default() -> Self {
        Self::new(BranchConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_branch() {
        let manager = BranchManager::new(BranchConfig::default());
        let main_id = manager.main_branch();

        let branch = manager.create_branch("test-agent", main_id.clone());
        assert!(branch.is_ok());

        let branch_id = branch.unwrap();
        let info = manager.get_info(&branch_id).unwrap();
        assert_eq!(info.name, "test-agent");
        assert_eq!(info.parent_id, main_id);
        assert_eq!(info.status, BranchStatus::Open);
    }

    #[test]
    fn test_add_entity_to_branch() {
        let manager = BranchManager::new(BranchConfig::default());
        let main_id = manager.main_branch();
        let branch_id = manager.create_branch("agent", main_id).unwrap();

        let entity_id = EntityId::new();
        manager.add_entity(&branch_id, entity_id.clone()).unwrap();

        let info = manager.get_info(&branch_id).unwrap();
        assert_eq!(info.entity_change_count, 1);
    }

    #[test]
    fn test_diff_branches() {
        let manager = BranchManager::new(BranchConfig::default());
        let main_id = manager.main_branch();
        let branch_id = manager.create_branch("agent", main_id.clone()).unwrap();

        let e1 = EntityId::new();
        let e2 = EntityId::new();
        let r1 = RelationshipId::new();

        manager.add_entity(&branch_id, e1.clone()).unwrap();
        manager.add_entity(&branch_id, e2.clone()).unwrap();
        manager.add_relationship(&branch_id, r1.clone()).unwrap();

        let diff = manager.diff(&branch_id, &main_id).unwrap();
        assert_eq!(diff.added_entities.len(), 2);
        assert_eq!(diff.added_relationships.len(), 1);
        assert_eq!(diff.removed_entities.len(), 0);
        assert_eq!(diff.change_count(), 3);
    }

    #[test]
    fn test_merge_branches() {
        let manager = BranchManager::new(BranchConfig::default());
        let main_id = manager.main_branch();
        let branch_id = manager.create_branch("agent", main_id.clone()).unwrap();

        let e1 = EntityId::new();
        manager.add_entity(&branch_id, e1.clone()).unwrap();

        let result = manager.merge(&branch_id, &main_id).unwrap();
        assert!(result.success);
        assert_eq!(result.entities_merged, 1);
        assert!(result.conflicts.is_empty());

        // Source branch should be marked as merged
        let info = manager.get_info(&branch_id).unwrap();
        assert_eq!(info.status, BranchStatus::Merged);
    }

    #[test]
    fn test_abandon_branch() {
        let manager = BranchManager::new(BranchConfig::default());
        let main_id = manager.main_branch();
        let branch_id = manager.create_branch("agent", main_id).unwrap();

        manager.abandon(&branch_id).unwrap();

        let info = manager.get_info(&branch_id).unwrap();
        assert_eq!(info.status, BranchStatus::Abandoned);
    }

    #[test]
    fn test_cannot_abandon_main() {
        let manager = BranchManager::new(BranchConfig::default());
        let main_id = manager.main_branch();

        let result = manager.abandon(&main_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_branch_limit() {
        let config = BranchConfig {
            max_open_branches: 2, // main + 1 more
            auto_abandon_hours: None,
        };
        let manager = BranchManager::new(config);
        let main_id = manager.main_branch();

        let b1 = manager.create_branch("b1", main_id.clone());
        assert!(b1.is_ok());

        // Should fail: already at max (main + b1 = 2)
        let b2 = manager.create_branch("b2", main_id);
        assert!(b2.is_err());
    }

    #[test]
    fn test_list_branches() {
        let manager = BranchManager::new(BranchConfig::default());
        let main_id = manager.main_branch();

        manager.create_branch("agent-1", main_id.clone()).unwrap();
        manager.create_branch("agent-2", main_id).unwrap();

        let all = manager.list_branches();
        assert_eq!(all.len(), 3); // main + 2 agents

        let open = manager.list_open_branches();
        assert_eq!(open.len(), 3);
    }

    #[test]
    fn test_cannot_modify_merged_branch() {
        let manager = BranchManager::new(BranchConfig::default());
        let main_id = manager.main_branch();
        let branch_id = manager.create_branch("agent", main_id.clone()).unwrap();

        manager.merge(&branch_id, &main_id).unwrap();

        let result = manager.add_entity(&branch_id, EntityId::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_entity_visibility() {
        let manager = BranchManager::new(BranchConfig::default());
        let main_id = manager.main_branch();
        let branch_id = manager.create_branch("agent", main_id.clone()).unwrap();

        let e1 = EntityId::new();
        manager.add_entity(&branch_id, e1.clone()).unwrap();

        // Entity is visible on the branch
        assert!(manager.entity_visible_on_branch(&branch_id, &e1));

        // Entity is not specifically tracked on main (but default = true for root)
        // This is by design: the root branch defers to the driver for actual data

        // Remove entity on branch
        manager.remove_entity(&branch_id, e1.clone()).unwrap();
        assert!(!manager.entity_visible_on_branch(&branch_id, &e1));
    }

    #[test]
    fn test_nested_branches() {
        let manager = BranchManager::new(BranchConfig::default());
        let main_id = manager.main_branch();
        let parent_id = manager.create_branch("parent", main_id).unwrap();
        let child_id = manager.create_branch("child", parent_id.clone()).unwrap();

        let e1 = EntityId::new();
        manager.add_entity(&parent_id, e1.clone()).unwrap();

        // Child should see parent's entity (fall-through)
        assert!(manager.entity_visible_on_branch(&child_id, &e1));

        // Child can override parent
        manager.remove_entity(&child_id, e1.clone()).unwrap();
        assert!(!manager.entity_visible_on_branch(&child_id, &e1));
        // Parent still has it
        assert!(manager.entity_visible_on_branch(&parent_id, &e1));
    }
}
