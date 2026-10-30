import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

/** Unit tests for the app's logic (dev only, frontend-tech-stack §2). Components come later. */
export default defineConfig({
  resolve: {
    alias: { "@": fileURLToPath(new URL(".", import.meta.url)) },
  },
  test: {
    include: ["lib/**/*.test.ts", "features/**/*.test.ts"],
    environment: "node",
  },
});
