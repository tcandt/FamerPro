# ADR-005: Local-First SQLite WAL Persistence & Audit

## Status
Accepted

## Context
FarmerPro operates locally without requiring Docker, cloud services, or external database servers. The persistence layer must store workflows, immutable published snapshots, job states, step-by-step audit events, checkpoints, and device lease records with high concurrency and resilience to abrupt shutdowns.

## Decision
1. **SQLite with WAL Mode**: Use SQLite with `PRAGMA journal_mode = WAL` and `PRAGMA foreign_keys = ON` for fast concurrent reads and ACID transactions.
2. **Versioned Forward Migrations**: Database schema upgrades are managed via sequential migration scripts (e.g. `001_initial_schema.sql`).
3. **Data Redaction & Retention**: Sensitive fields (tokens, secrets) are referenced by ID into the OS credential store (Windows Credential Manager) and never logged in plain text. Screenshots stored for verification follow a strict retention policy.
4. **Draft vs. Immutable Version**: Workflow editing operates on `draft_json` in the `workflows` table. Execution requires publishing a snapshot into `workflow_versions` with a SHA-256 integrity hash, preventing runtime race conditions when an operator edits a live workflow.

## Consequences
- Fast, zero-configuration local installation.
- Reliable recovery from crash/power loss via SQLite WAL.
- Strict auditability: all step executions, inputs, and receipts are recorded in `step_events`.
