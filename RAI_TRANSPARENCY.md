# Responsible AI Transparency

## What is NeuroGraph?

NeuroGraph is an open-source temporal knowledge graph engine that extracts structured knowledge from unstructured text. It uses LLMs (or offline regex fallback) to identify entities and relationships, stores them with temporal metadata, and enables graph-powered retrieval-augmented generation (RAG).

## What can NeuroGraph do?

- Extract entities and relationships from text using LLMs or rule-based methods
- Store knowledge with bi-temporal validity windows
- Answer questions using hybrid search (semantic + keyword + graph traversal)
- Detect communities of related entities
- Visualize knowledge graphs in an interactive dashboard
- Operate fully offline without any API keys

## What are NeuroGraph's intended uses?

NeuroGraph is designed for:
- **AI agent memory** — persistent, structured memory for conversational agents
- **Knowledge management** — organizing and querying large bodies of knowledge
- **Research** — exploring temporal relationships in datasets
- **Developer tooling** — graph-powered RAG for applications

NeuroGraph is **not** designed for:
- Making autonomous decisions without human oversight
- Processing sensitive personal data without appropriate safeguards
- Replacing expert judgment in high-stakes domains (medical, legal, financial)

## How was NeuroGraph evaluated?

NeuroGraph has not yet been evaluated on standard academic benchmarks (e.g., DMR, LongMemEval). We plan to publish benchmark results as the project matures. Current testing is limited to unit tests and integration tests in the repository.

We welcome community contributions to benchmark evaluation.

## What are the limitations?

- **LLM dependency**: Entity extraction quality depends heavily on the underlying LLM. Regex fallback provides basic coverage but misses nuanced entities.
- **No formal ontology**: The system does not enforce ontological constraints by default. Extracted entity types may be inconsistent across documents.
- **Cost**: When using cloud LLMs, indexing and querying incur API costs. Use the built-in cost tracker to monitor spend.
- **Scale**: While the Rust core is fast, large-scale benchmarks (>1M nodes) have not been independently verified.
- **Bias**: LLM-based extraction may reflect biases present in the underlying model.

## How can users minimize risks?

- **Monitor costs**: Use the built-in cost tracker and set budget limits via `ng.budget(usd)`.
- **Verify outputs**: Do not rely solely on NeuroGraph answers for critical decisions. Cross-reference with primary sources.
- **Offline mode**: For privacy-sensitive data, use the zero-API-key mode with local regex NER and FastEmbed.
- **Start small**: Test with a representative sample of your data before full-scale indexing.
- **Review extracted knowledge**: Periodically review the knowledge graph for accuracy, especially in domains where precision matters.

## Data handling

- NeuroGraph processes data locally by default (embedded sled database).
- When cloud LLMs are configured (OpenAI, Anthropic, etc.), text is sent to those providers' APIs. Review their privacy policies.
- No data is sent to NeuroGraph maintainers or any telemetry service.
