import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { NodeDetail } from "./components/NodeDetail";
import { NodeTree } from "./components/NodeTree";
import { WorkspaceGate } from "./components/WorkspaceGate";
import { Node, Settings, StatusLabels, TreeNode, WimImageInfo } from "./types";
import "./App.css";

function App() {
  const { t, i18n } = useTranslation();
  const [rootPath, setRootPath] = useState("");
  const [admin, setAdmin] = useState<boolean | null>(null);
  const [message, setMessage] = useState("");
  const [status, setStatus] = useState<"idle" | "initialized" | "error">("idle");
  const [workspaceReady, setWorkspaceReady] = useState(false);
  const [nodes, setNodes] = useState<Node[]>([]);
  const [baseName, setBaseName] = useState("base");
  const [baseSize, setBaseSize] = useState(60);
  const [baseDesc, setBaseDesc] = useState("");
  const [wimPath, setWimPath] = useState("");
  const [wimIndex, setWimIndex] = useState(1);
  const [wimImages, setWimImages] = useState<WimImageInfo[]>([]);
  const [diffName, setDiffName] = useState("child");
  const [diffDesc, setDiffDesc] = useState("");
  const [selectedNode, setSelectedNode] = useState("");

  const statusLabels = useMemo<StatusLabels>(
    () => ({
      normal: t("node-status.normal"),
      missing_file: t("node-status.missing-file"),
      missing_parent: t("node-status.missing-parent"),
      missing_bcd: t("node-status.missing-bcd"),
      mounted: t("node-status.mounted"),
      error: t("node-status.error"),
    }),
    [t],
  );

  const adminLabel = useMemo(() => {
    if (admin === null) return "...";
    return admin ? t("admin-yes") : t("admin-no");
  }, [admin, t]);

  const refreshNodes = async () => {
    if (!workspaceReady) return;
    try {
      const list = await invoke<Node[]>("scan_workspace");
      setNodes(list);
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

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
          setWorkspaceReady(true);
          await refreshNodes();
        } else {
          setMessage(t("status-uninitialized"));
          setWorkspaceReady(false);
        }
      } catch (err) {
        setStatus("error");
        setMessage(t("status-error", { msg: String(err) }));
      }
    };
    bootstrap();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!workspaceReady || !nodes.length) {
      setSelectedNode("");
      return;
    }
    if (!selectedNode) {
      setSelectedNode(nodes[0].id);
    } else if (!nodes.some((n) => n.id === selectedNode)) {
      setSelectedNode(nodes[0].id);
    }
  }, [workspaceReady, nodes, selectedNode]);

  const treeData = useMemo<TreeNode[]>(() => {
    const map = new Map<string, TreeNode>();
    nodes.forEach((n) => map.set(n.id, { ...n, children: [] }));
    const roots: TreeNode[] = [];

    map.forEach((node) => {
      const parentId = node.parent_id || "";
      if (parentId && map.has(parentId)) {
        map.get(parentId)!.children.push(node);
      } else {
        roots.push(node);
      }
    });

    const sortRecursively = (list: TreeNode[]) => {
      list.sort((a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime());
      list.forEach((child) => sortRecursively(child.children));
    };
    sortRecursively(roots);
    return roots;
  }, [nodes]);

  const selectedDetail = useMemo(
    () => nodes.find((n) => n.id === selectedNode) || null,
    [nodes, selectedNode],
  );
  const parentNode = useMemo(
    () => nodes.find((n) => n.id === selectedDetail?.parent_id) || null,
    [nodes, selectedDetail],
  );

  useEffect(() => {
    if (selectedDetail) {
      setDiffName(`${selectedDetail.name}-child`);
      setDiffDesc("");
    }
  }, [selectedDetail?.id]);

  const handleLocaleChange = (lng: string) => {
    i18n.changeLanguage(lng);
  };

  const handleListWim = async () => {
    try {
      const res = await invoke<WimImageInfo[]>("list_wim_images", { imagePath: wimPath });
      setWimImages(res);
      setMessage(t("message-wim-loaded", { count: res.length }));
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

  const handleOpenExisting = async () => {
    if (!rootPath.trim()) {
      setMessage(t("status-error", { msg: t("error-empty-root") }));
      setStatus("error");
      return;
    }
    try {
      const result = await invoke<{ settings: Settings }>("init_root", {
        rootPath,
        locale: i18n.language,
      });
      setStatus("initialized");
      setWorkspaceReady(true);
      setMessage(t("status-initialized", { path: result.settings.root_path }));
      await refreshNodes();
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

  const handleCreateWorkspace = async () => {
    if (!rootPath.trim()) {
      setMessage(t("status-error", { msg: t("error-empty-root") }));
      setStatus("error");
      return;
    }
    try {
      await invoke<{ settings: Settings }>("init_root", {
        rootPath,
        locale: i18n.language,
      });
      const res = await invoke<{ node: Node }>("create_base_vhd", {
        name: baseName,
        desc: baseDesc || null,
        wimFile: wimPath,
        wimIndex,
        sizeGb: baseSize,
      });
      setStatus("initialized");
      setWorkspaceReady(true);
      setMessage(t("message-base-created", { name: res.node.name }));
      await refreshNodes();
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

  const handleCreateDiff = async () => {
    if (!selectedNode) return;
    try {
      const res = await invoke<{ node: Node }>("create_diff_vhd", {
        parentId: selectedNode,
        name: diffName,
        desc: diffDesc || null,
      });
      setMessage(t("message-diff-created", { name: res.node.name }));
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
      setMessage(t("message-boot-set"));
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

  const handleDelete = async () => {
    if (!selectedNode) return;
    try {
      await invoke("delete_subtree", { nodeId: selectedNode });
      setMessage(t("message-deleted"));
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
      setMessage(t("message-repaired-bcd", { guid: guid ?? t("message-no-guid") }));
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
          <select id="locale" value={i18n.language} onChange={(e) => handleLocaleChange(e.target.value)}>
            <option value="zh-CN">{t("locale-zh")}</option>
            <option value="en">{t("locale-en")}</option>
          </select>
        </div>
      </header>

      {!workspaceReady ? (
        <WorkspaceGate
          rootPath={rootPath}
          setRootPath={setRootPath}
          wimPath={wimPath}
          setWimPath={setWimPath}
          wimIndex={wimIndex}
          setWimIndex={setWimIndex}
          baseSize={baseSize}
          setBaseSize={setBaseSize}
          baseName={baseName}
          setBaseName={setBaseName}
          baseDesc={baseDesc}
          setBaseDesc={setBaseDesc}
          wimImages={wimImages}
          onListWim={handleListWim}
          onOpenExisting={handleOpenExisting}
          onCreateWorkspace={handleCreateWorkspace}
          status={status}
          message={message}
          admin={admin}
          adminLabel={adminLabel}
          t={t}
        />
      ) : (
        <>
          <section className="card">
            <div className="row">
              <span className="label">{t("admin-status", { status: "" })}</span>
              <span className={`pill ${admin ? "ok" : "warn"}`}>{adminLabel}</span>
              <span className="mono">{rootPath}</span>
              <button className="ghost-btn" onClick={refreshNodes}>
                {t("refresh-button")}
              </button>
            </div>
            <div className={`message ${status}`}>
              <span>{message}</span>
            </div>
          </section>

          <section className="card">
            <div className="node-panels">
              <NodeTree
                data={treeData}
                selectedId={selectedNode}
                onSelect={(id) => setSelectedNode(id)}
                statusLabels={statusLabels}
                t={t}
              />
              <NodeDetail
                selected={selectedDetail}
                parentNode={parentNode}
                statusLabels={statusLabels}
                diffName={diffName}
                diffDesc={diffDesc}
                setDiffName={setDiffName}
                setDiffDesc={setDiffDesc}
                onCreateDiff={handleCreateDiff}
                onBoot={handleBootReboot}
                onRepair={handleRepair}
                onDelete={handleDelete}
                t={t}
              />
            </div>
          </section>
        </>
      )}
    </main>
  );
}

export default App;
