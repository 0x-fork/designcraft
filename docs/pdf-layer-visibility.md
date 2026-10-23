# Placed PDF layer visibility

Object Layer Options distinguishes a PDF's own default configuration from an explicit visibility choice, including showing every layer. Use File Defaults resets that choice. Only placed PDFs in a mixed selection are changed.

The engine command `object.layerOptions` accepts `hidden` names and optional `useDefaults`. Existing calls with `hidden: []` retain their reset behavior; pass `useDefaults: false` to explicitly show all layers. `useDefaults: true` resets defaults and cannot be combined with hidden names. Queries include `overridden` separately from each layer's `visible` and `default` states.

Native saves use format 4 only for an explicit all-on override: older readers would otherwise silently render file defaults. Existing nonempty hidden lists retain their older semantics and format requirements. Raster and PDF export honor the choice; PDF export keeps its existing warned rasterization behavior. Embedded IDML export stores a separate PDF variant per placement and leaves original linked bytes untouched. External-only IDML export rejects active PDF layer choices because the original external file would lose the choice.

This fixes visibility-state handling, not full InDesign layer/print parity. It does not implement editing the contents of placed PDFs or native INDD placement. Catalog/reference safety fixes are tracked separately. The concurrent graphic-layer metadata PR also changes native format capability; its version selection must be reconciled when merging both features.
