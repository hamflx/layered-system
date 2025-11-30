import { WimImageInfo } from "../types";
import { Badge } from "./ui/Badge";
import { Button } from "./ui/Button";
import { Card } from "./ui/Card";
import { Input } from "./ui/Input";

type Props = {
  rootPath: string;
  setRootPath: (v: string) => void;
  wimPath: string;
  setWimPath: (v: string) => void;
  wimIndex: number;
  setWimIndex: (v: number) => void;
  baseSize: number;
  setBaseSize: (v: number) => void;
  baseName: string;
  setBaseName: (v: string) => void;
  baseDesc: string;
  setBaseDesc: (v: string) => void;
  wimImages: WimImageInfo[];
  onListWim: () => Promise<void>;
  onOpenExisting: () => Promise<void>;
  onCreateWorkspace: () => Promise<void>;
  status: "idle" | "initialized" | "error";
  message: string;
  admin: boolean | null;
  adminLabel: string;
  t: (key: string, options?: any) => string;
};

export function WorkspaceGate(props: Props) {
  const {
    rootPath,
    setRootPath,
    wimPath,
    setWimPath,
    wimIndex,
    setWimIndex,
    baseSize,
    setBaseSize,
    baseName,
    setBaseName,
    baseDesc,
    setBaseDesc,
    wimImages,
    onListWim,
    onOpenExisting,
    onCreateWorkspace,
    status,
    message,
    admin,
    adminLabel,
    t,
  } = props;
  const statusTone = status === "initialized" ? "positive" : status === "error" ? "danger" : "neutral";
  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <Card>
        <div className="flex items-center justify-between gap-3">
          <span className="text-sm font-semibold text-ink-700">{t("admin-status", { status: "" })}</span>
          <Badge tone={admin ? "positive" : "warn"} className="px-3 py-1">
            {adminLabel}
          </Badge>
        </div>
        <div className="mt-4 space-y-3">
          <Input
            value={rootPath}
            onChange={(e) => setRootPath(e.target.value)}
            placeholder={t("root-placeholder")}
            spellCheck={false}
          />
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <Button className="w-full py-3" onClick={onOpenExisting}>
              {t("init-root")}
            </Button>
          </div>
        </div>
        <Badge tone={statusTone} className="mt-3 w-full justify-start px-4 py-3 text-sm font-semibold">
          {message}
        </Badge>
      </Card>

      <Card>
        <div className="flex flex-col gap-1">
          <h3 className="text-xl font-semibold text-ink-900">{t("section-base-title")}</h3>
          <p className="text-sm text-ink-700">{t("status-uninitialized")}</p>
        </div>
        <div className="mt-4 space-y-3">
          <Input
            value={wimPath}
            onChange={(e) => setWimPath(e.target.value)}
            placeholder={t("wim-path-placeholder")}
            spellCheck={false}
          />
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <label className="flex flex-col gap-1 text-sm font-semibold text-ink-700">
              {t("wim-index-label")}
              <Input
                type="number"
                min={1}
                value={wimIndex}
                onChange={(e) => setWimIndex(Number(e.target.value))}
              />
            </label>
            <label className="flex flex-col gap-1 text-sm font-semibold text-ink-700">
              {t("base-size-label")}
              <Input
                type="number"
                min={20}
                value={baseSize}
                onChange={(e) => setBaseSize(Number(e.target.value))}
              />
            </label>
          </div>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <Input
              value={baseName}
              onChange={(e) => setBaseName(e.target.value)}
              placeholder={t("base-name-placeholder")}
            />
            <Input
              value={baseDesc}
              onChange={(e) => setBaseDesc(e.target.value)}
              placeholder={t("base-desc-placeholder")}
            />
          </div>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <Button variant="secondary" className="w-full py-3" onClick={onListWim}>
              {t("list-wim-button")}
            </Button>
            <Button className="w-full py-3" onClick={onCreateWorkspace}>
              {t("create-base-button")}
            </Button>
          </div>
          {wimImages.length > 0 && (
            <div className="grid gap-2 rounded-xl border border-peach-200/70 bg-peach-50/50 p-3">
              {wimImages.map((img) => (
                <div
                  key={img.index}
                  className="rounded-lg border border-white/70 bg-white/80 px-3 py-2 text-sm text-ink-900 shadow-sm shadow-peach-300/20"
                >
                  <strong>{img.index}</strong> {img.name} {img.description ? `- ${img.description}` : ""}{" "}
                  {img.size ? `(${img.size})` : ""}
                </div>
              ))}
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}
