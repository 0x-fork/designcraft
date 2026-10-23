# Placed graphic layer options

IDML `GraphicLayerOption` / `GraphicLayer` settings are retained on each
`Graphic.layer_options`, not on the shared asset. Two placements of one linked
file can therefore keep different layer choices while sharing image bytes.
Scalar option/layer attributes, layer order, layer IDs/names, original/current
visibility, link-update policy and view/export/print flags survive import,
native save/reopen and IDML export. XML Self IDs are regenerated. Native-edited
attribute maps cannot inject XML syntax or override those Self IDs.

This is preservation, **not layer-aware rendering**. The current renderer and
PDF exporter use the asset's composite/default appearance and do not apply
these per-placement choices. Changed visibility and explicit view/export/print
states produce an import warning, a persistent `unsupportedGraphicLayers`
preflight issue, and a PDF export warning. Nested graphic cells, anchored
objects and parent items remain detectable after native reopening.

Native files containing these settings require format version 4. Earlier
readers reject that version instead of loading and silently discarding the new
field. Documents without layer settings retain their existing version 2/3
requirements. Older graphics without `layerOptions` still deserialize with
`None`, and their JSON representation is unchanged.

Synthetic regressions cover independent overrides on two placements sharing
one asset, metadata round trips, native reader requirements, nested cells,
preflight/PDF warnings and invalid native attribute names. Private originals
and artwork are not repository fixtures; any collection results are local
metadata checks, not an independent reference-rendering or fidelity oracle.

Local collection check (2026-10-23): a five-document pilot passed before
expansion to 205 previously converted documents. All 205 completed IDML →
native save/reopen → IDML; all 1,113 layer records across 68 documents matched
their input scalar attributes and order, ignoring regenerated Self IDs.
Persistent preflight warnings appeared in the 57 documents with overrides.
The private source files, layer names, document paths and per-file logs remain
outside this repository. Original INDD files were not opened by this check.

Remaining work: apply visibility to PSD layers/PDF optional content, implement
layer-comp choices and a layer-options UI, and validate appearance against
verified same-save reference PDFs. Gap 19 remains open.
