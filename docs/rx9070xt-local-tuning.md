# RX 9070 XT local observations (2026-11-27)

Machine: Ryzen 7 9700X, RX 9070 XT 16 GB, 24 GB RAM, Windows 11,
display driver 32.0.32029.2010. Diagnostic build, Series S, ray-tracing
builder off, renderer ports CONSTANTS, DESCRIPTORS, CBV, GUARD and JOURNAL
on. Shader and previously used pipeline caches were warm.

## Retained settings

```powershell
$env:VICTORY_WINDOWS_BAR_MB = '3072'
$env:VICTORY_CARD_BUFFERS_MIN_MB = '1'
$env:VICTORY_UPSCALER = '6'
$env:VICTORY_RESIDENCY = '1'
```

These local settings do not change the runtime's global defaults.
Existing allocation budget checks and fallback paths remain enabled.

- About 3 GiB of window spans moved to CPU-mapped VRAM, relieving system
  memory pressure. No shader writes were declared for those windows in
  the observed opening. Span sizes are not driver allocation totals.
- Lowering the card-buffer threshold from 4 MiB to 1 MiB, with the default
  2048 MiB cap unchanged, moved larger buffers' comparison work off the
  CPU. Near the end of the measured openings, CPU comparisons fell from
  about 87.7 to 31.4 MiB/frame and snapshots from 74.4 to 28.1 MiB/frame.
  Snapshot time fell from about 9.2 to 3.8 ms/frame. This change did not
  further reduce peak private commit in that comparison.
- Selecting the title's own reconstruction (`VICTORY_UPSCALER=6`) visibly
  reduced hair noise and flickering. The player confirmed a clear
  improvement and no character problem in that run. This matches the
  same-card report in [issue 279](https://github.com/Belobug/VIctory-Belobug-AMD/issues/279).

With these settings on runtime sources at `c00861d`, a 100-second opening
sample had a median of 26.75 FPS across periodic pipeline-ready statistics,
22327 MiB peak private commit, 16614.5 MiB peak working set and 2212.8 MiB
minimum free physical memory. Draw and indirect-drop counts were zero.
Loading stalls and heavier scenes below 30 FPS remain. The changing opening
is not a fixed-scene A/B benchmark or long-run stability evidence.

## Reverted experiments

Batching descriptor retirement by heap (`5850f75`, motivated by
[issue 438](https://github.com/Belobug/VIctory-Belobug-AMD/issues/438)) passed
three focused allocator tests. A short run reduced allocator lock calls
from one per range to 82 for 10265 ranges in one reporting interval,
but did not establish a net FPS gain.

The upstream in-place indirect-argument ordering change
[`631400f`](https://github.com/avatarmanz/VIctory-7900XTX/commit/631400f598e2ab5002bcbcfab8f89c4dff7d70f1),
locally `a10564c`, was followed by a report of characters alternating
between lit and black. The player clarified that the hair reconstruction
change looked correct and that flickering appeared during the ordering
experiment. This does not establish that descriptor batching caused it.
Both changes were reverted: ordering for the visual failure, batching
because its FPS benefit remained unproven. Runtime sources and settings were
restored to the earlier configuration; neither experiment is a delivered
performance or visual fix. Zero draw-drop counts and successful unit tests
were insufficient to establish visual correctness.

Later observations also found intermittent local flickering on the restored
baseline, at varying locations for roughly ten seconds of a whole run.
The earlier ordering experiment therefore does not establish the cause.
The reconstruction setting improves hair, but is not a complete flicker fix.

## Upload-pool fragmentation

`VICTORY_STAGING_BLOCK_MB` optionally selects 8, 16, 32 or 64 MiB blocks for
the system-memory upload pool. Unset or invalid values retain 64 MiB.
The texture heap pool stays at 64 MiB. Uploads larger than half a pool
block still use standalone buffers; live-piece ownership and the existing
2500 ms reuse delay are unchanged.

The `staging pool` diagnostic separates bytes still owned, returned pieces
waiting for the reuse delay, and reusable ranges. It also reports the
subset of reusable bytes in wholly empty blocks. Standalone buffers are
excluded. For example, one 64 MiB-pool sample held 640 MiB: 47.6 MiB still
owned, 18.8 MiB waiting, and 573.6 MiB reusable, of which only 64 MiB was in
a whole empty block. Dropping empty blocks alone cannot reclaim the holes.

In short opening samples, the final ten pool statistics averaged 492.8 MiB
with 64 MiB blocks and 278.4 MiB with 16 MiB blocks. Still-owned pieces were
about 50 MiB in both. The 16 MiB run also captured frames, so this is evidence
of reduced pool storage, not an FPS comparison or a precise change in total
process memory. Three focused allocator tests cover the smaller block
boundary, non-overlap, protected live/resting pieces and texture-heap age.

The pop watcher now reads Series S's composed back buffer as well as ordinary
single-texture output. A bounded run produced 1070 local frames. Inspected
large changes included credits, camera cuts and police lighting; these do
not identify the intermittent rendering fault reported by the player.
