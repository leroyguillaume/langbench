---
title: Statistics
order: 8
summary: Why the minimum is the estimate, why the dispersion is a verdict, and when a gap is not a difference.
---

## Why min-of-N, not the median

Contention noise is **one-sided**: it can only slow you down, never speed you up.
The median of a distribution pushed against a hard floor is not the true value.

So we report the **minimum** as the estimate of the machine's capability, and we
keep the dispersion beside it as a **quality signal for the campaign**. If the
spread is wide, the measurement is worthless — including its minimum. The
dispersion is not an error bar on the result; it is a verdict on the run.

## A difference smaller than the dispersion is not a difference

A table of minima invites one operation, and every reader performs it: divide two
rows. That ratio is the only cross-backend claim this project publishes — and the
campaign is entitled to it **only when the gap survives its own noise**.

Two rows whose minima differ by 3%, on a campaign whose rows each wobble by 9%,
are not a 3% result. They are the same number, measured twice, on a machine that
was busy. The minimum of one happened to fall lower than the minimum of the other,
and a second campaign on the same hardware would as happily reverse them. So the
verdict is a **tie**: not *equal* — *indistinguishable*, which is a statement about
this campaign and not about the backends.

The bar a gap has to clear is the **worse** of the two rows' dispersions: a claim
about a pair is only as defensible as its shakier half. And a row with fewer than
three samples has **no known dispersion** — the median absolute deviation of two
observations is structurally zero, and a structural zero is not a quiet machine.
It buys the pair no tolerance.

This is why the site's head-to-head is computed in `src/compare.rs` and not in the
browser that displays it. What counts as a difference is a definition of what this
project measures, and it lives beside min-of-N — one definition, one place, for the
same reason the site does not re-implement the statistics it plots.

A benchmark that reports `1.03×` where it should report *we cannot tell* has not
made a small error. It has made the only error that matters.
