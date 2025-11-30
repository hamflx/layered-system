import { NodeStatus } from "../types";

export type BadgeTone = "positive" | "warn" | "danger" | "info" | "neutral" | "muted";

const statusToneMap: Record<NodeStatus, BadgeTone> = {
  normal: "positive",
  missing_file: "danger",
  missing_parent: "warn",
  missing_bcd: "warn",
  mounted: "info",
  error: "danger",
};

export function statusToneFor(status: NodeStatus): BadgeTone {
  return statusToneMap[status];
}
