import { describe, expect, test } from "vitest";
import { measuredBatch, measuredChange, plannedSettings } from "./ledger";
import type { MediaFile } from "../types";

const source: MediaFile = {
  id: "a",
  name: "a.png",
  extension: "png",
  kind: "image",
  status: "ready",
  phase: "Waiting",
  sizeBytes: 1000,
  warnings: [],
};
describe("conversion ledger evidence", () => {
  test("reports both larger outputs and reductions with signed measurements", () => {
    expect(measuredChange(1000, 1250)).toMatchObject({ percent: "+25.0%", tone: "larger" });
    expect(measuredChange(1000, 250)).toMatchObject({ percent: "−75.0%", tone: "smaller" });
    expect(measuredChange(0, 50).percent).toBe("—");
    expect(measuredChange(1000, 1000).percent).toBe("0.0%");
  });
  test("compares only the same successful measured subset in a partial batch", () => {
    const files: MediaFile[] = [
      {
        ...source,
        status: "succeeded",
        output: { name: "a.jpg", format: "jpeg", sizeBytes: 400 },
      },
      { ...source, id: "failed", sizeBytes: 50000, status: "failed" },
      { ...source, id: "canceled", sizeBytes: 80000, status: "canceled" },
      { ...source, id: "waiting", sizeBytes: 90000 },
      { ...source, id: "unmeasured", status: "succeeded" },
    ];
    expect(measuredBatch(files)).toMatchObject({
      count: 1,
      input: 1000,
      output: 400,
      change: { percent: "−60.0%" },
    });
  });
  test("does not invent settings for a source that could not be inspected", () => {
    expect(plannedSettings(source)).toBe("Choose a supported source");
  });
});
