# Reports

One report per architecture, each rendered by `langbench md` from the campaign of the same
name in [`samples/`](../samples/) — never from anything else, and never by hand.

| Report | architecture | Campaign |
| --- | --- | --- |
| [`aarch64.md`](aarch64.md) | ARM64 | [`samples/aarch64.ndjson`](../samples/aarch64.ndjson) |
| [`x86_64.md`](x86_64.md) | x86-64 | [`samples/x86_64.ndjson`](../samples/x86_64.ndjson) |

A row whose link is dead is a campaign that has not been run yet — the
[`bench`](../.github/workflows/bench.yaml) workflow writes both files together, or
neither.

**The two are not comparable in absolute terms.** A millisecond on x86-64 and a
millisecond on aarch64 are not the same claim, so the two reports are never merged
into one table and the website never charts them together. Compare backends
*within* one architecture; the ratio is what travels across.
See [`METHODOLOGY.md#the-architecture-rule`](../METHODOLOGY.md#the-architecture-rule).

Each report leads with every reason its host was a poor benchmark target. Read
that part first: it is what tells you whether the numbers under it mean anything.
