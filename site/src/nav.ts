// The sidebar, derived from what exists rather than typed out by hand.
//
// A link that has to be maintained is a link that will be wrong: publish a campaign on
// a second architecture, declare a second workload, and the nav has to know without
// anybody remembering. So it is built from the same two lists the routes are built
// from — which is also the guarantee that every entry points at a page that exists.
//
// The three fixed pages — what this is, the measurements, the methodology — are the ones
// that do not come from data. There is one of each, and there is no third: the methodology
// was eleven pages and is now one, because a reader disputing a number should not have to
// pick which of eleven documents holds the argument.

import { campaignsOf, workloads } from "./site";

export interface NavLink {
  href: string;
  label: string;
  children?: NavLink[];
}

export interface NavSection {
  title: string;
  links: NavLink[];
}

/**
 * Everything the sidebar shows, for a given `base` — which is `/langbench/` on Pages
 * and `/` under `astro dev`, so it is passed in rather than assumed.
 *
 * A campaign hangs under its workload. That is not a layout preference: a campaign is
 * one machine measuring one workload, an absolute timing never crosses an
 * architecture, and a nav that listed `x86_64` at the top level would be inviting the
 * one comparison this project refuses to publish.
 */
export function sections(base: string): NavSection[] {
  const work: NavLink[] = workloads.map((workload) => ({
    href: `${base}workloads/${workload.id}/`,
    label: workload.id,
    children: campaignsOf(workload.id).map((campaign) => ({
      href: `${base}workloads/${campaign.workload}/${campaign.architecture}/`,
      // The architecture names the campaign, because on one workload it is what
      // separates two of them — and it is the thing you may not compare across.
      label: campaign.architecture,
    })),
  }));

  return [
    {
      title: "langbench",
      links: [
        { href: base, label: "What this is" },
        // What every number a campaign records means. The measurements are the same on
        // every campaign — they *are* what this project measures — so they belong to
        // langbench itself rather than to any one campaign, and every results table links
        // here. Not "Data": the data is `samples.ndjson`, which this page contains none of.
        { href: `${base}measurements/`, label: "Measurements" },
        // How those numbers are produced, and what may be concluded from them.
        { href: `${base}methodology/`, label: "Methodology" },
      ],
    },
    {
      // Empty only in a repository with no `workload.yaml` at all, which the build
      // refuses to produce a site for.
      title: "Workloads",
      links: work,
    },
    {
      title: "Tools",
      links: [{ href: `${base}compare/`, label: "Compare" }],
    },
  ];
}
