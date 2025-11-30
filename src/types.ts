export type Settings = {
  root_path: string;
  locale: string;
  seq_counter: number;
  last_boot_guid?: string | null;
};

export type NodeStatus =
  | "normal"
  | "missing_file"
  | "missing_parent"
  | "missing_bcd"
  | "mounted"
  | "error";

export type Node = {
  id: string;
  parent_id?: string | null;
  name: string;
  path: string;
  bcd_guid?: string | null;
  desc?: string | null;
  created_at: string;
  status: NodeStatus;
  boot_files_ready: boolean;
};

export type WimImageInfo = {
  index: number;
  name: string;
  description?: string;
  size?: string;
};

export type TreeNode = Node & { children: TreeNode[] };
export type StatusLabels = Record<NodeStatus, string>;
