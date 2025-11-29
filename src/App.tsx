import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import "./App.css";

type Settings = {
  root_path: string;
  locale: string;
  seq_counter: number;
  last_boot_guid?: string | null;
};

type NodeStatus = "normal" | "missing_file" | "missing_parent" | "missing_bcd" | "mounted" | "error";

type Node = {
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

type InitResponse = {
  settings: Settings;
};

type WimImageInfo = {
  index: number;
  name: string;
  description?: string;
  size?: string;
};

function App() {
  const { t, i18n } = useTranslation();
  const [rootPath, setRootPath] = useState("");
  const [admin, setAdmin] = useState<boolean | null>(null);
  const [message, setMessage] = useState("");
  const [status, setStatus] = useState<"idle" | "initialized" | "error">("idle");
  const [nodes, setNodes] = useState<Node[]>([]);
  const [baseName, setBaseName] = useState("base");
  const [baseSize, setBaseSize] = useState(60);
  const [baseDesc, setBaseDesc] = useState("");
  const [wimPath, setWimPath] = useState("");
  const [wimIndex, setWimIndex] = useState(1);
  const [wimImages, setWimImages] = useState<WimImageInfo[]>([]);
  const [diffParent, setDiffParent] = useState("");
  const [diffName, setDiffName] = useState("child");
  const [diffDesc, setDiffDesc] = useState("");
  const [selectedNode, setSelectedNode] = useState("");

  const adminLabel = useMemo(() => {
    if (admin === null) return "...";
    return admin ? t("admin-yes") : t("admin-no");
  }, [admin, t]);

  useEffect(() => {
    const bootstrap = async () => {
      try {
        const isAdmin = await invoke<boolean>("check_admin");
        setAdmin(isAdmin);
      } catch (err) {
        setAdmin(false);
      }

      try {
        const settings = await invoke<Settings | null>("get_settings");
        if (settings) {
          setRootPath(settings.root_path);
          setStatus("initialized");
          setMessage(t("status-initialized", { path: settings.root_path }));
          i18n.changeLanguage(settings.locale || "zh-CN");
          await refreshNodes();
        } else {
          setMessage(t("status-uninitialized"));
        }
      } catch (err) {
        setStatus("error");
        setMessage(t("status-error", { msg: String(err) }));
      }
    };
    bootstrap();
  }, [i18n, t]);

  useEffect(() => {
    if (status === "idle") {
      setMessage(t("status-uninitialized"));
    } else if (status === "initialized" && rootPath) {
      setMessage(t("status-initialized", { path: rootPath }));
    }
  }, [rootPath, status, t]);

  const refreshNodes = async () => {
    try {
      const list = await invoke<Node[]>("scan_workspace");
      setNodes(list);
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

  const handleInit = async () => {
    if (!rootPath.trim()) {
      setMessage(t("status-error", { msg: "Root path is empty" }));
      setStatus("error");
      return;
    }
    try {
      const result = await invoke<InitResponse>("init_root", {
        rootPath,
        locale: i18n.language,
      });
      setStatus("initialized");
      setMessage(t("status-initialized", { path: result.settings.root_path }));
      await refreshNodes();
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

  const handleLocaleChange = (lng: string) => {
    i18n.changeLanguage(lng);
  };

  const handleListWim = async () => {
    try {
      const res = await invoke<WimImageInfo[]>("list_wim_images", { imagePath: wimPath });
      setWimImages(res);
      setMessage(`WIM images loaded (${res.length})`);
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

  const handleCreateBase = async () => {
    try {
      const res = await invoke<{ node: Node }>("create_base_vhd", {
        name: baseName,
        desc: baseDesc || null,
        wimFile: wimPath,
        wimIndex,
        sizeGb: baseSize,
      });
      setMessage(`Base created: ${res.node.name}`);
      await refreshNodes();
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

  const handleCreateDiff = async () => {
    try {
      const res = await invoke<{ node: Node }>("create_diff_vhd", {
        parentId: diffParent,
        name: diffName,
        desc: diffDesc || null,
      });
      setMessage(`Diff created: ${res.node.name}`);
      await refreshNodes();
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

  const handleBootReboot = async () => {
    if (!selectedNode) return;
    try {
      await invoke("set_bootsequence_and_reboot", { nodeId: selectedNode });
      setMessage("Boot sequence set, rebooting...");
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

  const handleDelete = async () => {
    if (!selectedNode) return;
    try {
      await invoke("delete_subtree", { nodeId: selectedNode });
      setMessage("Deleted subtree.");
      await refreshNodes();
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

  const handleRepair = async () => {
    if (!selectedNode) return;
    try {
      const guid = await invoke<string | null>("repair_bcd", { nodeId: selectedNode });
      setMessage(`Repaired BCD: ${guid ?? "no guid"}`);
      await refreshNodes();
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

  return (
    <main className="app">
      <header className="header">
        <div>
          <p className="eyebrow">{t("subtitle")}</p>
          <h1>{t("title")}</h1>
        </div>
        <div className="locale-switcher">
          <label htmlFor="locale">{t("locale-label")}</label>
          <select
            id="locale"
            value={i18n.language}
            onChange={(e) => handleLocaleChange(e.target.value)}
          >
            <option value="zh-CN">{t("locale-zh")}</option>
            <option value="en">{t("locale-en")}</option>
          </select>
        </div>
      </header>

      <section className="card">
        <div className="row">
          <span className="label">{t("admin-status", { status: "" })}</span>
          <span className={`pill ${admin ? "ok" : "warn"}`}>{adminLabel}</span>
        </div>
        <div className="form">
          <input
            value={rootPath}
            onChange={(e) => setRootPath(e.target.value)}
            placeholder={t("root-placeholder")}
            spellCheck={false}
          />
          <button onClick={handleInit}>{t("init-root")}</button>
        </div>
        <div className={`message ${status}`}>
          <span>{message}</span>
        </div>
      </section>

      <section className="card">
        <h3>初始化基础盘</h3>
        <div className="form column">
          <input
            value={wimPath}
            onChange={(e) => setWimPath(e.target.value)}
            placeholder="WIM/ESD 路径"
            spellCheck={false}
          />
          <div className="form split">
            <label>
              Index
              <input
                type="number"
                min={1}
                value={wimIndex}
                onChange={(e) => setWimIndex(Number(e.target.value))}
              />
            </label>
            <label>
              Size(GB)
              <input
                type="number"
                min={20}
                value={baseSize}
                onChange={(e) => setBaseSize(Number(e.target.value))}
              />
            </label>
          </div>
          <div className="form split">
            <input value={baseName} onChange={(e) => setBaseName(e.target.value)} placeholder="名称" />
            <input value={baseDesc} onChange={(e) => setBaseDesc(e.target.value)} placeholder="描述（可选）" />
          </div>
          <div className="form split">
            <button onClick={handleListWim}>列出镜像</button>
            <button onClick={handleCreateBase}>创建基础盘</button>
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

      <section className="card">
        <h3>创建差分盘</h3>
        <div className="form column">
          <select value={diffParent} onChange={(e) => setDiffParent(e.target.value)}>
            <option value="">选择父节点</option>
            {nodes.map((n) => (
              <option key={n.id} value={n.id}>
                {n.name} ({n.id.slice(0, 6)})
              </option>
            ))}
          </select>
          <div className="form split">
            <input value={diffName} onChange={(e) => setDiffName(e.target.value)} placeholder="名称" />
            <input value={diffDesc} onChange={(e) => setDiffDesc(e.target.value)} placeholder="描述（可选）" />
          </div>
          <button onClick={handleCreateDiff}>创建差分</button>
        </div>
      </section>

      <section className="card">
        <h3>节点管理</h3>
        <div className="nodes">
          {nodes.map((n) => (
            <div
              key={n.id}
              className={`node ${selectedNode === n.id ? "selected" : ""}`}
              onClick={() => setSelectedNode(n.id)}
            >
              <div className="node-title">
                <strong>{n.name}</strong>
                <span className={`pill small ${n.status === "normal" ? "ok" : "warn"}`}>{n.status}</span>
              </div>
              <div className="node-sub">
                <span>{n.path}</span>
                <span>{n.bcd_guid ?? "no BCD"}</span>
              </div>
            </div>
          ))}
        </div>
        <div className="form split">
          <button onClick={refreshNodes}>刷新</button>
          <button onClick={handleBootReboot} disabled={!selectedNode}>
            设置下次启动并重启
          </button>
          <button onClick={handleRepair} disabled={!selectedNode}>
            修复 BCD
          </button>
          <button onClick={handleDelete} disabled={!selectedNode}>
            删除子树
          </button>
        </div>
      </section>
    </main>
  );
}

export default App;
