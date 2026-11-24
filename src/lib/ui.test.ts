import { describe, expect, it } from "vitest";
import {
  compatibilityTone,
  dependencyTone,
  groupBy,
  humanizeGroup,
  isJobFinished,
  isJobRunning,
  jobTone,
  percentLabel,
  severityTone,
} from "./ui";

describe("tone mappers", () => {
  it("maps compatibility status", () => {
    expect(compatibilityTone("compatible")).toBe("ok");
    expect(compatibilityTone("degraded")).toBe("warn");
    expect(compatibilityTone("incompatible")).toBe("danger");
    expect(compatibilityTone("unknown")).toBe("neutral");
    expect(compatibilityTone(null)).toBe("neutral");
  });

  it("maps diagnostic severity", () => {
    expect(severityTone("ok")).toBe("ok");
    expect(severityTone("warning")).toBe("warn");
    expect(severityTone("error")).toBe("danger");
    expect(severityTone("skipped")).toBe("neutral");
  });

  it("maps dependency status", () => {
    expect(dependencyTone("installed")).toBe("ok");
    expect(dependencyTone("outdated")).toBe("warn");
    expect(dependencyTone("missing")).toBe("danger");
    expect(dependencyTone("invalid")).toBe("danger");
  });

  it("maps job status", () => {
    expect(jobTone("running")).toBe("accent");
    expect(jobTone("cancelling")).toBe("accent");
    expect(jobTone("succeeded")).toBe("ok");
    expect(jobTone("failed")).toBe("danger");
    expect(jobTone("cancelled")).toBe("warn");
    expect(jobTone("idle")).toBe("neutral");
    expect(jobTone(null)).toBe("neutral");
  });
});

describe("job predicates", () => {
  it("treats cancelling as running", () => {
    expect(isJobRunning("running")).toBe(true);
    expect(isJobRunning("cancelling")).toBe(true);
    expect(isJobRunning("succeeded")).toBe(false);
    expect(isJobRunning(undefined)).toBe(false);
  });

  it("recognises terminal states", () => {
    expect(isJobFinished("succeeded")).toBe(true);
    expect(isJobFinished("failed")).toBe(true);
    expect(isJobFinished("cancelled")).toBe(true);
    expect(isJobFinished("running")).toBe(false);
  });
});

describe("percentLabel", () => {
  it("rounds real percentages only", () => {
    expect(percentLabel(42.4)).toBe("42%");
    expect(percentLabel(42.6)).toBe("43%");
    expect(percentLabel(0)).toBe("0%");
  });

  it("returns null when upstream reports nothing", () => {
    expect(percentLabel(null)).toBeNull();
    expect(percentLabel(undefined)).toBeNull();
    expect(percentLabel(Number.NaN)).toBeNull();
  });
});

describe("groupBy", () => {
  it("keeps first-seen order and buckets every item", () => {
    const items = [
      { group: "System", id: "os" },
      { group: "Network", id: "net" },
      { group: "System", id: "cpu" },
    ];
    const grouped = groupBy(items, (item) => item.group);
    expect(grouped.map(([name]) => name)).toEqual(["System", "Network"]);
    expect(grouped[0][1].map((item) => item.id)).toEqual(["os", "cpu"]);
    expect(grouped[1][1].map((item) => item.id)).toEqual(["net"]);
  });

  it("returns an empty list for empty input", () => {
    expect(groupBy([], (value: string) => value)).toEqual([]);
  });
});

describe("humanizeGroup", () => {
  it("turns backend identifiers into readable labels", () => {
    expect(humanizeGroup("Game files")).toBe("Game files");
    expect(humanizeGroup("game_files")).toBe("Game files");
    expect(humanizeGroup("  windows-sdk ")).toBe("Windows sdk");
    expect(humanizeGroup("")).toBe("");
  });
});
