// The tests run against Astro's own Vite config, not a second one written by hand.
//
// `getViteConfig` hands back the config the site is actually built with — the React
// integration, the `base`, the aliases. A parallel config here would be a second
// definition of how this site compiles, and it would drift on the first plugin
// somebody adds to `astro.config.mjs`.

/// <reference types="vitest/config" />
// The reference above is what teaches Vite's `UserConfig` about the `test` key:
// Vitest declares it by module augmentation, and `getViteConfig` takes Vite's type.

import { getViteConfig } from "astro/config";

export default getViteConfig({
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./vitest.setup.ts"],
    // The WebAssembly module is the harness itself; the unit tests cover the pure
    // TypeScript around it. What the WASM computes is tested in Rust, by
    // `cargo test`, against the same code the report uses.
    exclude: ["**/node_modules/**", "**/dist/**", "src/wasm/**"],
  },
});
