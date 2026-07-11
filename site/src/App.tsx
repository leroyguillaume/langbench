import { useEffect, useMemo, useState } from "react";
import type { Aggregate, Analysis, FpMode } from "./analysis";
import { fetchAnalysis } from "./analysis";
import { BarChart, type ChartRow } from "./components/BarChart";
import { ResultsTable, type Sort, type SortKey, sortRows } from "./components/ResultsTable";
import { bytes, dispersion, milliseconds, optional, ratio } from "./format";
import { logger } from "./logger";
import { MODES, modeSeries, SEQUENTIAL, WALL_SERIES } from "./series";
import { readUrl, type UrlState, writeUrl } from "./url";

/** The campaign this site publishes. The raw samples — see `scripts/data.js`. */
const CAMPAIGN_URL = `${import.meta.env.BASE_URL}data/samples.ndjson`;

export function App() {
  const [state, setState] = useState<UrlState>(readUrl);
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [error, setError] = useState<string | null>(null);

  // The whole analysis is recomputed by the WASM when `includeWarmup` changes:
  // it is a different aggregation of the same file, not a filter over a result.
  // Everything else is a view, and is done here.
  useEffect(() => {
    let live = true;
    setError(null);
    fetchAnalysis(CAMPAIGN_URL, { include_warmup: state.includeWarmup })
      .then((next) => {
        if (live) {
          setAnalysis(next);
        }
      })
      .catch((cause: unknown) => {
        const message = cause instanceof Error ? cause.message : String(cause);
        logger.error("campaign.failed", { message });
        if (live) {
          setError(message);
        }
      });
    return () => {
      live = false;
    };
  }, [state.includeWarmup]);

  useEffect(() => writeUrl(state), [state]);

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
  if (analysis === null) {
    return <p className="status">Reading the campaign…</p>;
  }

  return <Report analysis={analysis} state={state} setState={setState} />;
}

interface ReportProps {
  analysis: Analysis;
  state: UrlState;
  setState: (state: UrlState) => void;
}

function Report({ analysis, state, setState }: ReportProps) {
  const { campaign } = analysis;

  const algo = analysis.algos.find((entry) => entry.algo === state.algo) ?? analysis.algos[0];
  const languages = useMemo(
    () => [...new Set(algo?.aggregates.map((row) => row.language) ?? [])].sort(),
    [algo],
  );

  const visible = useMemo(() => {
    const rows = (algo?.aggregates ?? []).filter(
      (row) =>
        state.modes.includes(row.mode) &&
        (state.language === null || row.language === state.language) &&
        (state.search === "" || row.backend.includes(state.search.toLowerCase())),
    );
    return sortRows(rows, state.sort);
  }, [algo, state.modes, state.language, state.search, state.sort]);

  const onSort = (key: SortKey) => {
    // Second click on the same column reverses it; a new column starts ascending,
    // which for a timing means fastest-first — the order the report ranks in.
    const sort: Sort =
      state.sort.key === key
        ? { key, descending: !state.sort.descending }
        : { key, descending: false };
    setState({ ...state, sort });
  };

  const series = modeSeries(state.modes);

  // Grouped: one row per backend, one bar per mode. The bars a filter left
  // standing keep their own colour — the series is the mode, not the position.
  const runRows: ChartRow[] = useMemo(() => {
    const byBackend = new Map<string, ChartRow>();
    for (const row of visible) {
      const existing = byBackend.get(row.backend) ?? {
        key: row.backend,
        label: row.backend,
        values: state.modes.map(() => null),
      };
      const index = state.modes.indexOf(row.mode);
      if (index >= 0) {
        existing.values[index] = row.run_wall?.min ?? null;
      }
      byBackend.set(row.backend, existing);
    }
    return [...byBackend.values()];
  }, [visible, state.modes]);

  const wallRows: ChartRow[] = visible.map((row) => ({
    key: `${row.backend_id}-${row.mode}`,
    label: `${row.backend} · ${row.mode}`,
    values: [row.run_elapsed?.min ?? null, row.run_startup?.min ?? null],
  }));

  const buildRows: ChartRow[] = visible
    .filter((row) => row.build_elapsed !== null)
    .map((row) => ({
      key: `${row.backend_id}-${row.mode}`,
      label: `${row.backend} · ${row.mode}`,
      values: [row.build_elapsed?.min ?? null],
    }));

  const sizeRows: ChartRow[] = visible
    .filter((row) => row.binary_bytes !== null)
    .map((row) => ({
      key: `${row.backend_id}-${row.mode}`,
      label: `${row.backend} · ${row.mode}`,
      values: [row.binary_bytes],
    }));

  return (
    <main className="page">
      <header className="masthead">
        <h1>langbench</h1>
        <p>
          Compiler and runtime backends, measured on one machine, on{" "}
          {new Date(campaign.timestamp).toLocaleDateString()}. Every number below is derived from
          the raw samples by the harness itself — the site computes no statistic of its own.
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

      <Tiles rows={visible} analysis={analysis} />

      <Filters
        analysis={analysis}
        state={state}
        setState={setState}
        languages={languages}
        algoKeys={analysis.algos.map((entry) => entry.algo)}
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
        <p>What the image ships. Absent for a backend that compiles nothing ahead of the run.</p>
        <BarChart rows={sizeRows} series={SEQUENTIAL} format={bytes} />
      </section>

      <section className="card">
        <h2>Every number</h2>
        <p>
          Sort any column. The ratio is against the fastest row on screen — a within-campaign,
          within-ISA number. Absolute timings never cross an ISA.
        </p>
        <ResultsTable rows={visible} sort={state.sort} onSort={onSort} />
      </section>

      <section className="card">
        <h2>Backends</h2>
        <div className="backends">
          {analysis.backends
            .filter((backend) => backend.algo === algo?.algo)
            .map((backend) => (
              <div className="backend" key={backend.id}>
                <h3>{backend.backend}</h3>
                <p>{backend.description}</p>
                {backend.comments !== null && <p>{backend.comments}</p>}
                <dl>
                  <dt>language</dt>
                  <dd>{backend.language}</dd>
                  <dt>compiler</dt>
                  <dd>{optional(backend.compiler)}</dd>
                  <dt>interpreter</dt>
                  <dd>{optional(backend.interpreter)}</dd>
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

      <footer className="footer">
        Read METHODOLOGY.md before trusting any number here. The samples this page is built from are{" "}
        <a href={CAMPAIGN_URL}>samples.ndjson</a> — the campaign's only artefact.
      </footer>
    </main>
  );
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
          {fastest === null ? "nothing measured" : `${fastest.backend} · ${fastest.mode}`}
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
        <div className="tile-label">Backends</div>
        <div className="tile-value">{analysis.backends.length}</div>
        <div className="tile-note">{rows.length} rows on screen</div>
      </div>
    </div>
  );
}

interface FiltersProps {
  analysis: Analysis;
  state: UrlState;
  setState: (state: UrlState) => void;
  languages: string[];
  algoKeys: string[];
}

function Filters({ state, setState, languages, algoKeys }: FiltersProps) {
  const toggleMode = (mode: FpMode) => {
    const modes = state.modes.includes(mode)
      ? state.modes.filter((candidate) => candidate !== mode)
      : MODES.filter((candidate) => state.modes.includes(candidate) || candidate === mode);
    // Never leave the reader with an empty chart and no way back.
    setState({ ...state, modes: modes.length === 0 ? MODES : modes });
  };

  return (
    <div className="filters">
      <label className="filter">
        <span>Algorithm</span>
        <select
          value={state.algo ?? ""}
          onChange={(event) => setState({ ...state, algo: event.target.value })}
        >
          {algoKeys.map((key) => (
            <option key={key} value={key}>
              {key}
            </option>
          ))}
        </select>
      </label>

      <label className="filter">
        <span>Language</span>
        <select
          value={state.language ?? ""}
          onChange={(event) =>
            setState({ ...state, language: event.target.value === "" ? null : event.target.value })
          }
        >
          <option value="">every language</option>
          {languages.map((language) => (
            <option key={language} value={language}>
              {language}
            </option>
          ))}
        </select>
      </label>

      <label className="filter">
        <span>Backend</span>
        <input
          type="search"
          placeholder="gcc, cpython…"
          value={state.search}
          onChange={(event) => setState({ ...state, search: event.target.value })}
        />
      </label>

      <div className="filter">
        <span>FP mode</span>
        <div className="chart-bars">
          {MODES.map((mode) => (
            <label className="toggle" key={mode}>
              <input
                type="checkbox"
                checked={state.modes.includes(mode)}
                onChange={() => toggleMode(mode)}
              />
              {mode}
            </label>
          ))}
        </div>
      </div>

      <label className="toggle">
        <input
          type="checkbox"
          checked={state.includeWarmup}
          onChange={(event) => setState({ ...state, includeWarmup: event.target.checked })}
        />
        aggregate the warmup rounds
      </label>
    </div>
  );
}
