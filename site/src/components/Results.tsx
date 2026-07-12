// One campaign, read out: the tiles, the charts, the table, and what it lost.
//
// The head-to-head used to be a card at the bottom of this page. It has a page of
// its own now, and this one links to it carrying the ISA and the algorithm — never
// the filters. A pair is not a table, and a reader who narrowed this table to one
// language has not thereby declined to compare it with another.

import { useEffect, useMemo, useState } from "react";
import type { Aggregate, Analysis, Failure, LoadedCampaign } from "../analysis";
import { useCampaigns } from "../campaigns";
import { bytes, dispersion, milliseconds, optional, ratio } from "../format";
import { label, labelWithMode } from "../identity";
import { modeSeries, SEQUENTIAL, WALL_SERIES } from "../series";
import { compareHref, type ResultsState, readResults, writeResults } from "../url";
import { BarChart, type ChartRow } from "./BarChart";
import { FilterBar } from "./FilterBar";
import { filterRows, ResultsTable, type Sort, type SortKey, sortRows } from "./ResultsTable";

export function Results() {
  // The URL is read once, on mount, and written on every change: the address bar
  // describes what is on screen, and a link to it puts somebody else in front of
  // the same claim.
  const [state, setState] = useState<ResultsState>(readResults);
  const { campaigns, error } = useCampaigns(state.includeWarmup);

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
  if (campaigns === null) {
    return <p className="status">Reading the campaigns…</p>;
  }

  // One ISA at a time, always. Two campaigns from two architectures are two
  // experiments: an x86-64 millisecond and an aarch64 millisecond are not the same
  // claim, and a chart that puts them in one bar group invites exactly the
  // comparison `METHODOLOGY.md#the-isa-rule` forbids. The reader picks an ISA; the
  // site never adds one to another.
  const loaded = campaigns.find((entry) => entry.analysis.arch === state.arch) ?? campaigns[0];
  if (loaded === undefined) {
    return (
      <main className="page">
        <div className="warnings">
          <h2>This build publishes no campaign</h2>
          <p>
            No <code>samples/&lt;arch&gt;.ndjson</code> was found when the site was built.
          </p>
        </div>
      </main>
    );
  }

  return <Report loaded={loaded} campaigns={campaigns} state={state} setState={setState} />;
}

interface ReportProps {
  loaded: LoadedCampaign;
  campaigns: LoadedCampaign[];
  state: ResultsState;
  setState: (state: ResultsState) => void;
}

function Report({ loaded, campaigns, state, setState }: ReportProps) {
  const { analysis } = loaded;
  const { campaign } = analysis;

  const algo = analysis.algos.find((entry) => entry.algo === state.algo) ?? analysis.algos[0];
  const aggregates = useMemo(() => algo?.aggregates ?? [], [algo]);

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
        analysis.failures.filter((failure) => failure.algo === algo?.algo),
        state.filters,
      ),
    [analysis.failures, algo, state.filters],
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

  const scope = {
    arch: analysis.arch,
    algo: algo?.algo ?? null,
    includeWarmup: state.includeWarmup,
  };

  return (
    <main className="page">
      <header className="masthead">
        <h1>langbench</h1>
        <p>
          Compiler and runtime backends, measured on <strong>{analysis.arch}</strong>
          {analysis.hostname !== null && ` (${analysis.hostname})`} on{" "}
          {new Date(campaign.timestamp).toLocaleDateString()}. Every number below is derived from
          the raw samples by the harness itself — the site computes no statistic of its own.
        </p>
      </header>

      {campaigns.length > 1 && (
        <p className="isa-note">
          This build publishes {campaigns.length} campaigns, one per ISA, and never shows them
          together: an <strong>absolute timing does not cross an ISA</strong>. A millisecond here
          and a millisecond on{" "}
          {campaigns.find((entry) => entry.analysis.arch !== analysis.arch)?.analysis.arch} are not
          the same claim. Compare implementations <em>within</em> one architecture — the ratio is
          what travels.
        </p>
      )}

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
        scope={scope}
        onScope={(next) =>
          setState({
            ...state,
            arch: next.arch,
            algo: next.algo,
            includeWarmup: next.includeWarmup,
          })
        }
        filters={state.filters}
        onFilters={(filters) => setState({ ...state, filters })}
        rows={aggregates}
        algos={analysis.algos.map((entry) => entry.algo)}
        arches={campaigns.map((entry) => entry.analysis.arch)}
        arch={analysis.arch}
      />

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
          Sort any column. The ratio is against the fastest row on screen — a within-campaign,
          within-ISA number. Absolute timings never cross an ISA.{" "}
          <a href={compareHref(scope)}>Put two languages head to head →</a>
        </p>
        <ResultsTable rows={visible} sort={state.sort} onSort={onSort} />
      </section>

      <Failures failures={failures} total={analysis.failures.length} />

      <section className="card">
        <h2>The implementations</h2>
        <div className="backends">
          {analysis.backends
            .filter((entry) => entry.algo === algo?.algo)
            .map((entry) => (
              <div className="backend" key={entry.id}>
                <h3>{label(entry)}</h3>
                <p>{entry.description}</p>
                {entry.comments !== null && <p>{entry.comments}</p>}
                <dl>
                  <dt>language</dt>
                  <dd>{entry.language}</dd>
                  <dt>compiler</dt>
                  <dd>{optional(entry.compiler)}</dd>
                  <dt>interpreter</dt>
                  <dd>{optional(entry.interpreter)}</dd>
                </dl>
              </div>
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
          <dt>grid</dt>
          <dd>
            {campaign.grid_size} × {campaign.grid_size}, max_iter {campaign.max_iter}
          </dd>
          <dt>-march</dt>
          <dd>{campaign.march === "" ? "none" : campaign.march}</dd>
          <dt>threads</dt>
          <dd>{campaign.cpu}</dd>
          <dt>rounds</dt>
          <dd>
            {campaign.rounds} run / {campaign.build_rounds} build / {campaign.warmup_rounds} warmup
          </dd>
          <dt>strict checksum</dt>
          <dd>{algo?.strict_checksum ?? "n/a"}</dd>
          <dt>langbench</dt>
          <dd>{campaign.langbench_version}</dd>
        </dl>
      </section>

      <p className="footnote">
        The samples this page is built from are{" "}
        <a href={`${import.meta.env.BASE_URL}data/${analysis.arch}.ndjson`}>
          samples/{analysis.arch}.ndjson
        </a>{" "}
        — this campaign's only artefact. Everything above is recomputed from it, in Rust, in your
        browser.
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
