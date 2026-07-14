// The warm-up rounds, and the one control on this site that is not a filter.
//
// A filter changes *which* rows you are looking at. This changes *what the numbers
// are*: it asks the harness to aggregate the campaign again, folding in rounds it
// deliberately left out. That is a different kind of act, and it was being offered
// as a bare checkbox in a row of filters, labelled "aggregate the warmup rounds" —
// which tells you what the code does, not what you would learn.
//
// The rounds themselves are never deleted. They are in `samples.ndjson`, flagged as
// warmup, and that is the point: an exclusion you cannot inspect is an exclusion you
// have to take on trust. This toggle is how you inspect it.

interface Props {
  /** How many rounds of every implementation the campaign flagged as warm-up. */
  rounds: number;
  includeWarmup: boolean;
  onChange: (includeWarmup: boolean) => void;
  /** The compare page has less room and has already said most of this. */
  compact?: boolean;
}

/** The section of the methodology that argues for the exclusion this box undoes. */
const METHODOLOGY = `${import.meta.env.BASE_URL}methodology/#warm-up-rounds`;

export function Warmup({ rounds, includeWarmup, onChange, compact = false }: Props) {
  // A campaign run with `--warmup-rounds 0` has nothing to fold in, and a toggle
  // that changes nothing is a toggle that reads as broken.
  if (rounds === 0) {
    return null;
  }

  const first = rounds === 1 ? "The first round" : `The first ${rounds} rounds`;

  return (
    <div className={compact ? "warmup warmup-compact" : "warmup"}>
      <label className="toggle">
        <input
          type="checkbox"
          checked={includeWarmup}
          onChange={(event) => onChange(event.target.checked)}
        />
        <span>
          Include the <strong>warm-up</strong> {rounds === 1 ? "round" : "rounds"}
        </span>
      </label>

      {compact ? (
        // The compare page has said most of this already, but never the *why*: a control
        // that changes what the numbers are owes the reader the argument, wherever it
        // appears.
        <p className="warmup-note">
          <a href={METHODOLOGY}>Why they are left out →</a>
        </p>
      ) : (
        <p className="warmup-note">
          {first} of every implementation {rounds === 1 ? "is" : "are"} run and recorded like any
          other, then left out of the numbers. A program's first run is its worst one — a JIT has
          not compiled the hot loop yet, a filesystem cache is cold, a JVM is still loading classes
          — and it says more about the machine getting going than about the backend. Nothing is
          deleted: the {rounds === 1 ? "round is" : "rounds are"} in <code>samples.ndjson</code>,
          flagged, and this box is how you look at {rounds === 1 ? "it" : "them"} — an exclusion you
          cannot inspect is one you have to take on trust.{" "}
          <a href={METHODOLOGY}>The methodology makes the case in full →</a>
        </p>
      )}
    </div>
  );
}

/**
 * What the reader is looking at once they tick it — said out loud, next to the
 * numbers.
 *
 * The published figures are the ones with the warm-up rounds left out. A reader who
 * turned them on and then quoted a number would be quoting something this project
 * does not stand behind, and nothing on screen would have told them so.
 */
export function WarmupBanner({ rounds }: { rounds: number }) {
  return (
    <p className="isa-note warmup-banner">
      <strong>These numbers include the warm-up {rounds === 1 ? "round" : "rounds"}.</strong> They
      are not the campaign's published figures — untick the box to get those back.
      <br />
      Expect little to move, and know why. <strong>Run min</strong> cannot go up: a minimum taken
      over more samples can only fall or stay, and the warm-up round is the slow one, so it stays.{" "}
      <strong>Dispersion</strong> is a <em>median</em> absolute deviation, built to ignore a single
      outlier — which is exactly what a warm-up round is — so it too moves by very little. The one
      column that always changes is <strong>Runs</strong>.
      <br />
      That is the point rather than a disappointment: it is the demonstration that the exclusion is
      not hiding anything. If a row <em>does</em> lurch when you tick this, that row's first run was
      expensive, and it is worth reading its samples.{" "}
      <a href={METHODOLOGY}>Warm-up rounds, in the methodology →</a>
    </p>
  );
}
