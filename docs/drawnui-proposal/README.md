# NX graphics/UI proposal sources

The [main proposal](../NX-Drawn-UI-MVP-Object-Model-Proposal.md) defines the full target model. The `.nx` files provide illustrative declarations, not shipped standard libraries or a guarantee of current runtime support.

- `core/core.nx`: shared values and illustrative document shape.
- `graphics/graphics.nx`: retained geometry and positioned content.
- `ui/ui.nx`: later read-only measured layout/content.
- [Authoring and preview](authoring-and-preview.md): agent-oriented helper/tool contracts for the first slice.
- [Future design register](../NX-Drawn-UI-MVP-Object-Model-Future.md): decisions to settle before compatibility stabilization.
- [Initial OpenSpec change](../../openspec/changes/archive/2026-10-29-add-nx-graphics-core-and-direct-skia/proposal.md): the bounded implementation scope.

Production `@nx/graphics` must be standalone and independent of `@nx/ui`. The illustrative graphics-to-UI inheritance/import in these sources describes the larger embedding model and must not be copied as a runtime dependency for the first graphics slice. Current NX optional-property spelling and empty-sequence support should be checked against the compiler, rather than copied from Appendix B's historical notes.
