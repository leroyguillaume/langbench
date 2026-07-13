// One campaign, read out: the tiles, the charts, the table, and what it lost.
//
// **Which campaign is the route's business, not this island's.** The page is
// `/workloads/<workload>/<architecture>/`, and the two names arrive as props — so
// there is no campaign selector here, and there cannot be two architectures on one
// screen. The architecture rule is enforced by the shape of the site rather than by a
// note under a chart.
//
// The campaign is then picked by what its *header* says, never by the file whose name
// matched: a path is a label somebody typed, and the header is what the run recorded.
//
// The head-to-head is a page of its own, and this one links to it carrying the
// campaign — never the filters. A pair is not a table, and a reader who narrowed this
// table to one language has not thereby declined to compare it with another.

import { type ReactNode, useEffect, useMemo, useState } from "react";
import type { Aggregate, Analysis, Failure, LoadedCampaign } from "../analysis";
import { useCampaigns } from "../campaigns";
import { bytes, dispersion, mebibytes, milliseconds, optional, ratio } from "../format";
import { anchorId, label, labelWithMode, toolchain } from "../identity";
import { modeSeries, SEQUENTIAL, WALL_SERIES } from "../series";
import { compareHref, type ResultsState, readResults, writeResults } from "../url";
import { BarChart, type ChartRow } from "./BarChart";
import { FilterBar } from "./FilterBar";
import { filterRows, ResultsTable, type Sort, type SortKey, sortRows } from "./ResultsTable";
import { Warmup, WarmupBanner } from "./Warmup";

interface ResultsProps {
  /** The workload this campaign measured, from the route. */
  workload: string;
  /** The architecture it measured it on, from the route. */
  architecture: string;
  /**
   * The column reference, rendered from `methodology/columns.md` by Astro and slotted
   * into this island as HTML. It is prose, it is the same prose the methodology
   * publishes as a page of its own, and it is built at build time — the browser gets
   * no Markdown and no renderer for it.
   */
  columns?: ReactNode;
}

export function Results({ workload, architecture, columns }: ResultsProps) {
  // The URL is read once, on mount, and written on every change: the address bar
  // describes what is on screen, and a link to it puts somebody else in front of
  // the same claim. The *campaign* is not in the query string — it is the path.
  const [state, setState] = useState<ResultsState>(readResults);
  const { campaigns, error, pending } = useCampaigns(state.includeWarmup);

  useEffect(() => writeResults(state), [state]);

  if (error !== null) {
    return (
      <main className="page">
        <div className="warnings">
          <h2>This campaign could not be read</h2>
          <p>{error}</p>
        </div>
      </main>
    );
  }
  // Only when there is nothing to show at all. A *re*-aggregation keeps the previous
  // numbers on screen and dims them: tearing the page down and putting it back
  // collapses the document, and the browser takes the reader's scroll position with
  // it.
  if (campaigns === null) {
    return <p className="status">Reading the campaign…</p>;
  }

  // The campaign this route names, identified by its own header. No fallback: a page
  // that quietly rendered a *different* campaign than the one in its address would be
  // publishing a number under the wrong machine's name, which is the one mistake this
  // whole project is arranged to prevent.
  const loaded = campaigns.find(
    (entry) =>
      entry.analysis.architecture === architecture &&
      entry.analysis.campaign.workload.id === workload,
  );
  if (loaded === undefined) {
    return (
      <main className="page">
        <div className="warnings">
          <h2>This campaign is not in the build</h2>
          <p>
            No published campaign says it measured <code>{workload}</code> on{" "}
            <code>{architecture}</code>. The routes come from the campaign files, so this means the
            file that produced this page carries a header naming something else.
          </p>
        </div>
      </main>
    );
  }

  return (
    <Report
      loaded={loaded}
      workload={workload}
      state={state}
      setState={setState}
      columns={columns}
      pending={pending}
    />
  );
}

interface ReportProps {
  loaded: LoadedCampaign;
  workload: string;
  state: ResultsState;
  setState: (state: ResultsState) => void;
  columns?: ReactNode;
  /** The harness is re-aggregating; these numbers are the previous ones. */
  pending: boolean;
}

function Report({ loaded, workload: id, state, setState, columns, pending }: ReportProps) {
  const { analysis } = loaded;
  const { campaign } = analysis;

  const workload =
    analysis.workloads.find((entry) => entry.workload === id) ?? analysis.workloads[0];
  const aggregates = useMemo(() => workload?.aggregates ?? [], [workload]);

  // What the filters left standing, in the order the harness ranked it: fastest
  // first, on the statistic the report headlines. This is what the charts draw.
  const filtered = useMemo(
    () => filterRows(aggregates, state.filters),
    [aggregates, state.filters],
  );

  // And this is what the table shows. The sort belongs to the *table* — it is the
  // reader asking to read the same rows in another order, not asking a different
  // question. A chart that reordered its bars every time a column header was
  // clicked would be redrawing four figures to answer a question nobody asked of
  // them, and a `Binary` sort would rank the run chart by binary size, which is a
  // chart that says something it does not mean. The filters do scope the charts:
  // those change *which* rows are on screen, and that is a different claim.
  const visible = useMemo(() => sortRows(filtered, state.sort), [filtered, state.sort]);

  // The failures narrow with everything else. A reader who filtered down to
  // `python` and sees no failures has been told something true only if the filter
  // reached this table too.
  const failures = useMemo(
    () =>
      filterRows(
        analysis.failures.filter((failure) => failure.workload === workload?.workload),
        state.filters,
      ),
    [analysis.failures, workload, state.filters],
  );

  const onSort = (key: SortKey) => {
    // Second click on the same column reverses it; a new column starts ascending,
    // which for a timing means fastest-first — the order the report ranks in.
    const sort: Sort =
      state.sort.key === key
        ? { key, descending: !state.sort.descending }
        : { key, descending: false };
    setState({ ...state, sort });
  };

  const series = modeSeries(state.filters.modes);

  // Grouped: one row per implementation, one bar per mode. The bars a filter left
  // standing keep their own colour — the series is the mode, not the position.
  const runRows: ChartRow[] = useMemo(() => {
    const grouped = new Map<string, ChartRow>();
    for (const row of filtered) {
      const key = row.backend_id;
      const existing = grouped.get(key) ?? {
        key,
        label: label(row),
        values: state.filters.modes.map(() => null),
      };
      const index = state.filters.modes.indexOf(row.mode);
      if (index >= 0) {
        existing.values[index] = row.run_wall?.min ?? null;
      }
      grouped.set(key, existing);
    }
    return [...grouped.values()];
  }, [filtered, state.filters.modes]);

  const chartRow = (row: Aggregate, values: (number | null)[]): ChartRow => ({
    key: `${row.backend_id}-${row.mode}`,
    label: labelWithMode(row),
    values,
  });

  const wallRows: ChartRow[] = filtered.map((row) =>
    chartRow(row, [row.run_elapsed?.min ?? null, row.run_startup?.min ?? null]),
  );

  const buildRows: ChartRow[] = filtered
    .filter((row) => row.build_elapsed !== null)
    .map((row) => chartRow(row, [row.build_elapsed?.min ?? null]));

  const sizeRows: ChartRow[] = filtered
    .filter((row) => row.binary_bytes !== null)
    .map((row) => chartRow(row, [row.binary_bytes]));

  const memoryRows: ChartRow[] = filtered
    .filter((row) => row.run_peak_bytes !== null)
    .map((row) => chartRow(row, [row.run_peak_bytes?.min ?? null]));

  // What the head-to-head has to be handed: which campaign these rows came from. The
  // filters do not travel — they narrow a table, and a pair is not a table — but a
  // "Compare" link that quietly switched campaign would be inviting exactly the
  // comparison the architecture rule forbids.
  const scope = {
    architecture: analysis.architecture,
    workload: workload?.workload ?? null,
    includeWarmup: state.includeWarmup,
  };

  return (
    <main className={pending ? "page recomputing" : "page"} aria-busy={pending}>
      <header className="masthead">
        <h1>
          {campaign.workload.id} on {analysis.architecture}
        </h1>
        <p>
          Every backend of the{" "}
          <a href={`${import.meta.env.BASE_URL}workloads/${campaign.workload.id}/`}>
            {campaign.workload.id}
          </a>{" "}
          workload, measured on <strong>{analysis.architecture}</strong>
          {analysis.hostname !== null && ` (${analysis.hostname})`} on{" "}
          {new Date(campaign.timestamp).toLocaleDateString()}. Every number below is derived from
          the raw samples by the harness itself — the site computes no statistic of its own. The
          timings on this page are comparable to each other and to nothing else: an absolute timing
          does not cross an architecture, and a ratio is what travels.
        </p>
      </header>

      {analysis.warnings.length > 0 && (
        <section className="warnings">
          <h2>This host was not a clean benchmark target</h2>
          <ul>
            {analysis.warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </section>
      )}

      {/* The tiles are a fastest, a spread and a worst — none of which has an
          order. They read the filtered rows, so a sort cannot change a headline. */}
      <Tiles rows={filtered} analysis={analysis} />

      <FilterBar
        filters={state.filters}
        onFilters={(filters) => setState({ ...state, filters })}
        rows={aggregates}
      />

      {/* Not in the filter bar, deliberately: a filter changes which rows you are
          looking at, and this changes what the numbers *are* — the harness aggregates
          the campaign again. Different act, different place, and it says what it does. */}
      <Warmup
        rounds={campaign.warmup_rounds}
        includeWarmup={state.includeWarmup}
        onChange={(includeWarmup) => setState({ ...state, includeWarmup })}
      />

      {state.includeWarmup && campaign.warmup_rounds > 0 && (
        <WarmupBanner rounds={campaign.warmup_rounds} />
      )}

      <section className="card">
        <h2>Run — external wall-clock, min of {campaign.rounds}</h2>
        <p>
          Container creation, runtime init and compute. The minimum, because contention noise is
          one-sided: it can only ever slow a run down.
        </p>
        <BarChart rows={runRows} series={series} format={milliseconds} />
      </section>

      <section className="card">
        <h2>Where the wall-clock goes</h2>
        <p>
          The gap between the program's own clock and ours is startup — container creation plus
          whatever the runtime does before <code>main</code>. It is a property of the backend, and
          it is a result.
        </p>
        <BarChart
          rows={wallRows}
          series={WALL_SERIES}
          format={milliseconds}
          stacked
          total={(values) =>
            milliseconds(values.reduce<number>((sum, value) => sum + (value ?? 0), 0))
          }
        />
      </section>

      <section className="card">
        <h2>Compile — the compiler's own clock, min of {campaign.build_rounds}</h2>
        <p>
          Measured inside the container, so it times the compiler and not Docker. A{" "}
          <code>docker run</code> costs several times a <code>gcc</code> invocation on one file.
        </p>
        <BarChart rows={buildRows} series={SEQUENTIAL} format={milliseconds} />
      </section>

      {memoryRows.length > 0 && (
        <section className="card">
          <h2>Peak memory — the whole container, min of {campaign.rounds}</h2>
          <p>
            The high-water mark of everything inside the container, the runtime included. The
            minimum, and here the argument is exact rather than statistical: page cache and a lazy
            collector can only ever push a peak <em>up</em>, never below what the backend actually
            had to allocate.
          </p>
          <BarChart rows={memoryRows} series={SEQUENTIAL} format={mebibytes} />
        </section>
      )}

      <section className="card">
        <h2>Binary size</h2>
        <p>
          What the image ships. Absent for an implementation that compiles nothing ahead of the run.
        </p>
        <BarChart rows={sizeRows} series={SEQUENTIAL} format={bytes} />
      </section>

      <section className="card">
        <h2>Every number</h2>
        <p>
          Nineteen columns, and none of them mean what you would guess from the name alone. If this
          is your first benchmark table, read <a href="#columns">what each column means</a> — it is
          written for exactly that, and it starts with how to read a row in thirty seconds. The
          short version: look at <strong>Dispersion</strong> first, because nothing else on a row
          can be more trustworthy than it.
        </p>
        <p className="card-aside">
          Two columns are the site's own and are not in that reference. <strong>Ratio</strong> is
          how many times slower a row is than the fastest row <em>currently on screen</em> — filter
          the table and the baseline moves, because a baseline you cannot see is not a baseline.{" "}
          <strong>Δ strict</strong> is how far this row's answer landed from the <code>strict</code>{" "}
          reference: <code>0</code> means the same answer to the bit, which every{" "}
          <code>strict</code> row is obliged to produce. Click any header to sort; the charts above
          keep their own order. <a href={compareHref(scope)}>Put two languages head to head →</a>
        </p>
        <ResultsTable rows={visible} sort={state.sort} onSort={onSort} />
      </section>

      <Failures failures={failures} total={analysis.failures.length} />

      {/* What the columns mean, before what the rows are: a reader who has just met
          this table needs `Startup` explained before they need to know which JVM ran
          the Kotlin. Rendered from `docs/columns.md` by Astro, at build time, and
          slotted in here — the browser never sees a Markdown renderer. */}
      <section className="card reference">{columns}</section>

      <section className="card">
        <h2>The implementations</h2>
        <p>
          One card per row of the table: what it is, and what the person who wrote it wanted you to
          know. Three fields identify an implementation — the <strong>language</strong> the program
          is written in, the <strong>compiler</strong> that turned it into machine code, and the{" "}
          <strong>interpreter</strong> that executed it. Most implementations have only one of the
          last two, and <code>n/a</code> is an answer rather than a gap: a compiled binary has no
          interpreter, and an interpreted language has nothing compiled ahead of the run.
        </p>
        <div className="impls">
          {analysis.backends
            .filter((entry) => entry.workload === workload?.workload)
            .map((entry) => (
              // The anchor a table row links to. Its `id` is the triple, like every
              // other thing on this site a reader can point at.
              <article className="impl" id={anchorId(entry)} key={entry.id}>
                <header className="impl-head">
                  <h3>{entry.language}</h3>
                  <span className="impl-chain">{toolchain(entry)}</span>
                </header>

                <dl className="impl-triple">
                  <div className="impl-field">
                    <dt>compiler</dt>
                    <dd>{optional(entry.compiler)}</dd>
                  </div>
                  <div className="impl-field">
                    <dt>interpreter</dt>
                    <dd>{optional(entry.interpreter)}</dd>
                  </div>
                </dl>

                <p className="impl-desc">{entry.description}</p>

                {/* The manifest's `comments`: a caveat, a pinned version, a warning about
                    what this row does *not* say. It reads as a footnote, so it looks like
                    one — never as a second paragraph of the description. */}
                {entry.comments !== null && <p className="impl-note">{entry.comments}</p>}
              </article>
            ))}
        </div>
      </section>

      <section className="card">
        <h2>The machine, and the campaign</h2>
        <dl className="machine">
          {analysis.machine_fields.map((field) => (
            <div key={field.label} style={{ display: "contents" }}>
              <dt>{field.label}</dt>
              <dd>{field.value}</dd>
            </div>
          ))}
          {/* How the work was sized, as the workload declared it — whatever its knobs
              happen to be called. A grid and an iteration ceiling are Mandelbrot's
              business; the site knows only that a workload has params. */}
          {campaign.workload.params.map((param) => (
            <div key={param.name} style={{ display: "contents" }}>
              <dt>{param.name}</dt>
              <dd>{String(param.value)}</dd>
            </div>
          ))}
          <dt>-march</dt>
          <dd>{campaign.march === "" ? "none" : campaign.march}</dd>
          <dt>threads</dt>
          <dd>{campaign.cpu}</dd>
          <dt>rounds</dt>
          <dd>
            {campaign.rounds} run / {campaign.build_rounds} build / {campaign.warmup_rounds} warmup
          </dd>
          <dt>strict checksum</dt>
          <dd>{workload?.strict_checksum ?? "n/a"}</dd>
          <dt>langbench</dt>
          <dd>{campaign.langbench_version}</dd>
        </dl>
      </section>

      <p className="footnote">
        The samples this page is built from are{" "}
        <a
          href={`${import.meta.env.BASE_URL}data/${campaign.workload.id}/${analysis.architecture}.ndjson`}
        >
          samples/{campaign.workload.id}/{analysis.architecture}.ndjson
        </a>{" "}
        — this campaign's only artefact, published byte for byte. Everything above is recomputed
        from it, in Rust, in your browser.
      </p>
    </main>
  );
}

/**
 * The implementations that are *not* in the tables above.
 *
 * A benchmark that quietly drops what did not work flatters itself. A crashed
 * implementation has no row, and a missing row reads exactly like an implementation
 * nobody ever wrote — so the campaign says which ones it lost, and to what. Nothing
 * here affects the rows that did finish: each is an independent run of an
 * independent image.
 */
function Failures({ failures, total }: { failures: Failure[]; total: number }) {
  if (total === 0) {
    return null;
  }
  if (failures.length === 0) {
    return (
      <section className="card">
        <h2>What did not finish</h2>
        <p>
          This campaign lost {total === 1 ? "one implementation" : `${total} implementations`}, but
          none of them match the filters above. Clear the filters to see them.
        </p>
      </section>
    );
  }
  return (
    <section className="card">
      <h2>What did not finish</h2>
      <p>
        {failures.length === 1
          ? "One scheduled implementation is"
          : `${failures.length} scheduled implementations are`}{" "}
        absent from the charts and the table above. {failures.length === 1 ? "It was" : "Each was"}{" "}
        quarantined at the point it broke — a build that failed, a run that crashed or hung, or a
        checksum that disagreed with the <code>strict</code> reference. A wrong run never enters the
        statistics, so{" "}
        {failures.length === 1 ? "it contributed no timing" : "none of them contributed a timing"}{" "}
        to anything.
      </p>
      <div className="table-scroll">
        <table className="failures">
          <thead>
            <tr>
              <th className="text">Language</th>
              <th className="text">Compiler</th>
              <th className="text">Interpreter</th>
              <th className="text">Mode</th>
              <th className="text">Where</th>
              <th className="text">What happened</th>
            </tr>
          </thead>
          <tbody>
            {failures.map((failure) => (
              <tr key={`${failure.backend_id}-${failure.mode}-${failure.stage}`}>
                <td className="text">{failure.language}</td>
                <td className="text muted-cell">{optional(failure.compiler)}</td>
                <td className="text muted-cell">{optional(failure.interpreter)}</td>
                <td className="text">{failure.mode}</td>
                <td className="text">{where(failure)}</td>
                <td className="failure-error">{failure.error}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

/**
 * Where an implementation was when it went. `build` means the image never compiled,
 * so there was nothing to run and no round to fail in; anything else names the phase
 * and the round — counted from one, as the campaign log counts them.
 */
function where(failure: Failure): string {
  if (failure.stage === "prepare") {
    return "build";
  }
  if (failure.phase === null || failure.round === null) {
    return "run";
  }
  return `${failure.phase} round ${failure.round + 1}`;
}

function Tiles({ rows, analysis }: { rows: Aggregate[]; analysis: Analysis }) {
  const measured = rows.filter((row) => row.run_wall !== null);
  const fastest = measured.reduce<Aggregate | null>(
    (best, row) =>
      best === null || (row.run_wall?.min ?? 0) < (best.run_wall?.min ?? 0) ? row : best,
    null,
  );
  const slowest = measured.reduce<Aggregate | null>(
    (worst, row) =>
      worst === null || (row.run_wall?.min ?? 0) > (worst.run_wall?.min ?? 0) ? row : worst,
    null,
  );
  // The campaign's own verdict on itself: the worst dispersion any row carries.
  const noisiest = measured
    .filter((row) => (row.run_wall?.n ?? 0) >= 3)
    .reduce<Aggregate | null>(
      (worst, row) =>
        worst === null || (row.run_wall?.mad_pct ?? 0) > (worst.run_wall?.mad_pct ?? 0)
          ? row
          : worst,
      null,
    );

  return (
    <div className="tiles">
      <div className="tile">
        <div className="tile-label">Fastest</div>
        <div className="tile-value">{milliseconds(fastest?.run_wall?.min ?? null)}</div>
        <div className="tile-note">
          {fastest === null ? "nothing measured" : labelWithMode(fastest)}
        </div>
      </div>
      <div className="tile">
        <div className="tile-label">Spread</div>
        <div className="tile-value">
          {fastest !== null && slowest !== null && fastest.run_wall !== null
            ? ratio(slowest.run_wall?.min ?? 0, fastest.run_wall.min)
            : "n/a"}
        </div>
        <div className="tile-note">fastest to slowest, on screen</div>
      </div>
      <div className="tile">
        <div className="tile-label">Worst dispersion</div>
        <div className="tile-value">{dispersion(noisiest?.run_wall ?? null)}</div>
        <div className="tile-note">
          {(noisiest?.run_wall?.mad_pct ?? 0) > 2
            ? "above 2%: this campaign cannot defend a percentage-level claim"
            : "below 2%: the campaign holds"}
        </div>
      </div>
      <div className="tile">
        <div className="tile-label">Implementations</div>
        <div className="tile-value">{analysis.backends.length}</div>
        <div className="tile-note">
          {rows.length} row{rows.length === 1 ? "" : "s"} on screen
        </div>
      </div>
    </div>
  );
}
