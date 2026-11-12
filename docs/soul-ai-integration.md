# SOUL AI integration boundary

**Status:** architecture plan only. The first bridge is read-only/proposal-only; no SOUL model is allowed to control a live Horizon session yet.

SOUL AI's current Phase 1 skills are trained for MiniCraft, not Minecraft. Their observation encoder expects a 7×7 MiniCraft tile patch, fixed MiniCraft inventory fields, relative mob data, and MiniCraft events; their output is a MiniCraft action integer. Horizon is a 3D Minecraft environment with different observations and control semantics. Passing Horizon data directly into the existing MiniCraft encoder or forwarding its action integers to game input would be an unsafe schema mismatch.

## Integration shape

- **Transport:** reuse Horizon's existing `horizon-launcher mcp` / `horizon-ctl mcp` surfaces. Do not create a second competing game-control protocol.
- **First milestone:** read-only MCP discovery and bounded observation collection. No account, launcher configuration, world, or gameplay mutation.
- **Adapter:** define `horizon.minecraft.observation.v1` and Minecraft-specific task/action vocabularies. Keep MiniCraft `T_wood`, `T_tools`, `T_iron` separate until a dedicated mapping is trained and validated.
- **Inference:** return a versioned `soul.horizon.proposal.v1` envelope with skill hash, observation sequence, candidate action, expected outcome, leaf support, confidence and provenance.
- **Safety:** Horizon computes its own independent gate from the latest state. SOUL cannot supply or override the gate decision. Stale observations, world/session changes, hazards, unsupported actions, user input and repeated no-progress all stop the loop.
- **Execution:** disabled by default. Only enable bounded single-step actions after Minecraft-specific held-out validation, hazard tests, explicit user opt-in and auditable replay tests.

## Evidence handling

The reported `T_wood` / `T_tools` / `T_iron` task-success values and `learned_agreement` values are separate metrics. The supplied package summary is not a substitute for artifact-level verification. Before packaging SOUL skills with Horizon, verify the ZIP's complete file inventory, manifest hashes, schemas, feature/action mappings and seed provenance. Do not interpret agreement with the mapped fallback as independent task success.

See the matching SOUL-side contract in `Soul-AI/docs/SOUL-HORIZON-INTEGRATION.md`. The contract is intentionally proposal-only until a Minecraft-specific adapter exists.
