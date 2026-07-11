// The one module allowed to touch `console`. Everything else logs through here.
//
// Structured key-values, never an interpolated message: `logger.debug("wasm.ready",
// { bytes })` is filtered by field, `console.log(\`wasm ready in ${ms}ms\`)` is
// filtered by grep. Verbosity comes from the environment, never from a call site.

export type Level = "debug" | "info" | "warn" | "error";

const ORDER: Record<Level, number> = { debug: 0, info: 1, warn: 2, error: 3 };

function threshold(): Level {
  const configured = import.meta.env.VITE_LOG_LEVEL;
  if (configured === "debug" || configured === "info" || configured === "warn") {
    return configured;
  }
  if (configured === "error") {
    return "error";
  }
  return import.meta.env.DEV ? "debug" : "info";
}

const MINIMUM = ORDER[threshold()];

type Fields = Record<string, unknown>;

function emit(level: Level, event: string, fields: Fields = {}): void {
  if (ORDER[level] < MINIMUM) {
    return;
  }
  const line = { level, event, ...fields };
  // The single sanctioned `console` call in the app. `console.error` for
  // anything a user should be able to hand back to us; the rest on `debug`, so
  // a browser's default view stays quiet.
  if (level === "error" || level === "warn") {
    console.error(line);
  } else {
    console.debug(line);
  }
}

export const logger = {
  debug: (event: string, fields?: Fields) => emit("debug", event, fields),
  info: (event: string, fields?: Fields) => emit("info", event, fields),
  warn: (event: string, fields?: Fields) => emit("warn", event, fields),
  error: (event: string, fields?: Fields) => emit("error", event, fields),
};
