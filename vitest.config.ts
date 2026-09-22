import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Unit-test the pure TS libraries (format, abbreviation, helpers): node env is enough.
    environment: "node",
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    globals: false,
  },
});
