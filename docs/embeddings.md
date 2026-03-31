# Embedding Architecture

> **Universal, provider-agnostic embedding subsystem** — 19 models, 7 providers, zero-code extensibility.

## Overview

NeuroGraph's embedding system is designed so that when tomorrow's model drops, you plug it in with **zero core code changes**. The architecture consists of five layers:

```
┌──────────────────────────────────────────────────────────────┐
│                     EmbeddingRouter                          │
│  (selects provider, handles fallback, LRU cache)             │
├──────────┬──────────┬──────────┬─────────────────────────────┤
│  OpenAI- │  Ollama  │   Hash   │  Custom impl of            │
│  Compat  │  Legacy  │   Local  │  Embedder trait             │
│  Client  │  Client  │          │                             │
│ ─OpenAI  │ ─nomic   │          │ ─Your own model             │
│ ─Gemini  │ ─mxbai   │          │ ─Enterprise API             │
│ ─Cohere  │ ─qwen3   │          │ ─TOML config provider       │
│ ─Voyage  │ ─any     │          │                             │
│ ─Jina    │          │          │                             │
│ ─Mistral │          │          │                             │
│ ─Azure   │          │          │                             │
├──────────┴──────────┴──────────┴─────────────────────────────┤
│                      HNSW Index                              │
│  O(log n) approximate nearest-neighbor search                │
└──────────────────────────────────────────────────────────────┘
```

## Supported Models (19)

| Provider | Model | Dimensions | Cost/1M Tokens | Notes |
|----------|-------|-----------|----------------|-------|
| **OpenAI** | text-embedding-3-small | 1536 | $0.020 | |
| **OpenAI** | text-embedding-3-large | 3072 | $0.130 | |
| **Gemini** | text-embedding-004 | 768 | FREE | |
| **Gemini** | gemini-embedding-exp-03 | 3072 | FREE | Experimental |
| **Gemini** | gemini-embedding-2-preview | 3072 | $0.200 | Multimodal, MRL |
| **Cohere** | embed-v4.0 | 1536 | $0.100 | |
| **Cohere** | embed-english-v3.0 | 1024 | $0.100 | |
| **Voyage** | voyage-3-large | 1024 | $0.180 | |
| **Voyage** | voyage-4-large | 1024 | $0.180 | MoE architecture |
| **Voyage** | voyage-4-lite | 512 | $0.050 | |
| **Voyage** | voyage-code-3 | 1024 | $0.180 | Code-optimized |
| **Jina** | jina-embeddings-v3 | 1024 | $0.018 | MRL support |
| **Jina** | jina-embeddings-v4 | 2048 | $0.050 | Multimodal |
| **Mistral** | mistral-embed | 1024 | $0.100 | |
| **Ollama** | nomic-embed-text | 768 | FREE | Local |
| **Ollama** | mxbai-embed-large | 1024 | FREE | Local |
| **Ollama** | snowflake-arctic-embed | 1024 | FREE | Local |
| **Ollama** | bge-m3 | 1024 | FREE | Local |
| **Ollama** | qwen3-embedding:0.6b | 768 | FREE | Local |

## Quick Start

### Builder API (Code)

```rust
use neurograph_core::NeuroGraph;

// Default: hash embeddings (offline, free)
let ng = NeuroGraph::builder().build().await?;

// OpenAI
let ng = NeuroGraph::builder().openai_embeddings().build().await?;

// Gemini
let ng = NeuroGraph::builder().gemini_embeddings().build().await?;

// Jina
let ng = NeuroGraph::builder().jina_embeddings().build().await?;

// From TOML config file
let ng = NeuroGraph::builder()
    .embeddings_from_config("neurograph.toml")
    .build().await?;
```

### TOML Config (Zero Code)

```toml
[embeddings]
active = "gemini-2"

[embeddings.providers.gemini-2]
type = "gemini"
api_key_env = "GEMINI_API_KEY"
model = "gemini-embedding-2-preview"
dimensions = 3072

[embeddings.providers.openai-backup]
type = "openai"
model = "text-embedding-3-small"

# Future model — zero code changes required!
[embeddings.providers.future-model]
type = "openai-compat"
base_url = "https://api.futureai.com/v1"
api_key_env = "FUTUREAI_API_KEY"
model = "future-embed-v1"
dimensions = 4096
```

## HNSW Vector Index

The MemoryDriver uses a from-scratch HNSW (Hierarchical Navigable Small World) index for vector similarity search:

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Insert | O(log n) amortized | Multi-layer graph construction |
| Search | O(log n) average | Beam search with ef_search candidates |
| Remove | O(n) worst case | Remove + neighbor reconnection |

### Configuration

```rust
use neurograph_core::HnswConfig;

let config = HnswConfig {
    max_connections: 16,     // M: edges per node per layer
    max_connections_0: 32,   // M0: edges on layer 0 (2*M)
    ef_construction: 200,    // Build-time candidate pool
    ef_search: 50,           // Query-time candidate pool
    level_multiplier: 0.36,  // 1/ln(M)
};
```

### When HNSW is Used

- **Unfiltered vector search** → HNSW (O(log n))
- **Group-filtered search** → Brute-force fallback (O(n))
- **Index empty** → Brute-force fallback

## Environment Variables

| Variable | Provider |
|----------|----------|
| `OPENAI_API_KEY` | OpenAI |
| `GEMINI_API_KEY` | Google Gemini |
| `COHERE_API_KEY` | Cohere |
| `VOYAGE_API_KEY` | Voyage AI |
| `JINA_API_KEY` | Jina AI |
| `MISTRAL_API_KEY` | Mistral |
| `AZURE_OPENAI_API_KEY` | Azure OpenAI |
| `OLLAMA_HOST` | Ollama (default: `http://localhost:11434`) |

## Dimension Alignment

When switching between models with different output dimensions, the `DimensionAligner` handles:
- **Truncation**: Strips excess dimensions
- **Padding**: Zero-pads shorter vectors
- **Cosine similarity**: Works across aligned vectors

## Adding a New Provider

### Option 1: TOML Config (Recommended)

Add a `[embeddings.providers.your-model]` section to your `neurograph.toml`. No compilation needed.

### Option 2: Registry Entry

Add a factory method to `EmbeddingRegistry` in `providers.rs`:

```rust
pub fn your_new_model() -> OpenAICompatibleConfig {
    OpenAICompatibleConfig {
        base_url: "https://api.newprovider.com/v1".into(),
        model: "embed-v1".into(),
        api_key: ApiKeySource::Env("NEWPROVIDER_API_KEY".into()),
        // ...
    }
}
```

### Option 3: Custom Embedder Trait

Implement the `Embedder` trait for full control:

```rust
#[async_trait]
impl Embedder for MyEmbedder {
    fn model_name(&self) -> &str { "my-model" }
    fn dimensions(&self) -> usize { 1024 }
    async fn embed_batch(&self, texts: &[String]) -> EmbedderResult<Vec<Vec<f32>>> {
        // Your implementation
    }
}
```
