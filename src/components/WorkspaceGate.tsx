import { WimImageInfo } from "../types";

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
  return (
    <div className="workspace-gate">
      <section className="card">
        <div className="row">
          <span className="label">{t("admin-status", { status: "" })}</span>
          <span className={`pill ${admin ? "ok" : "warn"}`}>{adminLabel}</span>
        </div>
        <div className="form column">
          <input
            value={rootPath}
            onChange={(e) => setRootPath(e.target.value)}
            placeholder={t("root-placeholder")}
            spellCheck={false}
          />
          <div className="form split">
            <button onClick={onOpenExisting}>{t("init-root")}</button>
          </div>
        </div>
        <div className={`message ${status}`}>
          <span>{message}</span>
        </div>
      </section>

      <section className="card">
        <h3>{t("section-base-title")}</h3>
        <p className="muted">{t("status-uninitialized")}</p>
        <div className="form column">
          <input
            value={wimPath}
            onChange={(e) => setWimPath(e.target.value)}
            placeholder={t("wim-path-placeholder")}
            spellCheck={false}
          />
          <div className="form split">
            <label>
              {t("wim-index-label")}
              <input
                type="number"
                min={1}
                value={wimIndex}
                onChange={(e) => setWimIndex(Number(e.target.value))}
              />
            </label>
            <label>
              {t("base-size-label")}
              <input
                type="number"
                min={20}
                value={baseSize}
                onChange={(e) => setBaseSize(Number(e.target.value))}
              />
            </label>
          </div>
          <div className="form split">
            <input
              value={baseName}
              onChange={(e) => setBaseName(e.target.value)}
              placeholder={t("base-name-placeholder")}
            />
            <input
              value={baseDesc}
              onChange={(e) => setBaseDesc(e.target.value)}
              placeholder={t("base-desc-placeholder")}
            />
          </div>
          <div className="form split">
            <button onClick={onListWim}>{t("list-wim-button")}</button>
            <button onClick={onCreateWorkspace}>{t("create-base-button")}</button>
          </div>
          {wimImages.length > 0 && (
            <div className="wim-list">
              {wimImages.map((img) => (
                <div key={img.index} className="wim-item">
                  <strong>{img.index}</strong> {img.name} {img.description ? `- ${img.description}` : ""}{" "}
                  {img.size ? `(${img.size})` : ""}
                </div>
              ))}
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
