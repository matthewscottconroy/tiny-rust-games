# Benchmarks

Several demos make a claim about performance in their documentation. Until these
existed, nothing checked those claims — `missing_docs` keeps documentation
present, the catalogue check keeps it current, the tests keep behaviour correct,
and none of that says whether the fast path is actually fast.

```bash
cargo bench --manifest-path benchmarks/Cargo.toml --bench claims
just bench      # same thing
```

Numbers below are from one developer machine (Linux, x86-64) and are meaningful
as *ratios between the two implementations*, not as absolute timings.

## Spatial partitioning: the claim does not hold at the demo's own scale

`tech-demos/bevy/spatial-partitioning` describes "O(1) neighbour queries" and
displays a running comparison of brute-force pair count against spatial pair
count. Both halves of that are true. The conclusion a reader draws from it —
that the grid is therefore faster — is not, at the size the demo runs.

| Points | Brute force | Grid | Result |
|-------:|------------:|-----:|--------|
| 16 | 52 ns | 2.25 µs | grid **43× slower** |
| 64 | 685 ns | 10.0 µs | grid **15× slower** |
| 256 | 10.7 µs | 36.8 µs | grid **3.4× slower** |
| 1024 | 173 µs | 275 µs | grid **1.6× slower** |
| 4096 | 2.68 ms | 2.59 ms | grid 1.04× faster |

**The crossover is around four thousand points. The demo defaults to sixty.**

The grid does exactly what it claims: it performs far fewer distance
comparisons. It is slower anyway, because at these sizes the comparisons were
never the cost. Building a `HashMap` of buckets every frame — hashing, bucket
allocation, pointer chasing — costs more than simply comparing every pair of
sixty points, which is 1,770 comparisons of contiguous memory that the CPU
prefetches perfectly.

This is the useful lesson, and it is one the demo could not teach on its own:

- **an asymptotic win is not a win until N is large enough to pay for it.**
  O(N) with a large constant loses to O(N²) with a tiny one over a surprisingly
  wide range;
- **counting operations is not measuring time.** The demo's HUD is honest about
  comparison counts and silent about the allocation that dominates;
- a production version would reuse the bucket storage between frames rather than
  rebuilding it, which moves the crossover down a lot. That the *naive*
  implementation of a good technique loses is itself worth knowing.

The demo's documentation has been corrected to say this rather than implying a
speedup it does not deliver at its default settings. The technique is still
worth teaching — it is what you need at four thousand entities — and now it is
taught with the boundary attached.

## Snake simulation: enormous headroom

| Board | 200 steps |
|-------|----------:|
| 20×15 | 1.14 µs |
| 80×60 | 1.14 µs |

About 6 ns per step. `snake-lib` deliberately does no per-frame work — only
per-tick — and at nine ticks per second the simulation uses roughly five
millionths of a frame's budget. The interesting consequence is for replay and
netcode: stepping is so cheap that re-simulating from a seed is essentially
free, which is the property those features would rely on.

## Tic-tac-toe win detection: fine, and worth knowing why

`winner()` rescans the whole board on every call, and frontends call `status()`
every frame. Worst case is a full board with no winner, where nothing
short-circuits:

| Board | `winner()` |
|-------|----------:|
| 3×3 | 39 ns |
| 9×9 | 601 ns |
| 19×19 | 4.5 µs |

Even a 19×19 board costs 0.03% of a 16 ms frame. The straightforward
implementation is the right one, and now that is a measurement rather than an
assumption. Growth is roughly quadratic in board side, so this would want
revisiting if the library ever targeted much larger boards — Go's 361 points are
already at the edge of the table above.
