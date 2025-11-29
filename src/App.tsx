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

type InitResponse = {
  settings: Settings;
};

function App() {
  const { t, i18n } = useTranslation();
  const [rootPath, setRootPath] = useState("");
  const [admin, setAdmin] = useState<boolean | null>(null);
  const [message, setMessage] = useState("");
  const [status, setStatus] = useState<"idle" | "initialized" | "error">("idle");

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
    } catch (err) {
      setStatus("error");
      setMessage(t("status-error", { msg: String(err) }));
    }
  };

  const handleLocaleChange = (lng: string) => {
    i18n.changeLanguage(lng);
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
    </main>
  );
}

export default App;
