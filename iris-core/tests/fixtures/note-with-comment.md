---
# YAML comment — must survive round-trip
id: 01JQZ8NOTEID000000000000CD
type: note
created: 2026-01-15T11:00:00Z
modified: 2026-01-15T11:00:00Z
schema_version: 1
domain: iris-dev
tags: [sync, crdt]
distillation_level: raw
relations:
  - type: related-to
    target: 01JQZ8TASKID000000000000EF
---

Naive sync ships first, CRDT later. Conflict copies mean no silent data loss.
