# RX 9070 XT local observations (2026-11-27)

Machine: Ryzen 7 9700X, RX 9070 XT 16 GB, 24 GB RAM, Windows 11,
display driver 32.0.32029.2010. Diagnostic build, Series S, ray-tracing
builder off, renderer ports CONSTANTS, DESCRIPTORS, CBV, GUARD and JOURNAL
on. Shader and previously used pipeline caches were warm.

## Retained settings

```powershell
$env:VICTORY_WINDOWS_BAR_MB = '3072'
$env:VICTORY_CARD_BUFFERS_MIN_KB = '128'
$env:VICTORY_CARD_BUFFERS_MB = '1536'
$env:VICTORY_STAGING_BLOCK_MB = '16'
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

With the earlier 1 MiB buffer threshold, default 2048 MiB card-buffer cap
and 64 MiB upload blocks on runtime sources at `c00861d`, a 100-second opening
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

## Smaller buffer comparisons on the GPU

The later local preset uses a 128 KiB threshold and 1536 MiB card-buffer
cap, following the configuration reported in
[issue 438](https://github.com/Belobug/VIctory-Belobug-AMD/issues/438).
With 16 MiB upload blocks and the normal eight recording workers, a
100-second opening run measured 29.15 median FPS (periodic ready-scene
statistics), 21838.5 MiB peak private commit, 16385.2 MiB peak working set,
and 2392.2 MiB minimum free physical memory. CPU comparisons were about
3.1 MiB/frame, snapshots 2.5 MiB/frame and 0.4 ms/frame. Observed local VRAM
use was about 10920 MiB against a 14355 MiB budget in one late interval.
No draw drops, indirect drops or device errors were logged.

The low periodic FPS was still 7.7, so this is not stable 30 FPS. Dynamic
opening samples and background system use limit comparison precision.
Separate experiments restoring the runtime's streaming-priority adjustment
and reducing recording workers to six did not demonstrate a performance
benefit here and were not added to the local preset.

## Intermittent dark patches: captured, not fixed

Following the player's timing hint, capture was extended through the end
of the opening sample. Consecutive frames confirmed irregular black patches
on characters and occasional background loss followed by recovery. Earlier
captures had ended before this interval. These are distinct from the hair
noise improved by the reconstruction setting.

Three separate diagnostic runs retained the patches: all title buffers with
two sides (`VICTORY_CARD_ALL=1`), table constants copied at submission
(`VICTORY_PORT_CBV_PLACE=0`), and the graphics heap copied/synchronized
instead of mapped in place (`VICTORY_PORT_CONSTANTS=0`). None is added to the
local preset as a fix. This does not rule out other synchronization or
shader faults; it rules out each of these switches as a sufficient remedy.
Captured images and logs remain outside the repository.

## Raw twins in the descriptor cache

The failing interval logged approximately 250–290 null raw twins per frame.
[Upstream PR 437](https://github.com/avatarmanz/VIctory-7900XTX/pull/437)
identified the same class of null view as a cause of foliage flicker on
Linux/RADV. Its direct unaligned-view creation was not adopted here.

The native Windows implementation already corrects a raw twin's address in
a thread's staging slot, after checking that the driver's descriptor bytes
match the expected GPU address and length. The recording cache constructed
views in its own slots, so they failed the staging-slot check and became
null whenever relocation was needed. Cache misses now build buffer views
through the existing staging path and copy the corrected pair into the
CPU-only cache. Hits retain the cached descriptors. The existing guard and
unsupported-driver fallback are unchanged; shader translation is unchanged.

The opt-in native AMD regression test
`cached_raw_twins_keep_the_typed_views_exact_byte_start` failed before the
change: a view starting at byte 4 became `(0, 0)` in the cache although the
ordinary descriptor had `(GPU address + 4, 64 bytes)`. It passes after the
change for byte offsets 4/8/12 and halfword offsets 2/6, on cache misses and
hits. This checks native descriptor contents, not a cross-vendor rendering
guarantee. Runtime visual validation is recorded separately.

The native run after this fix reported zero null twins throughout, including
about 310 misaligned twins/frame at the end. It captured 785 frames and
still showed intermittent character patches: the null-twin defect is fixed,
but it was not the only visual problem. The 110-second opening's periodic
median was 28.8 FPS, with no draw drops or logged device errors. Capture
overhead and the changing scene prevent an isolated performance claim.

## Unattended local startup

The local launchers use `VICTORY_KEYS_AFTER_WINDOW=1` with
`VICTORY_KEYS=10-11:Enter`: wait ten seconds after the visible presentation
window is available, then hold Enter/the mapped gamepad A button for one
second. The input hook logs pressed, released, or a missed interval. This
is diagnostic-build functionality. Without the new window-clock flag,
existing scripts retain their input-start clock.

A native startup logged the press at 10003 ms and release at 11005 ms from
window readiness and entered the story without manual input. The window
clock does not establish that an arbitrary title's menu is ready; the logs
make input delivery timing inspectable if a later build loads differently.

## CPU mapping attribution and spare submission uploads

With the resource book enabled, `VICTORY_MEMORY_OWNERS=1` associates live
CPU mappings with upload buffers, shared buffers, mapped card buffers and
shader-visible descriptor heaps. The existing memory census reports the
exposed spans separately from the Windows allocations containing their
starts. COM lifetime notices remove records without retaining resources.
The census is concurrent: unmatched starts and untracked driver allocations
are reported, and neither committed address space nor exposed spans are a
measure of unique physical RAM.

An opening census found 1562 MiB of upload mappings, including a single
512 MiB allocation. `VICTORY_SEND_SPARE_MB` now optionally bounds completed
submission uploads kept for reuse (0–4096 MiB). Unset or invalid values keep
the previous defaults: 768 MiB in system memory, 256 MiB with SEND_BAR.
The bound applies only after completion, when the existing pool receives
the buffer back. Allocation sizes, copying and submission ordering are
unchanged. Diagnostics report spare bytes, allocation/discard totals and
the largest recent request, so repeated allocations can be detected.

The local 24 GiB preset selects 256 MiB. In a 45-second scene sample, two
returned buffers totaling 768 MiB were discarded; allocation count settled
at 11 (968 MiB cumulatively allocated), and the retained set totaled 200 MiB.
Later requests peaked near 22 MiB in the reported intervals. The census
then found 1022 MiB of upload mappings; its largest was the remaining
128 MiB submission buffer. No continuing pool-allocation churn was logged.

The preceding default-budget sample and the bounded sample both had a
30.0 FPS median of nine periodic ready-scene statistics. Peak working sets
were 15715.6 and 15263.4 MiB; minimum free RAM 2470.3 and 3104.3 MiB;
peak private commit 21904.4 and 21724.7 MiB. Neither logged draw drops or
device errors. These are short, dynamic opening runs, so the process-wide
deltas and the FPS minima (23.0 and 16.5) are not controlled causal estimates.
The directly observed result is reduced retained upload capacity without
repeated allocation in that scene. Loading stalls and intermittent patches
remain unresolved.

The optional existing 60 FPS override was also tried with this pool budget.
All three loader timing checks applied, including the simulation's minimum
step; the runtime interval was 16.667 ms. Menus reached 60 FPS and a light
scene interval reached 52.8, but the 85-second opening had a periodic median
of 27.3 with loading/heavy-scene dips. Peak working set was 15366.5 MiB and
minimum free physical memory 3144.3 MiB. The upload pool still stabilized
(12 allocations, two discards); no draw drops or device errors were logged.
The final part also captured images, so this is not a controlled FPS
comparison. Raising the cap alone does not resolve the heavy-scene limit;
the local default retains the title's 30 FPS setting. The captured final
seven seconds were inspected, without establishing that all intermittent
corruption is gone.
