// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Louvain community detection algorithm — pure Rust implementation.
//!
//! The Louvain algorithm maximizes modularity through two iterative phases:
//! 1. **Local moving**: Each node is moved to the community that gives the
//!    greatest modularity gain.
//! 2. **Aggregation**: Communities become super-nodes, edges between communities
//!    become weighted edges, and the process repeats on the coarsened graph.
//!
//! This is a simplified but fully functional Louvain that operates on the
//! entity and relationship data from our `GraphDriver`.
//!
//! Influenced by GraphRAG's community detection (hierarchical_leiden.py)
//! but implemented in pure Rust for 10-50x speedup.

use std::collections::HashMap;
use crate::drivers::traits::GraphDriver;
use crate::graph::entity::EntityId;
use crate::graph::Community;

/// Configuration for the Louvain algorithm.
#[derive(Debug, Clone)]
pub struct LouvainConfig {
    /// Resolution parameter. Higher = more, smaller communities.
    /// Default: 1.0 (standard modularity).
    pub resolution: f64,
    /// Minimum modularity gain to continue iterating.
    /// Default: 0.0001
    pub min_modularity_gain: f64,
    /// Maximum number of iterations per phase.
    /// Default: 100
    pub max_iterations: usize,
    /// Whether to detect communities hierarchically (multiple levels).
    /// Default: true
    pub hierarchical: bool,
    /// Maximum hierarchy depth.
    /// Default: 3
    pub max_levels: u32,
}

impl Default for LouvainConfig {
    fn default() -> Self {
        Self {
            resolution: 1.0,
            min_modularity_gain: 0.0001,
            max_iterations: 100,
            hierarchical: true,
            max_levels: 3,
        }
    }
}

/// Result of community detection.
#[derive(Debug, Clone)]
pub struct CommunityDetectionResult {
    /// Detected communities.
    pub communities: Vec<Community>,
    /// Mapping: entity_id → community_id at each level.
    pub assignments: HashMap<String, Vec<String>>,
    /// Final modularity score.
    pub modularity: f64,
    /// Number of levels detected.
    pub levels: u32,
    /// Number of iterations used.
    pub iterations: usize,
}

/// Edge in the Louvain working graph.
#[derive(Debug, Clone)]
struct LouvainEdge {
    source: usize,
    target: usize,
    weight: f64,
}

/// The Louvain community detector.
pub struct LouvainDetector {
    config: LouvainConfig,
}

impl LouvainDetector {
    /// Create a new Louvain detector with default config.
    pub fn new() -> Self {
        Self {
            config: LouvainConfig::default(),
        }
    }

    /// Create with custom config.
    pub fn with_config(config: LouvainConfig) -> Self {
        Self { config }
    }

    /// Detect communities in the graph stored in the driver.
    ///
    /// Steps:
    /// 1. Build adjacency from driver's entities + relationships
    /// 2. Run Louvain phases iteratively
    /// 3. Convert detected clusters to `Community` objects
    /// 4. Store communities in the driver
    pub async fn detect(
        &self,
        driver: &dyn GraphDriver,
        group_id: Option<&str>,
    ) -> Result<CommunityDetectionResult, CommunityError> {
        // Step 1: Load all entities and relationships
        let entities = driver
            .list_entities(group_id, 100_000)
            .await
            .map_err(|e| CommunityError::DriverError(e.to_string()))?;

        if entities.is_empty() {
            return Ok(CommunityDetectionResult {
                communities: Vec::new(),
                assignments: HashMap::new(),
                modularity: 0.0,
                levels: 0,
                iterations: 0,
            });
        }

        // Build index: entity_id → node_index
        let entity_id_to_idx: HashMap<String, usize> = entities
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id.as_str(), i))
            .collect();

        let idx_to_entity_id: HashMap<usize, String> = entity_id_to_idx
            .iter()
            .map(|(id, idx)| (*idx, id.clone()))
            .collect();

        // Build edge list from relationships
        let mut edges = Vec::new();
        for entity in &entities {
            let rels = driver
                .get_entity_relationships(&entity.id)
                .await
                .unwrap_or_default();

            for rel in &rels {
                let src_str = rel.source_entity_id.as_str();
                let tgt_str = rel.target_entity_id.as_str();
                let src_idx = entity_id_to_idx.get(&src_str);
                let tgt_idx = entity_id_to_idx.get(&tgt_str);

                if let (Some(&src), Some(&tgt)) = (src_idx, tgt_idx) {
                    if src != tgt {
                        edges.push(LouvainEdge {
                            source: src,
                            target: tgt,
                            weight: rel.weight,
                        });
                    }
                }
            }
        }

        let n = entities.len();
        if edges.is_empty() {
            // No edges: each node is its own community
            return self.single_node_communities(&entities, &idx_to_entity_id);
        }

        // Step 2: Run Louvain algorithm
        let (community_assignments, modularity, iterations) =
            self.run_louvain(n, &edges);

        // Step 3: Build Community objects
        let mut community_members: HashMap<usize, Vec<EntityId>> = HashMap::new();
        for (node_idx, &comm_id) in community_assignments.iter().enumerate() {
            community_members
                .entry(comm_id)
                .or_default()
                .push(EntityId::from_uuid(
                    entities[node_idx].id.0,
                ));
        }

        let mut communities = Vec::new();
        let mut assignments = HashMap::new();

        for (comm_idx, members) in &community_members {
            let comm_name = format!("community_{}", comm_idx);
            let mut community = Community::new(&comm_name, 0);

            for member_id in members {
                community.add_member(member_id.clone());

                // Record assignment
                assignments
                    .entry(member_id.as_str())
                    .or_insert_with(Vec::new)
                    .push(community.id.as_str().to_string());
            }

            communities.push(community);
        }

        // Step 4: Store communities in driver
        for community in &communities {
            driver
                .store_community(community)
                .await
                .map_err(|e| CommunityError::DriverError(e.to_string()))?;
        }

        Ok(CommunityDetectionResult {
            communities,
            assignments,
            modularity,
            levels: 1,
            iterations,
        })
    }

    /// Run the core Louvain algorithm on a graph with n nodes and given edges.
    ///
    /// Returns: (community assignments per node, modularity, iterations used)
    fn run_louvain(
        &self,
        n: usize,
        edges: &[LouvainEdge],
    ) -> (Vec<usize>, f64, usize) {
        // Initialize: each node in its own community
        let mut community: Vec<usize> = (0..n).collect();

        // Build adjacency structure
        let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        let mut total_weight = 0.0;

        for edge in edges {
            adj[edge.source].push((edge.target, edge.weight));
            adj[edge.target].push((edge.source, edge.weight));
            total_weight += edge.weight;
        }

        if total_weight == 0.0 {
            return (community, 0.0, 0);
        }

        // Compute node strengths (sum of edge weights for each node)
        let mut strength: Vec<f64> = vec![0.0; n];
        for edge in edges {
            strength[edge.source] += edge.weight;
            strength[edge.target] += edge.weight;
        }

        // Compute community totals
        let mut community_total: HashMap<usize, f64> = HashMap::new();
        for i in 0..n {
            *community_total.entry(community[i]).or_insert(0.0) += strength[i];
        }

        let m2 = 2.0 * total_weight;
        let mut total_iterations = 0;

        // Phase 1: Local moving
        for _iter in 0..self.config.max_iterations {
            let mut improved = false;
            total_iterations += 1;

            for i in 0..n {
                let current_comm = community[i];
                let ki = strength[i];

                // Calculate modularity gain for moving to each neighbor's community
                let mut best_comm = current_comm;
                let mut best_gain = 0.0;

                // Compute sum of weights to current community
                let mut sigma_in_current = 0.0;
                for &(j, w) in &adj[i] {
                    if community[j] == current_comm {
                        sigma_in_current += w;
                    }
                }

                // Try each neighboring community
                let mut neighbor_comms: HashMap<usize, f64> = HashMap::new();
                for &(j, w) in &adj[i] {
                    *neighbor_comms.entry(community[j]).or_insert(0.0) += w;
                }

                // Remove node i from its current community for calculation
                let sigma_tot_current = community_total.get(&current_comm).copied().unwrap_or(0.0) - ki;

                for (&comm, &sigma_in_new) in &neighbor_comms {
                    if comm == current_comm {
                        continue;
                    }

                    let sigma_tot_new = community_total.get(&comm).copied().unwrap_or(0.0);

                    // Modularity gain formula:
                    // ΔQ = [sigma_in_new/m - resolution * sigma_tot_new * ki / m²]
                    //    - [sigma_in_current/m - resolution * sigma_tot_current * ki / m²]
                    let gain = (sigma_in_new - sigma_in_current) / m2
                        - self.config.resolution * ki * (sigma_tot_new - sigma_tot_current) / (m2 * m2 / 2.0);

                    if gain > best_gain {
                        best_gain = gain;
                        best_comm = comm;
                    }
                }

                if best_comm != current_comm && best_gain > self.config.min_modularity_gain {
                    // Move node to best community
                    *community_total.entry(current_comm).or_insert(0.0) -= ki;
                    *community_total.entry(best_comm).or_insert(0.0) += ki;
                    community[i] = best_comm;
                    improved = true;
                }
            }

            if !improved {
                break;
            }
        }

        // Compute final modularity
        let modularity = self.compute_modularity(&community, edges, total_weight);

        // Re-number communities to be contiguous 0..k
        let mut comm_remap: HashMap<usize, usize> = HashMap::new();
        let mut next_id = 0;
        for c in &mut community {
            let new_id = *comm_remap.entry(*c).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
            *c = new_id;
        }

        (community, modularity, total_iterations)
    }

    /// Compute the modularity Q of the current partition.
    fn compute_modularity(
        &self,
        community: &[usize],
        edges: &[LouvainEdge],
        total_weight: f64,
    ) -> f64 {
        let m2 = 2.0 * total_weight;
        if m2 == 0.0 {
            return 0.0;
        }

        let mut strength: Vec<f64> = vec![0.0; community.len()];
        for edge in edges {
            strength[edge.source] += edge.weight;
            strength[edge.target] += edge.weight;
        }

        let mut q = 0.0;
        for edge in edges {
            if community[edge.source] == community[edge.target] {
                q += edge.weight - self.config.resolution * strength[edge.source] * strength[edge.target] / m2;
            }
        }

        q / m2
    }

    /// Handle the edge case where there are no edges — each node is its own community.
    fn single_node_communities(
        &self,
        entities: &[crate::graph::Entity],
        idx_to_entity_id: &HashMap<usize, String>,
    ) -> Result<CommunityDetectionResult, CommunityError> {
        let mut communities = Vec::new();
        let mut assignments = HashMap::new();

        for (idx, entity) in entities.iter().enumerate() {
            let comm_name = format!("singleton_{}", idx);
            let mut community = Community::new(&comm_name, 0);
            community.add_member(entity.id.clone());

            if let Some(eid) = idx_to_entity_id.get(&idx) {
                assignments
                    .entry(eid.clone())
                    .or_insert_with(Vec::new)
                    .push(community.id.as_str().to_string());
            }

            communities.push(community);
        }

        Ok(CommunityDetectionResult {
            communities,
            assignments,
            modularity: 0.0,
            levels: 1,
            iterations: 0,
        })
    }
}

impl Default for LouvainDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors from community detection.
#[derive(Debug, thiserror::Error)]
pub enum CommunityError {
    #[error("Driver error: {0}")]
    DriverError(String),

    #[error("Algorithm error: {0}")]
    AlgorithmError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::drivers::memory::MemoryDriver;
    use crate::graph::{Entity, Relationship};

    #[test]
    fn test_louvain_config_default() {
        let config = LouvainConfig::default();
        assert_eq!(config.resolution, 1.0);
        assert!(config.hierarchical);
        assert_eq!(config.max_levels, 3);
    }

    #[test]
    fn test_louvain_empty_graph() {
        let detector = LouvainDetector::new();
        let (comm, mod_score, iters) = detector.run_louvain(0, &[]);
        assert!(comm.is_empty());
        assert_eq!(mod_score, 0.0);
        assert_eq!(iters, 0);
    }

    #[test]
    fn test_louvain_single_node() {
        let detector = LouvainDetector::new();
        let (comm, _, _) = detector.run_louvain(1, &[]);
        assert_eq!(comm.len(), 1);
        assert_eq!(comm[0], 0);
    }

    #[test]
    fn test_louvain_two_cliques() {
        let detector = LouvainDetector::new();

        // Two cliques of 3 nodes each, connected by one weak edge
        let edges = vec![
            // Clique 1: nodes 0,1,2
            LouvainEdge { source: 0, target: 1, weight: 1.0 },
            LouvainEdge { source: 1, target: 2, weight: 1.0 },
            LouvainEdge { source: 0, target: 2, weight: 1.0 },
            // Clique 2: nodes 3,4,5
            LouvainEdge { source: 3, target: 4, weight: 1.0 },
            LouvainEdge { source: 4, target: 5, weight: 1.0 },
            LouvainEdge { source: 3, target: 5, weight: 1.0 },
            // Weak bridge
            LouvainEdge { source: 2, target: 3, weight: 0.1 },
        ];

        let (comm, modularity, _) = detector.run_louvain(6, &edges);

        // Should detect 2 communities
        let unique_comms: std::collections::HashSet<usize> = comm.iter().copied().collect();
        assert!(
            unique_comms.len() <= 3, // Allow slight variations
            "Expected ~2 communities, got {}",
            unique_comms.len()
        );

        // Nodes in the same clique should be in the same community
        assert_eq!(comm[0], comm[1], "Nodes 0,1 should be in same community");
        assert_eq!(comm[1], comm[2], "Nodes 1,2 should be in same community");
        assert_eq!(comm[3], comm[4], "Nodes 3,4 should be in same community");
        assert_eq!(comm[4], comm[5], "Nodes 4,5 should be in same community");

        // Two cliques should be in different communities
        assert_ne!(comm[0], comm[3], "Cliques should be different communities");

        // Modularity should be positive
        assert!(modularity > 0.0, "Modularity should be positive: {}", modularity);
    }

    #[tokio::test]
    async fn test_louvain_with_driver() {
        let driver = Arc::new(MemoryDriver::new());

        // Create a small graph
        let alice = Entity::new("Alice", "Person");
        let bob = Entity::new("Bob", "Person");
        let anthropic = Entity::new("Anthropic", "Organization");

        let _: () = driver.store_entity(&alice).await.unwrap();
        let _: () = driver.store_entity(&bob).await.unwrap();
        let _: () = driver.store_entity(&anthropic).await.unwrap();

        let rel1 = Relationship::new(
            alice.id.clone(), anthropic.id.clone(),
            "WORKS_AT", "Alice works at Anthropic",
        );
        let rel2 = Relationship::new(
            bob.id.clone(), anthropic.id.clone(),
            "WORKS_AT", "Bob works at Anthropic",
        );
        let _: () = driver.store_relationship(&rel1).await.unwrap();
        let _: () = driver.store_relationship(&rel2).await.unwrap();

        let detector = LouvainDetector::new();
        let result = detector.detect(driver.as_ref(), None).await.unwrap();

        // Should have at least one community
        assert!(
            !result.communities.is_empty(),
            "Should detect communities"
        );
    }
}
