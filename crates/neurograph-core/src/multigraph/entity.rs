// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Entity graph view: typed entity relationships extracted via regex NER.
//!
//! Entities are connected by typed relationships (works_at, located_in,
//! founded_by, etc.) extracted using pattern matching (offline mode)
//! or NER models. Supports BFS traversal and entity-centric retrieval.

use super::*;
use std::collections::{HashMap, HashSet, VecDeque};

/// Entity graph: nodes are named entities connected by typed relations.
pub struct EntityGraph {
    /// Entity nodes: id -> (name, entity_type)
    entities: HashMap<Uuid, (String, String)>,
    /// Relationship edges: source_id -> Vec<(target_id, rel_type, weight)>
    relationships: HashMap<Uuid, Vec<(Uuid, String, f64)>>,
    /// Name-based lookup index: lowercase_name -> entity_id
    name_index: HashMap<String, Uuid>,
    /// Total edge count
    total_edges: usize,
    #[allow(dead_code)]
    config: MultiGraphConfig,
}

/// A recognized entity from regex NER.
#[derive(Debug, Clone)]
pub struct RecognizedEntity {
    pub name: String,
    pub entity_type: String,
    pub span: (usize, usize),
    pub confidence: f64,
}

/// Regex-based Named Entity Recognition for offline mode.
struct RegexNER {
    patterns: Vec<(regex::Regex, String, f64)>,
}

impl RegexNER {
    fn new() -> Self {
        let patterns = vec![
            // Person names: capitalized word pairs
            (
                regex::Regex::new(r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)\b").unwrap(),
                "Person".to_string(),
                0.7,
            ),
            // Organizations: capitalized words followed by org indicators
            (
                regex::Regex::new(
                    r"\b([A-Z][a-zA-Z]*(?:\s+[A-Z][a-zA-Z]*)*)\s+(?:Inc|Corp|LLC|Ltd|Labs|AI|Research|Foundation|Institute|University|Company)\b",
                ).unwrap(),
                "Organization".to_string(),
                0.8,
            ),
            // Well-known tech companies (exact match, high confidence)
            (
                regex::Regex::new(
                    r"\b(Google|Microsoft|Apple|Amazon|Meta|Anthropic|OpenAI|DeepMind|Tesla|Netflix|Uber|Stripe|Databricks|Snowflake|Cohere|Mistral|Stability)\b",
                ).unwrap(),
                "Organization".to_string(),
                0.95,
            ),
            // Locations
            (
                regex::Regex::new(
                    r"\b(New York|San Francisco|London|Tokyo|Berlin|Paris|Seattle|Austin|Boston|Silicon Valley|Bay Area|Palo Alto|Mountain View)\b",
                ).unwrap(),
                "Location".to_string(),
                0.9,
            ),
            // Dates
            (
                regex::Regex::new(
                    r"\b(\d{4}[-/]\d{1,2}[-/]\d{1,2}|(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{1,2}(?:,?\s+\d{4})?)\b",
                ).unwrap(),
                "Date".to_string(),
                0.85,
            ),
            // Technologies / Products
            (
                regex::Regex::new(
                    r"\b(GPT-\d+|Claude|Gemini|Llama|BERT|Transformer|Kubernetes|Docker|React|Rust|Python|TypeScript|GraphRAG|Neo4j)\b",
                ).unwrap(),
                "Technology".to_string(),
                0.9,
            ),
        ];
        Self { patterns }
    }

    /// Extract entities from text.
    fn extract(&self, text: &str) -> Vec<RecognizedEntity> {
        let mut entities = Vec::new();
        let mut seen_spans: HashSet<(usize, usize)> = HashSet::new();

        for (pattern, entity_type, confidence) in &self.patterns {
            for captures in pattern.captures_iter(text) {
                if let Some(m) = captures.get(1) {
                    let span = (m.start(), m.end());
                    // Avoid overlapping entities
                    if seen_spans.iter().any(|(s, e)| {
                        (span.0 >= *s && span.0 < *e) || (span.1 > *s && span.1 <= *e)
                    }) {
                        continue;
                    }
                    seen_spans.insert(span);
                    entities.push(RecognizedEntity {
                        name: m.as_str().to_string(),
                        entity_type: entity_type.clone(),
                        span,
                        confidence: *confidence,
                    });
                }
            }
        }

        entities
    }
}

/// Regex-based relationship extraction for offline mode.
struct RelationExtractor {
    patterns: Vec<(regex::Regex, String)>,
}

impl RelationExtractor {
    fn new() -> Self {
        let patterns = vec![
            (
                regex::Regex::new(r"(?i)(.+?)\s+(?:works at|works for|employed by|joined)\s+(.+)")
                    .unwrap(),
                "works_at".to_string(),
            ),
            (
                regex::Regex::new(r"(?i)(.+?)\s+(?:founded|co-founded|started|created)\s+(.+)")
                    .unwrap(),
                "founded".to_string(),
            ),
            (
                regex::Regex::new(r"(?i)(.+?)\s+(?:is located in|based in|headquartered in)\s+(.+)")
                    .unwrap(),
                "located_in".to_string(),
            ),
            (
                regex::Regex::new(r"(?i)(.+?)\s+(?:acquired|bought|merged with)\s+(.+)")
                    .unwrap(),
                "acquired".to_string(),
            ),
            (
                regex::Regex::new(r"(?i)(.+?)\s+(?:is a|is an|serves as)\s+(.+)")
                    .unwrap(),
                "is_a".to_string(),
            ),
            (
                regex::Regex::new(r"(?i)(.+?)\s+(?:collaborates with|partners with|works with)\s+(.+)")
                    .unwrap(),
                "collaborates_with".to_string(),
            ),
        ];
        Self { patterns }
    }

    /// Extract relations from text, returning (subject, object, relation_type).
    fn extract(&self, text: &str) -> Vec<(String, String, String)> {
        let mut relations = Vec::new();
        for (pattern, rel_type) in &self.patterns {
            if let Some(captures) = pattern.captures(text) {
                if let (Some(subj), Some(obj)) = (captures.get(1), captures.get(2)) {
                    relations.push((
                        subj.as_str().trim().to_string(),
                        obj.as_str().trim().to_string(),
                        rel_type.clone(),
                    ));
                }
            }
        }
        relations
    }
}

impl EntityGraph {
    pub fn new(config: &MultiGraphConfig) -> Self {
        Self {
            entities: HashMap::new(),
            relationships: HashMap::new(),
            name_index: HashMap::new(),
            total_edges: 0,
            config: config.clone(),
        }
    }

    /// Add a memory item: extract entities and relationships, create graph edges.
    pub fn add_item(&mut self, item: &MemoryItem) -> Vec<GraphEdge> {
        let ner = RegexNER::new();
        let rel_extractor = RelationExtractor::new();
        let mut new_edges = Vec::new();

        // Step 1: Extract entities
        let recognized = ner.extract(&item.content);
        let mut local_entity_ids: HashMap<String, Uuid> = HashMap::new();

        for entity in &recognized {
            let key = entity.name.to_lowercase();
            let entity_id = if let Some(&existing_id) = self.name_index.get(&key) {
                existing_id
            } else {
                let id = Uuid::new_v4();
                self.entities
                    .insert(id, (entity.name.clone(), entity.entity_type.clone()));
                self.name_index.insert(key.clone(), id);
                id
            };
            local_entity_ids.insert(entity.name.clone(), entity_id);
        }

        // Step 2: Extract relationships
        let relations = rel_extractor.extract(&item.content);

        for (subj, obj, rel_type) in &relations {
            // Try to match subjects and objects to extracted entities
            let subj_id = local_entity_ids
                .get(subj)
                .or_else(|| {
                    let key = subj.to_lowercase();
                    self.name_index.get(&key)
                })
                .copied();

            let obj_id = local_entity_ids
                .get(obj)
                .or_else(|| {
                    let key = obj.to_lowercase();
                    self.name_index.get(&key)
                })
                .copied();

            if let (Some(source_id), Some(target_id)) = (subj_id, obj_id) {
                self.relationships
                    .entry(source_id)
                    .or_default()
                    .push((target_id, rel_type.clone(), 1.0));
                self.total_edges += 1;

                new_edges.push(GraphEdge {
                    id: Uuid::new_v4(),
                    source: source_id,
                    target: target_id,
                    view: GraphView::Entity,
                    relation: rel_type.clone(),
                    weight: 1.0,
                    valid_from: item.valid_from,
                    valid_until: item.valid_until,
                    metadata: HashMap::new(),
                });
            }
        }

        // Step 3: Co-occurrence edges (entities mentioned in the same text)
        let entity_ids: Vec<Uuid> = local_entity_ids.values().copied().collect();
        for i in 0..entity_ids.len() {
            for j in (i + 1)..entity_ids.len() {
                self.relationships
                    .entry(entity_ids[i])
                    .or_default()
                    .push((entity_ids[j], "co_occurs_with".to_string(), 0.5));
                self.relationships
                    .entry(entity_ids[j])
                    .or_default()
                    .push((entity_ids[i], "co_occurs_with".to_string(), 0.5));
                self.total_edges += 2;
            }
        }

        new_edges
    }

    /// BFS traversal from query-matching entities.
    pub fn traverse(&self, query: &str, opts: &QueryOptions) -> SubgraphResult {
        let start = std::time::Instant::now();
        let query_lower = query.to_lowercase();

        // Find seed entities by name matching
        let seeds: Vec<Uuid> = self
            .name_index
            .iter()
            .filter(|(name, _)| query_lower.contains(name.as_str()))
            .map(|(_, id)| *id)
            .collect();

        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut results: Vec<(Uuid, f64)> = Vec::new();
        let mut queue: VecDeque<(Uuid, f64, usize)> = seeds
            .iter()
            .map(|id| (*id, 1.0, 0))
            .collect();

        while let Some((node_id, score, depth)) = queue.pop_front() {
            if visited.contains(&node_id) || depth > 3 {
                continue;
            }
            if results.len() >= opts.max_results {
                break;
            }

            visited.insert(node_id);
            results.push((node_id, score));

            if let Some(neighbors) = self.relationships.get(&node_id) {
                for (neighbor_id, _rel, weight) in neighbors {
                    if !visited.contains(neighbor_id) {
                        queue.push_back((*neighbor_id, score * weight * 0.7, depth + 1));
                    }
                }
            }
        }

        SubgraphResult {
            view: GraphView::Entity,
            node_ids: results.iter().map(|(id, _)| *id).collect(),
            scores: results.iter().map(|(_, s)| *s).collect(),
            edges: Vec::new(),
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Get entity info by name.
    pub fn get_entity_by_name(&self, name: &str) -> Option<(Uuid, String, String)> {
        let key = name.to_lowercase();
        self.name_index.get(&key).and_then(|id| {
            self.entities
                .get(id)
                .map(|(name, etype)| (*id, name.clone(), etype.clone()))
        })
    }

    /// Number of entity relationship edges.
    pub fn edge_count(&self) -> usize {
        self.total_edges
    }

    /// Number of known entities.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_ner() {
        let ner = RegexNER::new();
        let entities = ner.extract("Alice Smith works at Anthropic in San Francisco");

        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"Alice Smith"), "Should find person: {:?}", names);
        assert!(names.contains(&"Anthropic"), "Should find org: {:?}", names);
        assert!(names.contains(&"San Francisco"), "Should find location: {:?}", names);
    }

    #[test]
    fn test_relation_extraction() {
        let extractor = RelationExtractor::new();
        let relations = extractor.extract("Alice works at Anthropic");
        assert!(!relations.is_empty());
        assert_eq!(relations[0].2, "works_at");
    }

    #[test]
    fn test_entity_graph_construction() {
        let config = MultiGraphConfig::default();
        let mut graph = EntityGraph::new(&config);

        let item = MemoryItem::new(
            "Alice Smith works at Anthropic in San Francisco".to_string(),
            vec![0.0; 128],
        );

        let _edges = graph.add_item(&item);
        assert!(graph.entity_count() > 0, "Should have extracted entities");
        // Check name index
        assert!(graph.get_entity_by_name("anthropic").is_some());
    }
}
