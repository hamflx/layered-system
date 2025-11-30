import { Node, StatusLabels } from "../types";

type Props = {
  selected: Node | null;
  parentNode: Node | null;
  statusLabels: StatusLabels;
  diffName: string;
  diffDesc: string;
  setDiffName: (v: string) => void;
  setDiffDesc: (v: string) => void;
  onCreateDiff: () => void;
  onBoot: () => void;
  onRepair: () => void;
  onDelete: () => void;
  t: (key: string, options?: any) => string;
};

export function NodeDetail({
  selected,
  parentNode,
  statusLabels,
  diffName,
  diffDesc,
  setDiffName,
  setDiffDesc,
  onCreateDiff,
  onBoot,
  onRepair,
  onDelete,
  t,
}: Props) {
  if (!selected) return <div className="empty">{t("detail-empty")}</div>;
  return (
    <div className="detail-pane">
      <div className="pane-head">
        <span>{t("node-detail-title")}</span>
        <span className="muted">{selected.name}</span>
      </div>
      <div className="detail-grid">
        <span className="detail-label">ID</span>
        <span className="detail-value mono">{selected.id}</span>
        <span className="detail-label">{t("detail-parent")}</span>
        <span className="detail-value">
          {parentNode ? `${parentNode.name} (${parentNode.id})` : t("common-none")}
        </span>
        <span className="detail-label">{t("detail-path")}</span>
        <span className="detail-value mono">{selected.path}</span>
        <span className="detail-label">{t("detail-bcd")}</span>
        <span className="detail-value mono">{selected.bcd_guid ?? t("common-missing")}</span>
        <span className="detail-label">{t("detail-created-at")}</span>
        <span className="detail-value">{selected.created_at}</span>
        <span className="detail-label">{t("detail-status")}</span>
        <span className="detail-value status-line">
          <span className={`pill tiny status-${selected.status}`}>{statusLabels[selected.status]}</span>
          <span className={`chip ${selected.boot_files_ready ? "ok" : "warn"}`}>
            {selected.boot_files_ready ? t("boot-ready") : t("boot-not-ready")}
          </span>
        </span>
        <span className="detail-label">{t("detail-desc")}</span>
        <span className="detail-value">{selected.desc || t("common-none")}</span>
      </div>

      <div className="card inline-card">
        <h4>{t("section-diff-title")}</h4>
        <p className="muted">{t("diff-desc-placeholder")}</p>
        <div className="form split">
          <input
            value={diffName}
            onChange={(e) => setDiffName(e.target.value)}
            placeholder={t("diff-name-placeholder")}
          />
          <input
            value={diffDesc}
            onChange={(e) => setDiffDesc(e.target.value)}
            placeholder={t("diff-desc-placeholder")}
          />
        </div>
        <button className="primary-btn" onClick={onCreateDiff}>
          {t("create-diff-button")}
        </button>
      </div>

      <div className="form column tight">
        <div className="form split">
          <button onClick={onBoot}>{t("set-boot-button")}</button>
          <button onClick={onRepair}>{t("repair-bcd-button")}</button>
        </div>
        <div className="form split">
          <button className="danger" onClick={onDelete}>
            {t("delete-subtree-button")}
          </button>
        </div>
      </div>
    </div>
  );
}
