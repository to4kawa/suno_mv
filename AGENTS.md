\# AGENTS



\## Purpose



This document defines how AI agents should work in this repository.



Project contracts are defined under:



\- contracts/



Architecture documents remain the source of truth.



\---



\## Workflow



Before making any changes:



1\. Read:

&#x20;  - AGENTS.md

&#x20;  - contracts/

&#x20;  - docs/adr/

&#x20;  - docs/architecture/

&#x20;  - docs/protocol/

&#x20;  - docs/implementation/

&#x20;  - docs/reviews/ (if relevant)



2\. Summarize your understanding.



3\. Propose an implementation plan.



4\. Wait for approval if architecture changes.



5\. Implement in small reviewable commits.



6\. Summarize completed work.



7\. Recommend manual verification steps.



\---



\## Development Principles



Do not break existing workflows.



Prefer incremental changes.



Implement small reviewable commits.



Avoid unrelated refactoring.



Introduce isolated module skeletons before integrating them into the application.



Integrate new components only after their interfaces have been reviewed.



\---



\## Coding Style



Prefer pure functions.



Separate protocol parsing from transport.



Keep UI and backend loosely coupled.



Avoid global mutable state.



\---



\## Testing



Prefer targeted unit tests.



Do not automatically run long-running build or test commands unless explicitly requested.



Summarize recommended manual verification after implementation.



\---



\## Review



If implementation conflicts with a contract or architecture document:



Stop.



Explain the conflict.



Do not guess.

