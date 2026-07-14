// The sidebar, derived from what exists rather than typed out by hand.
//
// A link that has to be maintained is a link that will be wrong: publish a campaign on
// a second architecture, declare a second workload, and the nav has to know without
// anybody remembering. So it is built from the same two lists the routes are built
// from — which is also the guarantee that every entry points at a page that exists.
//
// The methodology is the exception in kind, not in principle: its pages are Markdown
// files, and they are globbed rather than listed, ordered by their frontmatter.

import { href, pages } from "./methodology";
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
        // What the columns mean. They are the same on every campaign — they *are* what
        // this project measures — so they belong to langbench itself rather than to any
        // one campaign, and every results table links here.
        { href: `${base}metrics/`, label: "Metrics" },
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
    {
      title: "Methodology",
      links: pages.map((page) => ({
        href: href(base, page),
        label: page.frontmatter.title,
      })),
    },
  ];
}
