---
name: Documentation problem
about: Something documented here is wrong, unclear, or out of date
labels: documentation
---

**Where**
File and line if you have it.

**What it says, and why that is wrong**

Claims in this repository are meant to be machine-checked wherever possible, so
a documented claim that turns out to be false is a real bug — the benchmarks
were added after one such claim ("O(1) neighbour queries") turned out to be
misleading at the scale the demo actually runs.
