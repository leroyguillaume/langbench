// One horizontal bar chart, grouped or stacked. Every chart on the site is this.
//
// Horizontal, because the category labels are backend slugs — `python-cython-cpython`
// reads horizontally and would be unreadable rotated under a column. Plain HTML
// rather than SVG, because a flexbox track is responsive without measuring
// anything, and the marks specs (a 2px surface gap between fills, a 4px rounded
// data-end, a square baseline) are one CSS rule each.

import { useState } from "react";

/** A series is an identity, and it keeps its color whatever the filters leave standing. */
export interface Series {
  key: string;
  label: string;
  /** A CSS custom property name — `--series-1`. Assigned in fixed order, never cycled. */
  color: string;
}

export interface ChartRow {
  key: string;
  label: string;
  /** One entry per series, in the same order. `null` is "not measured", never zero. */
  values: (number | null)[];
}

interface Props {
  rows: ChartRow[];
  series: Series[];
  /** How a value is spelled — at the bar's tip, and in the tooltip. */
  format: (value: number) => string;
  /** Segments of one bar summing to a whole, rather than bars side by side. */
  stacked?: boolean;
  /** Spelled out beside the tip. Grouped charts label every bar; stacked ones label the total. */
  total?: (values: (number | null)[]) => string;
}

interface Hover {
  x: number;
  y: number;
  text: string;
}

export function BarChart({ rows, series, format, stacked = false, total }: Props) {
  const [hover, setHover] = useState<Hover | null>(null);

  // The scale is the whole chart's, not the row's: bars are compared *across*
  // rows, and a per-row scale would make every backend look equally fast.
  const scale = rows.reduce((max, row) => {
    const values = row.values.map((value) => value ?? 0);
    const extent = stacked ? values.reduce((sum, value) => sum + value, 0) : Math.max(...values, 0);
    return Math.max(max, extent);
  }, 0);

  if (rows.length === 0 || scale === 0) {
    return (
      <p className="bar-empty">Nothing to plot: no row of this campaign carries this number.</p>
    );
  }

  const show = (event: React.MouseEvent, text: string) => {
    setHover({ x: event.clientX, y: event.clientY, text });
  };

  return (
    <>
      <div className="chart">
        {rows.map((row) => (
          <div className="chart-row" key={row.key}>
            <div className="chart-label" title={row.label}>
              {row.label}
            </div>
            <div className="chart-track">
              {stacked ? (
                <div className="chart-bars">
                  <div className="chart-stack">
                    {row.values.map((value, index) => {
                      const spec = series[index];
                      if (spec === undefined || value === null || value <= 0) {
                        return null;
                      }
                      return (
                        <div
                          key={spec.key}
                          className="bar"
                          role="img"
                          style={{
                            width: `${(value / scale) * 100}%`,
                            background: `var(${spec.color})`,
                          }}
                          onMouseMove={(event) =>
                            show(event, `${row.label}\n${spec.label}: ${format(value)}`)
                          }
                          onMouseLeave={() => setHover(null)}
                          // The bar is the only thing carrying this value visually;
                          // a screen reader gets it as text or not at all.
                          aria-label={`${row.label}, ${spec.label}: ${format(value)}`}
                        />
                      );
                    })}
                  </div>
                  <span className="bar-value">{total?.(row.values) ?? ""}</span>
                </div>
              ) : (
                row.values.map((value, index) => {
                  const spec = series[index];
                  if (spec === undefined) {
                    return null;
                  }
                  // A mode this backend never declared gets no bar and no
                  // placeholder: three rows of "not measured" under every
                  // interpreted backend is noise, and the legend plus the colour
                  // of the bars that *are* there already say which mode is which.
                  if (value === null) {
                    return null;
                  }
                  return (
                    <div className="chart-bars" key={spec.key}>
                      <div
                        className="bar"
                        role="img"
                        style={{
                          width: `${(value / scale) * 100}%`,
                          background: `var(${spec.color})`,
                        }}
                        onMouseMove={(event) =>
                          show(event, `${row.label}\n${spec.label}: ${format(value)}`)
                        }
                        onMouseLeave={() => setHover(null)}
                        aria-label={`${row.label}, ${spec.label}: ${format(value)}`}
                      />
                      {/* Bars → value at the tip. Outside the bar, in a text token:
                          a light categorical hue is illegible as text. */}
                      <span className="bar-value">{format(value)}</span>
                    </div>
                  );
                })
              )}
            </div>
          </div>
        ))}
      </div>

      {/* A legend for two or more series, always. One series needs none — the
          title already names it. */}
      {series.length > 1 && (
        <div className="legend">
          {series.map((spec) => (
            <span className="legend-item" key={spec.key}>
              <span className="swatch" style={{ background: `var(${spec.color})` }} />
              {spec.label}
            </span>
          ))}
        </div>
      )}

      {hover !== null && (
        <div className="tooltip" style={{ left: hover.x, top: hover.y }} role="tooltip">
          {hover.text}
        </div>
      )}
    </>
  );
}
