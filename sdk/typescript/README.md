# @neurograph/sdk

TypeScript client for [NeuroGraph](https://github.com/neurographai/neurograph) — a Rust-powered temporal knowledge graph engine for AI agents.

## Install

```bash
npm install @neurograph/sdk
```

## Quick Start

```typescript
import { NeuroGraph } from '@neurograph/sdk';

const ng = new NeuroGraph({ url: 'http://localhost:8000' });

// Ingest knowledge
await ng.add('Alice joined Anthropic as a research scientist in March 2026');
await ng.add('Bob moved from Google to OpenAI in January 2026');

// Query
const result = await ng.query('Where does Alice work?');
console.log(result.answer); // "Anthropic"

// Time travel
const snapshot = await ng.at('2025-01-01');
console.log(snapshot.entityCount);

// Entity history
const history = await ng.history('Alice');
for (const rel of history) {
  console.log(`${rel.fact} (valid: ${rel.validFrom} - ${rel.validUntil})`);
}

// Branch and diff
await ng.branch('what-if');
const diff = await ng.diffBranches('main', 'what-if');

// Community detection
const communities = await ng.detectCommunities();
console.log(`Found ${communities.communities.length} communities`);
```

## Configuration

```typescript
const ng = new NeuroGraph({
  url: 'http://localhost:8000',  // Required: server URL
  apiKey: 'your-api-key',       // Optional: for authenticated servers
  timeoutMs: 30_000,            // Optional: request timeout (default: 30s)
  groupId: 'my-project',        // Optional: multi-tenant group ID
});
```

## API

| Method | Description |
|--------|-------------|
| `ng.add(text)` | Ingest text into the knowledge graph |
| `ng.addJson(data)` | Ingest a JSON object |
| `ng.addAt(text, date)` | Ingest with explicit timestamp |
| `ng.query(question)` | Query with natural language |
| `ng.search(query, limit?)` | Hybrid search for entities |
| `ng.at(date)` | Temporal snapshot at a date |
| `ng.history(entityName)` | Full relationship history |
| `ng.whatChanged(from, to)` | Temporal diff |
| `ng.branch(name)` | Create a branch |
| `ng.diffBranches(source, target)` | Diff two branches |
| `ng.detectCommunities()` | Run community detection |
| `ng.stats()` | Graph statistics |
| `ng.health()` | Server health check |

## Requirements

- Node.js 18+ (uses native `fetch`)
- A running NeuroGraph server

## License

[Apache-2.0](../../LICENSE)
