import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// The workspace vitest config does not enable globals, so testing-library
// cannot auto-cleanup between tests; unmount explicitly to keep queries
// unambiguous across tests in the same file.
afterEach(() => {
  cleanup();
});
