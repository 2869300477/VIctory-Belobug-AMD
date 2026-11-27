# RX 9070 XT local observations (2026-11-27)

Machine: Ryzen 7 9700X, RX 9070 XT 16 GB, 24 GB RAM, Windows 11,
display driver 32.0.32029.2010. The earlier measurements below used a
diagnostic build and Series S, ray-tracing
builder off, renderer ports CONSTANTS, DESCRIPTORS, CBV, GUARD and JOURNAL
on. Shader and previously used pipeline caches were warm.

The current local preset uses Series X and a 60 FPS target at the user's
request. A native startup confirmed 3840x2160 presentation and all three
60 FPS timing patches, including simulation timing. Title reconstruction
is retained; the default 2/3 scale implies roughly 2560x1440 internal
rendering, rather than native 4K. The original Series S comparisons below
do not establish performance or memory use for this higher-quality mode.

The first 45-second Series X scene sample recorded a periodic median of
22.6 FPS (11.0–46.5), peak private commit 25853.5 MiB, peak working set
16497.3 MiB, and minimum free physical memory 383.0 MiB. It logged no
draw/indirect drops or device errors. The player reported that the locations
which had flickered before no longer did in this run. This is a visual
improvement observed with the new preset, not an identified root-cause fix
or proof for all scenes. Series X, title reconstruction and 60 FPS remain
the local defaults; memory pressure and heavy-scene FPS remain open.

## Retained settings

```powershell
$env:VICTORY_WINDOWS_BAR_MB = '3072'
$env:VICTORY_WINDOWS_BAR_PERCENT = '90'
$env:VICTORY_CARD_BUFFERS_MIN_KB = '128'
$env:VICTORY_CARD_BUFFERS_MB = '1536'
$env:VICTORY_STAGING_BLOCK_MB = '16'
$env:VICTORY_SEND_SPARE_MB = '256'
$env:VICTORY_FRAME_INTERVAL_MS = '16.667'
$env:VICTORY_UPSCALER = '6'
$env:VICTORY_RESIDENCY = '1'
$env:VICTORY_WC_NO_BACK = '1'
$env:VICTORY_SHARED_PRELUDE_ALLOCATORS = '1'
$env:VICTORY_LIST_LAZY_ATTACH = '1'
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
comparison. Raising the cap alone does not resolve the heavy-scene limit.
The local default is 60 FPS at the user's request, using
`VICTORY_FRAME_INTERVAL_MS=16.667` with the matching simulation-step patch;
subsequent performance runs keep it enabled. The captured final
seven seconds were inspected, without establishing that all intermittent
corruption is gone.

## Buffer-range scan experiment, not retained

An occupancy bitmap was tried to skip empty heap-buffer slots without
changing refresh frequency or ownership. Three focused checks passed, and
a synthetic span benchmark improved from 0.48 to 0.36 ms. The native
60 FPS opening did not demonstrate a benefit: periodic median 21.6 FPS,
with the late span/refresh phase averaging 4.79 ms against 3.43 ms in the
earlier run. The dynamic scene and memory conditions prevent attributing
the difference solely to this change. It was removed; no extra regression
runs were made. The final local executable uses the original scan.

## Window placement under the Series X budget

`VICTORY_WINDOWS_BAR_PERCENT` optionally raises the adapter-budget ceiling
for mapped windows from 80 percent to a value between 80 and 95. The window
byte cap is independent and unchanged; the native default remains 80.
The local preset selects 90 with its existing 3072 MiB cap. This is the
same placement control discussed upstream in PR 394, without its unrelated
texture trimming, residency or staging changes.

In a 45-second Series X sample, the final window spans were 4 MiB in system
memory and 2956 MiB in mapped VRAM, compared with 560 / 2208 MiB in the prior
80-percent run. Local VRAM was 12230 MiB against a 14272 MiB budget. Private
commit peaked at 25304.7 MiB, working set at 17335.5 MiB, and minimum free RAM
was 584.3 MiB. Median periodic FPS stayed 22.6; no draw drops or device errors
were logged. This directly confirms placement of more windows in VRAM,
not a precise net physical-RAM saving or a frame-rate improvement. System
memory pressure remains substantial.

## Selective copy-back experiment

The seven commits of upstream PR 477 (`0834177`, including PR 476's
protection statistics) were ported with author attribution. The opt-in
`VICTORY_WC_NO_BACK=1` leaves shader writes on card-backed, write-watched
copies of write-combined title memory, for declarations at least 64 KiB.
Other declarations retain ordinary copy-back. The native default remains
off; the local development preset selects it, with `-FullCopyBack` available
to restore the prior behavior. `VICTORY_WC_KEEP_STORES` remains off.

Before validation, the endpoint-only protection check was replaced with
full interval coverage, rejecting cached interiors, holes and address
overflow. Disabled census tags avoid TLS writes. Its page-guard test now
requests a free low address, so Windows cannot skip it merely because an
unconstrained allocation lies above the title's address ceiling. Five
range/protection checks and the real guard-handler check passed; the
diagnostic executable built successfully. No full suite was run.

An 85-second Series X opening with the access census and late image capture
had a periodic median of 28.0 FPS (3.9–49.2), no draw drops and no device
errors. Copy-back fell to about 0.2 MiB/frame and 1–2 ms in reported intervals,
versus tens of MiB and over 20 ms in the prior run. The census tracked about
639 MiB of ranges and recorded zero untagged page reads; title writes were
recorded. Final captured frames showed no obvious recurrence of the earlier
black patches. Peak private commit remained 26508.3 MiB, peak working set
17657.4 MiB, and minimum free RAM 511.4 MiB.

This is a development experiment: the dynamic scene, longer sample,
census and capture prevent a controlled FPS claim. Page guards observe the
first access between armings and do not cover every possible read or alias.
Write-combined protection itself does not prohibit CPU reads; no claim is
made that this optimization is correct for every scene. Normal local play
keeps the census off. Memory pressure and loading stalls remain unresolved.

## Shared state-prelude allocators

An isolated native allocation census found about 298.5 MiB of additional
private commit for 256 separately allocated command allocators/lists, each
recording 64 barriers. These created the same 32 MiB write-combined driver
allocation groups seen in the game. Eight texture heaps of 64 MiB produced
about 513 MiB of commit, accounting for another distinct size group. The
census is an ignored diagnostic, not a game benchmark or a unique-RAM measure.

`VICTORY_SHARED_PRELUDE_ALLOCATORS=1` gives the serially recorded state
preludes in one host submission a shared command allocator. The host retains
the allocator behind the direct queue's actual fence value, and resets it
only once that signal is complete. Every prelude still occupies its original
position before its title list, with its existing transitions and uploads.
Main recording allocators and the relay/check lists keep their ownership.
The native default is off; the local preset enables it, with
`-PrivatePreludeAllocators` restoring the prior state-prelude allocation.

A focused native test held the queue behind an unsignaled fence, verified
that an unfinished allocator was not reused, released the gate, and checked
reuse and the copy results of multiple lists. It also rejects the fence's
device-removed sentinel. The diagnostic build and a 45-second Series X
scene run passed, with zero draw/indirect drops and no device errors.

The run's shared pool stabilized at 17 allocators, reused 21,942 times by
the last memory report. Peak private commit was 23746.4 MiB, peak working
set 16610.9 MiB, minimum free RAM 1876.5 MiB, and periodic median 31.5 FPS
(10.4–47.9). The preceding ordinary-buffer-residency experiment used
25305.7 MiB peak commit, 17664.9 MiB peak working set, minimum free RAM
620.1 MiB and median 30.5 FPS. That residency experiment did not reduce the
large allocation groups and was removed. These dynamic runs are not a
controlled FPS estimate; the allocator count and lower memory requirement
support retaining the shared-prelude change. No screenshots or flicker
checks were made, following the player's request.

## Deferred replacement of detached lists

`VICTORY_LIST_LAZY_ATTACH=1` takes a submitted list's host recording out
without immediately acquiring its replacement. The next Reset acquires
an already-open pool entry or a new host recording. This preserves pool
capacity, completed-submission retirement, command ordering and upload/
descriptor ownership; it does not include PR 335's allocator-age retirement
or smaller pool limits. The native default is off. Local play enables it;
`-EagerListReplacements` restores immediate replacement.

A targeted native test kept the detached recording alive while resetting
the original list, verified independent deferred writes, then exercised
both ordinary and split detachment. It passed in 0.07 seconds. A 45-second
Series X run ended with 1179 host lists (1441 in the prior shared-prelude
run), with the count nearly stable after world entry. Peak private commit
was 23326.0 MiB, peak working set 16227.2 MiB, minimum free RAM 2325.0 MiB.
No draw/indirect drops or device errors were logged. Median periodic FPS
was 27.3 (14.7–33.8), below the previous run; only the smaller retained-list
set and memory requirement are claimed as improvements. No image capture
or visual validation was performed. The executable is rebuilt with the
option's disabled path preserving the previous Reset behavior.

## Shared main command storage

`VICTORY_SHARED_RECORDING_ALLOCATORS=1` separates the native main list from
its command allocator. Up to 32 closed recordings share an allocator, with
one exclusive open recording at a time. Each list retains a lease through
its existing retirement path, including unsubmitted and replayable lists.
An allocator resets only after all leases are gone and the host direct
queue's actual fence has passed every execution recorded in that group.
Idle lists stay closed without a lease; taking one from the existing pool
opens it with a new lease. Pool size, retirement delay and execution order
are unchanged. The native default remains off; local Play enables this
with `-PrivateRecordingAllocators` available to restore private storage.

A focused native test checked exclusive open recording, shared closed
recordings, replay after a completed execution, and retirement while a
second execution was blocked behind an unsignaled fence. It checked the
copied values after reuse and inspected D3D12 debug-layer errors. The
existing detach/reset/recycle test also passed with sharing enabled and
debug-message checking. Their execution times were 0.08 and 0.07 seconds;
the diagnostic build succeeded. No full suite or visual check was run.

The 45-second Series X / 60 FPS opening (`223411`) stabilized at 76 main
allocators for about 1250 native lists, with 335751 shared acquisitions
and 11704 group resets at the last memory report. Peak private commit was
22405.5 MiB, working set 15544.8 MiB, and minimum free RAM 2533.0 MiB.
The preceding ordinary run (`221603`) used 23326.0 / 16227.2 MiB and had
2325.0 MiB minimum free RAM. No draws/indirect work were dropped and no
device errors were logged. Periodic median FPS was 29.3 (19.3–39.7), versus
27.3 previously; this dynamic opening is not a controlled FPS comparison.
Only the smaller allocator set and lower observed memory peaks support
retaining the change. Heavy-scene performance still falls short of 60 FPS.

Earlier GPU timestamps (`222029`, with CPU/timeline diagnostics) showed
roughly 25–28 ms of GPU work in heavy shots. The `222421` experiment kept
only DRED page-fault reporting instead of automatic breadcrumbs, but still
reported roughly 26–28 ms in heavy shots. Its periodic median was 32.7 FPS
and peak commit 23466.0 MiB; differing diagnostic overhead and the dynamic
scene prevent a causal FPS claim. Normal Play keeps DRED Light. Actual
viewport diagnostics observed 2560×1440 scene rendering and 3840×2160 output.
