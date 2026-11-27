# RX 9070 XT observations (2026-11-27)

Test machine: Ryzen 7 9700X, RX 9070 XT 16 GB, 24 GB RAM, Windows 11,
display driver 32.0.32029.2010. Diagnostic build, Series S, ray tracing
builder off, renderer ports CONSTANTS, DESCRIPTORS, CBV, GUARD and JOURNAL
on. Shader and previously used pipeline caches were warm.

## Data placement and reconstruction

The local launch preset uses:

```powershell
$env:VICTORY_WINDOWS_BAR_MB = '3072'
$env:VICTORY_CARD_BUFFERS_MIN_MB = '1'
$env:VICTORY_UPSCALER = '6'
$env:VICTORY_RESIDENCY = '1'
```

These are observations on this machine, not new global defaults. Existing
allocation budget checks and fallback paths remain in use.

- CPU-mapped VRAM windows reduced pressure on system memory. The placement
  diagnostic showed about 3 GiB of window spans there, with no declared
  shader writes to these windows during the observed opening. Span sizes
  are not driver allocation totals; this does not establish that arbitrary
  windows are read-only.
- Reducing the card-buffer threshold from 4 MiB to 1 MiB, with the default
  2048 MiB cap unchanged, moved the larger buffers' comparison work off the
  CPU. Near the end of these runs, CPU comparisons fell from about
  87.7 to 31.4 MiB/frame and snapshots from 74.4 to 28.1 MiB/frame.
  Snapshot time fell from about 9.2 to 3.8 ms/frame. Private commit did not
  decrease further in that comparison.
- The Series S FidelityFX path showed noisy hair. Selecting the title's
  own reconstruction (`VICTORY_UPSCALER=6`) visibly reduced the noise and
  flickering, confirmed by the player. This matches the same-card report
  in [issue 279](https://github.com/Belobug/VIctory-Belobug-AMD/issues/279).

With all three settings, a 100-second opening sample had a median of
26.75 FPS across periodic pipeline-ready statistics, 22327 MiB peak private
commit, 16614.5 MiB peak working set and 2212.8 MiB minimum free physical
memory. Draw and indirect-drop counts were zero. There were still loading
stalls and heavier scenes below 30 FPS. These changing opening sequences
are not fixed-scene A/B benchmarks or long-run stability evidence.

## Descriptor retirement

[Issue 438](https://github.com/Belobug/VIctory-Belobug-AMD/issues/438)
identified repeated descriptor allocator locking on another eight-core
RDNA 4 system. This tree now groups a finished list's dynamic descriptor
ranges by heap. It holds the live-heap registry once and each heap's
allocator once, including the cleanup path when a state is dropped.
Retirement still happens at the existing completion/reset/destruction
points. It does not return descriptors earlier.

The diagnostic line `dynamic descriptor releases per frame` reports both
range count and allocator-lock count. The focused allocator tests cover
held slots, fragmentation across the cursor, overlapping frees, clipping
and overflow boundaries. Only those three tests and a diagnostic build
were run for this code change.

In a subsequent 100-second opening run, one reporting interval retired
10265 ranges with 82 allocator locks. The run's median was 26.25 FPS,
against 26.75 before the change; this does not demonstrate an FPS gain.
No dynamic allocation refusals or draw drops were logged. The player
reported characters alternating between lit and black late in the run,
so visual correctness remains unresolved and is being investigated
separately from the lock-count reduction.
