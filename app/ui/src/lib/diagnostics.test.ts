// Every code the checks report has its own message here.
import { expect, test } from "vitest";
import checks from "../../../../crates/core/src/check.rs?raw";
import { describe } from "./diagnostics";

test("every check code is described", () => {
  const codes = [...checks.matchAll(/Severity::(?:Error|Warning),\s*"([a-z-]+)"/g)].map((m) => m[1]);
  expect(codes.length).toBeGreaterThan(10);
  const fallback = "ENGLISH";
  const undescribed = codes.filter(
    (code) => describe({ severity: "error", code, message: fallback, args: { problem: "has-value", label: "x", count: "2" }, location: null }) === fallback,
  );
  expect(undescribed).toEqual([]);
});
