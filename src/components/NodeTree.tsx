import { TreeNode, StatusLabels } from "../types";

type Props = {
  data: TreeNode[];
  selectedId: string;
  onSelect: (id: string) => void;
  statusLabels: StatusLabels;
  t: (key: string, options?: any) => string;
};

export function NodeTree({ data, selectedId, onSelect, statusLabels, t }: Props) {
  const renderTree = (list: TreeNode[]) => {
    if (!list.length) return <div className="empty">{t("tree-empty")}</div>;
    return (
      <ul className="tree-list">
        {list.map((node) => (
          <li key={node.id}>
            <div
              className={`tree-node ${selectedId === node.id ? "selected" : ""}`}
              onClick={() => onSelect(node.id)}
            >
              <div className="tree-title">
                <span className="node-name">{node.name}</span>
                <span className={`pill tiny status-${node.status}`}>{statusLabels[node.status]}</span>
              </div>
              <div className="node-meta">
                <span className="mono">{node.id}</span>
                <span className={`chip ${node.boot_files_ready ? "ok" : "warn"}`}>
                  {node.boot_files_ready ? t("boot-ready-short") : t("boot-not-ready-short")}
                </span>
              </div>
            </div>
            {node.children.length > 0 && renderTree(node.children)}
          </li>
        ))}
      </ul>
    );
  };

  return <div className="tree-pane">{renderTree(data)}</div>;
}
