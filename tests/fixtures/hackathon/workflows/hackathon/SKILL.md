---
name: hackathon
description: Orchestrates the hackathon workflow by polling Admiral for the next node and advancing it with the node's result.
---

# Hackathon orchestrator

You drive the run; Admiral owns the graph. Repeat until Admiral reports the run ended:

1. Run `admiral next --json`. It names the node and what running it requires.
2. Do exactly that: load the named skill, delegate to the named agent, ask the user
   the named question, or wait for a script Admiral runs itself.
3. Run `admiral advance --json` with the node's result. Admiral validates it and
   names the next node, or rejects it with a reason; fix and resend on rejection.

Never skip a node, never invent a result, and never ask the user anything the graph
does not ask: the planned interruptions are the mode question, the kickoff batch, the
idea pick and the submission review.
