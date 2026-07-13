// What names a row, on this site: the triple, and never a slug.
//
// An implementation is `(workload, language, compiler, interpreter)` — that is what a
// `bench.yaml` declares, and it is what `report.md` puts in its columns. The
// harness also carries a `backend` slug on the wire, because its Markdown template
// needs a string to hang an anchor off; the site never shows it and never sorts on
// it. `java-native-image` reads as "java, native" and is in fact
// `java + native-image + no interpreter` — a slug is a name somebody typed, and a
// name that has to be decoded is worse than three fields that say it outright.
//
// So: the slug crosses the WASM boundary (`Selection` is deserialized straight
// into Rust, and Rust picks rows by it), and it stops there. Everything a reader
// sees, links to, filters or sorts by is the triple.

import type { Aggregate, FpMode, Row } from "./analysis";
import { NOT_AVAILABLE } from "./format";

/** The three fields a manifest declares. `compiler` and `interpreter` are each optional — not both. */
export interface Triple {
  language: string;
  compiler: string | null;
  interpreter: string | null;
}

/** An implementation, in a given FP mode: the thing a chart bar or a table row is. */
export interface Identity extends Triple {
  mode: FpMode;
}

/**
 * What ran the language: `gcc`, `javac + openjdk`, `cpython`, `cython + cpython`.
 *
 * An absence is a fact, not a hole — a backend that compiles ahead of time has no
 * interpreter, and saying so is the point of keeping the two fields apart. But a
 * label is not a table: here the absent half is simply not mentioned, because
 * `gcc + n/a` reads like a bug.
 */
export function toolchain(triple: Triple): string {
  const parts = [triple.compiler, triple.interpreter].filter(
    (part): part is string => part !== null,
  );
  return parts.length === 0 ? NOT_AVAILABLE : parts.join(" + ");
}

/** A row, in one line: `c · gcc`, `java · javac + openjdk`, `python · cython + cpython`. */
export function label(triple: Triple): string {
  return `${triple.language} · ${toolchain(triple)}`;
}

/** The same, with the FP mode that produced these numbers. Two modes are two measurements. */
export function labelWithMode(identity: Identity): string {
  return `${label(identity)} · ${identity.mode}`;
}

/**
 * Absent, spelled: `-` in a URL and in a `<select>`, `n/a` in a cell.
 *
 * It is a *value*, not the lack of one, and the difference matters in a filter:
 * "every compiler" and "the implementations that have no compiler" are two
 * different questions, and the second one has an answer — every ahead-of-time
 * backend in the table. So `null` means the filter is off, and this means the
 * filter is on and looking for an absence.
 */
export const ABSENT = "-";

/**
 * A row as the query string spells it: `c/gcc/-/strict`, `java/native-image/-/fast`.
 *
 * Four segments, in the order the report's columns run — language, compiler,
 * interpreter, mode — because a link is a claim somebody else has to be able to
 * read. `?a=java-native:strict` needs the reader to know which half of the slug is
 * the compiler; `?a=java/native-image/-/strict` says it.
 */
export function identityKey(identity: Identity): string {
  return [
    identity.language,
    identity.compiler ?? ABSENT,
    identity.interpreter ?? ABSENT,
    identity.mode,
  ].join("/");
}

/**
 * The `id` of an implementation's card, and what a table row links to.
 *
 * Derived from the triple, like everything else a reader can see: clicking a row
 * puts this in the address bar, and `#impl-java-native-image-none` says what it
 * points at where `#mandelbrot-java-native-image` — the harness's slug — would have
 * to be decoded. An absence is spelled `none` rather than dropped, so two rows that
 * differ only by an absent half cannot land on the same anchor.
 */
export function anchorId(triple: Triple): string {
  return ["impl", triple.language, triple.compiler ?? "none", triple.interpreter ?? "none"].join(
    "-",
  );
}

/** The triple of whatever the harness sent us, whatever else it carries. */
export function tripleOf(row: Triple): Triple {
  return {
    language: row.language,
    compiler: row.compiler,
    interpreter: row.interpreter,
  };
}

export function sameTriple(left: Triple, right: Triple): boolean {
  return (
    left.language === right.language &&
    left.compiler === right.compiler &&
    left.interpreter === right.interpreter
  );
}

/**
 * The row a key points at, in this campaign — or `null`.
 *
 * A key is an I/O boundary: it arrives from a query string somebody may have typed,
 * bookmarked before a backend was renamed, or copied from another campaign
 * entirely. So it is resolved against the aggregates rather than trusted, and a
 * triple this campaign never measured is dropped. Refusing a whole page over a
 * stale bookmark helps nobody.
 */
export function findByKey(aggregates: Aggregate[], key: string | null): Aggregate | null {
  if (key === null) {
    return null;
  }
  const [language, compiler, interpreter, mode] = key.split("/");
  if (language === undefined || mode === undefined) {
    return null;
  }
  return (
    aggregates.find(
      (row) =>
        row.language === language &&
        (row.compiler ?? ABSENT) === compiler &&
        (row.interpreter ?? ABSENT) === interpreter &&
        row.mode === mode,
    ) ?? null
  );
}

/**
 * The handle the WebAssembly picks a row by.
 *
 * The one place the slug is allowed to exist, and it exists for exactly as long as
 * a function call: `compare()` deserializes this into `src/compare.rs`'s
 * `Selection`, which finds the samples by the same field the harness bucketed them
 * under. It is never rendered, never put in a URL, never sorted on.
 */
export function wasmRow(row: Aggregate): Row {
  return { backend: row.backend, mode: row.mode };
}
