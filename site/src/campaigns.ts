// One fetch, one WebAssembly instance, for the life of the tab.
//
// The pages are prerendered and the router swaps them client-side, which means the
// module registry outlives a navigation: this cache is what turns three static
// pages into an app that does not re-download a 500 kB campaign every time somebody
// clicks "Compare".
//
// Keyed by the options, because they are not a filter over a result — `include_warmup`
// is a different *aggregation* of the same file, and the harness has to do it again.
// The raw NDJSON is kept beside each analysis: it is the input of every other
// question the harness can be asked, and `compare()` is the second one.

import { useEffect, useState } from "react";
import { fetchCampaigns, type LoadedCampaign, type Options } from "./analysis";
import { logger } from "./logger";

/** Where the campaigns this build publishes are served from. See `scripts/data.js`. */
const DATA_URL = `${import.meta.env.BASE_URL}data/`;

const cache = new Map<string, Promise<LoadedCampaign[]>>();

function load(options: Options): Promise<LoadedCampaign[]> {
  const key = String(options.include_warmup);
  const hit = cache.get(key);
  if (hit !== undefined) {
    return hit;
  }
  const pending = fetchCampaigns(DATA_URL, options).catch((cause: unknown) => {
    // A failure is not cached: a reader who lost the network for one second should
    // get the campaign on the next navigation, not the error for ever.
    cache.delete(key);
    throw cause;
  });
  cache.set(key, pending);
  return pending;
}

export interface CampaignsState {
  campaigns: LoadedCampaign[] | null;
  error: string | null;
}

/**
 * Every campaign this build publishes, one per ISA — analyzed once, shared by every
 * page.
 *
 * It takes the flag, not an `Options` object: an object literal is a new object on
 * every render, and an effect that watched one would re-run for ever. The single
 * knob that changes what the harness computes is a boolean, so a boolean is what
 * this hook depends on.
 */
export function useCampaigns(includeWarmup: boolean): CampaignsState {
  const [state, setState] = useState<CampaignsState>({
    campaigns: null,
    error: null,
  });

  useEffect(() => {
    let live = true;
    setState({ campaigns: null, error: null });
    load({ include_warmup: includeWarmup })
      .then((campaigns) => {
        if (live) {
          setState({ campaigns, error: null });
        }
      })
      .catch((cause: unknown) => {
        const message = cause instanceof Error ? cause.message : String(cause);
        logger.error("campaign.failed", { message });
        if (live) {
          setState({ campaigns: null, error: message });
        }
      });
    return () => {
      live = false;
    };
  }, [includeWarmup]);

  return state;
}
