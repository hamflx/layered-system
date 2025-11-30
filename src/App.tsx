import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { NodeDetail } from "./components/NodeDetail";
import { NodeTree } from "./components/NodeTree";
import { WorkspaceGate } from "./components/WorkspaceGate";
import { Node, Settings, StatusLabels, TreeNode, WimImageInfo } from "./types";
import { Badge } from "./components/ui/Badge";
import { Button } from "./components/ui/Button";
import { Card } from "./components/ui/Card";

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
      const list = await invoke<Node[]>("list_nodes");
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

  useEffect(() => {
    if (!workspaceReady) return;
    refreshNodes();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workspaceReady]);

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

  const handleCheck = async () => {
    try {
      const list = await invoke<Node[]>("scan_workspace");
      setNodes(list);
      setMessage(t("message-checked"));
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
    <div className="min-h-screen bg-gradient-to-br from-peach-50 via-peach-200/50 to-peach-400/40 font-sans text-ink-900">
      <main className="mx-auto flex min-h-screen max-w-6xl flex-col gap-4 px-4 py-6 sm:px-6 lg:px-8">
        <Card className="p-5 shadow-lg shadow-peach-300/30">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.28em] text-peach-400">{t("subtitle")}</p>
              <h1 className="text-3xl font-bold leading-tight sm:text-4xl">{t("title")}</h1>
            </div>
            <div className="flex items-center gap-3 rounded-full border border-peach-200/80 bg-peach-50/80 px-3 py-2 shadow-inner shadow-peach-400/25">
              <label
                htmlFor="locale"
                className="text-xs font-semibold uppercase tracking-wide text-ink-700"
              >
                {t("locale-label")}
              </label>
              <select
                id="locale"
                value={i18n.language}
                onChange={(e) => handleLocaleChange(e.target.value)}
                className="rounded-full border border-peach-200 bg-white/90 px-3 py-2 text-sm font-semibold text-ink-900 shadow-sm shadow-peach-200/50 focus:border-peach-300 focus:outline-none focus:ring-2 focus:ring-peach-300/60"
              >
                <option value="zh-CN">{t("locale-zh")}</option>
                <option value="en">{t("locale-en")}</option>
              </select>
            </div>
          </div>
        </Card>

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
          <section className="flex min-h-[60vh] flex-col gap-4">
            <Card className="flex flex-wrap items-center justify-between gap-3 p-4 shadow-md shadow-peach-300/25">
              <div className="flex flex-wrap items-center gap-3">
                <Badge tone={admin ? "positive" : "warn"} className="px-3 py-1">
                  {adminLabel}
                </Badge>
                <span className="truncate font-mono text-sm text-ink-700">{rootPath}</span>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Button variant="secondary" onClick={refreshNodes}>
                  {t("refresh-button")}
                </Button>
                <Button variant="secondary" onClick={handleCheck}>
                  {t("check-button")}
                </Button>
                <Badge
                  tone={status === "initialized" ? "positive" : status === "error" ? "danger" : "neutral"}
                  className="max-w-xs truncate px-3 py-2"
                >
                  {message}
                </Badge>
              </div>
            </Card>

            <div className="grid min-h-0 flex-1 grid-cols-1 gap-4 lg:grid-cols-[340px_minmax(0,1fr)]">
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
        )}
      </main>
    </div>
  );
}

export default App;
