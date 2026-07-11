import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// GitHub Pages serves a project site under `/<repo>/`, never at the root. The
// base is a build-time input rather than a hardcoded string, so the same source
// builds for Pages, for a custom domain, and for `vite dev` at `/`.
const base = process.env.BASE_PATH ?? "/";

export default defineConfig({
  base,
  plugins: [react()],
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./vitest.setup.ts"],
    // The WebAssembly module is the harness itself; the unit tests cover the
    // pure TypeScript around it. What the WASM computes is tested in Rust, by
    // `cargo test`, against the same code the report uses.
    exclude: ["**/node_modules/**", "src/wasm/**"],
  },
});
