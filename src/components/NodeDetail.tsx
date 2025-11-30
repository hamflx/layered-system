import { statusToneFor } from "../lib/tone";
import { Node, StatusLabels } from "../types";
import { Badge } from "./ui/Badge";
import { Button } from "./ui/Button";
import { Card } from "./ui/Card";
import { Input } from "./ui/Input";

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
  const sectionClass = "rounded-xl border border-white/70 bg-white/80 p-4 shadow-sm shadow-peach-300/25";

  if (!selected)
    return (
      <Card className="flex h-full items-center justify-center border-dashed border-peach-300/70 bg-white/60 p-6 text-center text-sm text-ink-700 shadow-inner shadow-peach-300/30">
        {t("detail-empty")}
      </Card>
    );
  return (
    <Card className="h-full p-4">
      <div className="space-y-4">
        <div className={sectionClass}>
          <div className="grid grid-cols-[120px_1fr] gap-x-4 gap-y-3 text-sm sm:grid-cols-[140px_1fr]">
            <span className="text-xs font-semibold uppercase tracking-wide text-ink-700">ID</span>
            <span className="font-mono text-ink-900">{selected.id}</span>
            <span className="text-xs font-semibold uppercase tracking-wide text-ink-700">
              {t("detail-parent")}
            </span>
            <span className="text-ink-900">
              {parentNode ? `${parentNode.name} (${parentNode.id})` : t("common-none")}
            </span>
            <span className="text-xs font-semibold uppercase tracking-wide text-ink-700">
              {t("detail-path")}
            </span>
            <span className="font-mono text-ink-900">{selected.path}</span>
            <span className="text-xs font-semibold uppercase tracking-wide text-ink-700">
              {t("detail-bcd")}
            </span>
            <span className="font-mono text-ink-900">{selected.bcd_guid ?? t("common-missing")}</span>
            <span className="text-xs font-semibold uppercase tracking-wide text-ink-700">
              {t("detail-created-at")}
            </span>
            <span className="text-ink-900">{selected.created_at}</span>
            <span className="text-xs font-semibold uppercase tracking-wide text-ink-700">
              {t("detail-status")}
            </span>
            <span className="flex flex-wrap items-center gap-2">
              <Badge tone={statusToneFor(selected.status)} className="px-2 py-1 text-[11px]">
                {statusLabels[selected.status]}
              </Badge>
              <Badge
                tone={selected.boot_files_ready ? "positive" : "warn"}
                className="px-2 py-1 text-[11px]"
              >
                {selected.boot_files_ready ? t("boot-ready") : t("boot-not-ready")}
              </Badge>
            </span>
            <span className="text-xs font-semibold uppercase tracking-wide text-ink-700">
              {t("detail-desc")}
            </span>
            <span className="text-ink-900">{selected.desc || t("common-none")}</span>
          </div>
        </div>

        <div className={sectionClass}>
          <div className="flex flex-col gap-1">
            <h4 className="text-lg font-semibold text-ink-900">{t("section-diff-title")}</h4>
            <p className="text-sm text-ink-700">{t("diff-desc-placeholder")}</p>
          </div>
          <div className="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2">
            <Input
              value={diffName}
              onChange={(e) => setDiffName(e.target.value)}
              placeholder={t("diff-name-placeholder")}
            />
            <Input
              value={diffDesc}
              onChange={(e) => setDiffDesc(e.target.value)}
              placeholder={t("diff-desc-placeholder")}
            />
          </div>
          <div className="mt-3 flex justify-end">
            <Button onClick={onCreateDiff}>
              {t("create-diff-button")}
            </Button>
          </div>
        </div>

        <div className={sectionClass}>
          <h4 className="text-lg font-semibold text-ink-900">{t("node-actions")}</h4>
          <div className="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2">
            <Button variant="secondary" onClick={onBoot}>
              {t("set-boot-button")}
            </Button>
            <Button variant="secondary" onClick={onRepair}>
              {t("repair-bcd-button")}
            </Button>
            <Button variant="danger" onClick={onDelete}>
              {t("delete-subtree-button")}
            </Button>
          </div>
        </div>
      </div>
    </Card>
  );
}
