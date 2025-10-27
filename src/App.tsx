import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  register as registerShortcut,
  unregisterAll as unregisterAllShortcuts,
} from "@tauri-apps/plugin-global-shortcut";
import { CatIcon, ClipboardIcon, MoonIcon, StarIcon, SunIcon, TagIcon, TrashIcon } from "./icons";

interface ClipboardItem {
  id: number;
  content_type: string;
  content: string;
  preview: string;
  is_favorite: boolean;
  tags: string[];
  created_at: string;
}

type ThemeMode = "light" | "dark" | "auto";

interface AppConfig {
  max_history_items: number;
  auto_start: boolean;
  theme: ThemeMode;
  hotkey: string;
}

const SEARCH_DEBOUNCE = 260;
const MAX_HISTORY_MIN = 1;
const MAX_HISTORY_MAX = 5000;

const normalizeShortcutForPlugin = (shortcut: string) => {
  return shortcut
    .split("+")
    .map((segment) => {
      const token = segment.trim();
      if (!token) return token;

      switch (token.toLowerCase()) {
        case "win":
        case "windows":
        case "meta":
        case "super":
          return "Super";
        case "command":
        case "cmd":
          return "Command";
        case "commandorcontrol":
        case "commandorctrl":
        case "cmdorcontrol":
        case "cmdorctrl":
          return "CommandOrControl";
        case "control":
          return "Control";
        case "ctrl":
          return "Ctrl";
        case "alt":
        case "option":
          return "Alt";
        case "shift":
          return "Shift";
        default:
          return token;
      }
    })
    .join("+");
};

function App() {
  const [items, setItems] = useState<ClipboardItem[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [selectedItem, setSelectedItem] = useState<number | null>(null);
  const [tagInput, setTagInput] = useState("");
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [favoritesOnly, setFavoritesOnly] = useState(false);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [hotkeyDraft, setHotkeyDraft] = useState("");
  const [maxHistoryDraft, setMaxHistoryDraft] = useState("");
  const [savingConfig, setSavingConfig] = useState(false);
  const [autostartBusy, setAutostartBusy] = useState(false);
  const [resettingApp, setResettingApp] = useState(false);
  const searchRef = useRef("");
  const themeMediaQuery = useRef<MediaQueryList | null>(null);
  const themeMediaListener = useRef<((event: MediaQueryListEvent) => void) | null>(null);

  const showStatus = useCallback((message: string, duration = 2200) => {
    setStatusMessage(message);
    if (duration > 0) {
      setTimeout(() => setStatusMessage(null), duration);
    }
  }, []);

  const detachThemeListener = useCallback(() => {
    if (themeMediaQuery.current && themeMediaListener.current) {
      themeMediaQuery.current.removeEventListener("change", themeMediaListener.current);
    }
    themeMediaQuery.current = null;
    themeMediaListener.current = null;
  }, []);

  const applyTheme = useCallback(
    (mode: ThemeMode) => {
      const root = document.body;
      if (!root) return;

      detachThemeListener();

      let resolved: ThemeMode = mode;
      if (mode === "auto") {
        const media = window.matchMedia("(prefers-color-scheme: dark)");
        themeMediaQuery.current = media;
        resolved = media.matches ? "dark" : "light";

        const handler = (event: MediaQueryListEvent) => {
          const theme = event.matches ? "dark" : "light";
          root.dataset.theme = theme;
          document.documentElement.style.setProperty("color-scheme", theme);
        };

        themeMediaListener.current = handler;
        media.addEventListener("change", handler);
      }

      const finalTheme = resolved === "dark" ? "dark" : "light";
      root.dataset.theme = finalTheme;
      root.dataset.themeMode = mode;
      document.documentElement.style.setProperty("color-scheme", finalTheme);
    },
    [detachThemeListener]
  );

  const toggleMainWindow = useCallback(async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      const visible = await win.isVisible();
      
      if (visible) {
        await win.hide();
      } else {
        await win.show();
        await win.setFocus();
      }
    } catch (error) {
      console.error("Failed to toggle main window:", error);
    }
  }, []);

  const revealMainWindow = useCallback(async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      await win.show();
      await win.setFocus();
    } catch (error) {
      console.error("Failed to reveal main window:", error);
    }
  }, []);

  const persistConfig = useCallback(
    async (next: AppConfig, message?: string) => {
      setSavingConfig(true);
      try {
    await invoke("update_config", { newConfig: next });
    setConfig(next);
    setHotkeyDraft(next.hotkey);
    setMaxHistoryDraft(String(next.max_history_items));
        applyTheme(next.theme);
        if (message) {
          showStatus(message, 1600);
        }
      } catch (error) {
        console.error("Failed to persist config:", error);
        showStatus("保存设置失败，请稍后再试", 2200);
      } finally {
        setSavingConfig(false);
      }
    },
    [applyTheme, showStatus]
  );

  useEffect(() => {
    let isMounted = true;

    const initialize = async () => {
      try {
        const initial = await invoke<AppConfig>("get_config");
        if (!isMounted) return;
        setConfig(initial);
        setHotkeyDraft(initial.hotkey);
        setMaxHistoryDraft(String(initial.max_history_items));
        applyTheme(initial.theme);
      } catch (error) {
        console.error("Failed to initialize config:", error);
        showStatus("读取设置失败，请稍后再试", 2400);
      } finally {
        if (isMounted) {
          setAutostartBusy(false);
        }
      }
    };

    initialize();

    return () => {
      isMounted = false;
      detachThemeListener();
    };
  }, [applyTheme, detachThemeListener, showStatus]);

  const updateAutostart = useCallback(
    async (enabled: boolean) => {
      if (!config) return;
      setAutostartBusy(true);
      try {
        const updated = await invoke<AppConfig>("set_autostart", { enabled });
        setConfig(updated);
        setHotkeyDraft(updated.hotkey);
        setMaxHistoryDraft(String(updated.max_history_items));
        applyTheme(updated.theme);
        showStatus(enabled ? "已开启开机自启动" : "已关闭开机自启动", 1600);
      } catch (error) {
        console.error("Failed to toggle autostart:", error);
        showStatus("更新开机自启失败", 2200);
      } finally {
        setAutostartBusy(false);
      }
    },
    [applyTheme, config, showStatus]
  );

  const handleThemeModeChange = useCallback(
    async (mode: ThemeMode) => {
      if (!config || config.theme === mode) return;
      await persistConfig({ ...config, theme: mode }, "主题偏好已更新");
    },
    [config, persistConfig]
  );

  const cycleThemeMode = useCallback(async () => {
    if (!config) return;
    const order: ThemeMode[] = ["auto", "light", "dark"];
    const next = order[(order.indexOf(config.theme) + 1) % order.length];
    await handleThemeModeChange(next);
  }, [config, handleThemeModeChange]);

  const handleHotkeySave = useCallback(async () => {
    if (!config) return;
    const value = hotkeyDraft.trim();
    if (!value) {
      showStatus("快捷键不能为空", 1800);
      return;
    }
    await persistConfig({ ...config, hotkey: value }, "快捷键已更新");
  }, [config, hotkeyDraft, persistConfig, showStatus]);

  useEffect(() => {
    let unlistenTheme: UnlistenFn | undefined;
    let unlistenAutostart: UnlistenFn | undefined;
    let unlistenSettings: UnlistenFn | undefined;
    let unlistenOpen: UnlistenFn | undefined;

    const bindTrayEvents = async () => {
      unlistenTheme = await listen("tray-toggle-theme", () => {
        void cycleThemeMode();
      });

      unlistenAutostart = await listen("tray-toggle-autostart", () => {
        if (!config) return;
        void updateAutostart(!config.auto_start);
      });

      unlistenSettings = await listen("tray-open-settings", () => {
        setSettingsOpen(true);
      });

      unlistenOpen = await listen("tray-open-main", () => {
        void revealMainWindow();
      });
    };

    bindTrayEvents();

    return () => {
      if (unlistenTheme) unlistenTheme();
      if (unlistenAutostart) unlistenAutostart();
      if (unlistenSettings) unlistenSettings();
      if (unlistenOpen) unlistenOpen();
    };
  }, [config, cycleThemeMode, updateAutostart, revealMainWindow]);

  useEffect(() => {
    if (!settingsOpen) return;
    const handleKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setSettingsOpen(false);
      }
    };

    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [settingsOpen]);

  const loadHistory = useCallback(
    async ({ silent = false, limitOverride }: { silent?: boolean; limitOverride?: number } = {}) => {
      const limit = limitOverride ?? config?.max_history_items ?? 120;
      if (!silent) {
        setLoading(true);
      }
      try {
        const history = await invoke<ClipboardItem[]>("get_history", {
          limit,
          offset: 0,
        });
        setItems(history);
        setSelectedItem(null);
      } catch (error) {
        console.error("Failed to load history:", error);
        showStatus("载入剪贴板历史失败");
      } finally {
        if (!silent) {
          setLoading(false);
        }
      }
    },
    [config?.max_history_items, showStatus]
  );

  const handleResetApplication = useCallback(async () => {
    if (resettingApp) return;
    if (!window.confirm("确定要重置吗？所有历史记录、标签与设置将被清除。")) {
      return;
    }

    setResettingApp(true);
    try {
      const fresh = await invoke<AppConfig>("reset_application");
      setConfig(fresh);
      setHotkeyDraft(fresh.hotkey);
      setMaxHistoryDraft(String(fresh.max_history_items));
      setFavoritesOnly(false);
      setSearchQuery("");
      applyTheme(fresh.theme);
      await loadHistory({ silent: true, limitOverride: fresh.max_history_items });
      showStatus("已恢复初始状态", 2200);
    } catch (error) {
      console.error("Failed to reset application:", error);
      showStatus("重置失败，请稍后重试", 2400);
    } finally {
      setResettingApp(false);
    }
  }, [applyTheme, loadHistory, resettingApp, showStatus]);

  const searchHistory = useCallback(
    async (query: string) => {
      const keyword = query.trim();
      if (!keyword) {
        await loadHistory();
        return;
      }

      setLoading(true);
      try {
        const limit = config?.max_history_items ?? 120;
        const results = await invoke<ClipboardItem[]>("search_history", {
          query: keyword,
          limit,
        });
        setItems(results);
      } catch (error) {
        console.error("Failed to search history:", error);
        showStatus("搜索失败，请稍后再试");
      } finally {
        setLoading(false);
      }
    },
    [config?.max_history_items, loadHistory, showStatus]
  );

  const handleMaxHistorySave = useCallback(async () => {
    if (!config) return;
    const trimmed = maxHistoryDraft.trim();
    if (!trimmed) {
      showStatus("历史数量不能为空", 1800);
      return;
    }

    const parsed = Number.parseInt(trimmed, 10);
    if (Number.isNaN(parsed)) {
      showStatus("历史数量需为数字", 2000);
      return;
    }

    if (parsed < MAX_HISTORY_MIN || parsed > MAX_HISTORY_MAX) {
      showStatus(`历史数量需在 ${MAX_HISTORY_MIN} - ${MAX_HISTORY_MAX} 之间`, 2200);
      return;
    }

    await persistConfig({ ...config, max_history_items: parsed }, "历史数量上限已更新");
    await loadHistory({ silent: true, limitOverride: parsed });
  }, [config, maxHistoryDraft, persistConfig, loadHistory, showStatus]);

  const handleCopy = useCallback(
    async (item: ClipboardItem) => {
      if (item.content_type !== "text") {
        showStatus("当前版本仅支持将文本记录复制回剪贴板");
        return;
      }

      try {
        await invoke("copy_to_clipboard", { content: item.content });
        window.setTimeout(() => {
          void loadHistory({ silent: true });
        }, 200);
        setSelectedItem(null);
        showStatus("已复制到剪贴板", 1600);
      } catch (error) {
        console.error("Failed to copy:", error);
        showStatus("复制失败，请重试");
      }
    },
    [loadHistory, showStatus]
  );

  const toggleFavorite = useCallback(
    async (id: number) => {
      try {
        await invoke("toggle_favorite", { id });
        await loadHistory({ silent: true });
      } catch (error) {
        console.error("Failed to toggle favorite:", error);
        showStatus("收藏操作失败");
      }
    },
    [loadHistory, showStatus]
  );

  const deleteItem = useCallback(
    async (id: number) => {
      try {
        await invoke("delete_item", { id });
        await loadHistory({ silent: true });
        showStatus("已删除记录", 1500);
      } catch (error) {
        console.error("Failed to delete:", error);
        showStatus("删除失败，请稍后再试");
      }
    },
    [loadHistory, showStatus]
  );

  const clearHistory = useCallback(async () => {
    try {
      await invoke("clear_history");
      await loadHistory({ silent: true });
      showStatus("历史记录已清空", 1500);
    } catch (error) {
      console.error("Failed to clear history:", error);
      showStatus("清空失败，请稍后再试");
    }
  }, [loadHistory, showStatus]);

  const addTag = useCallback(
    async (itemId: number, tagName: string) => {
      const value = tagName.trim();
      if (!value) return;

      try {
        await invoke("add_tag", { itemId, tagName: value });
        setTagInput("");
        await loadHistory({ silent: true });
      } catch (error) {
        console.error("Failed to add tag:", error);
        showStatus("添加标签失败");
      }
    },
    [loadHistory, showStatus]
  );

  const removeTag = useCallback(
    async (itemId: number, tagName: string) => {
      try {
        await invoke("remove_tag", { itemId, tagName });
        await loadHistory({ silent: true });
      } catch (error) {
        console.error("Failed to remove tag:", error);
        showStatus("移除标签失败");
      }
    },
    [loadHistory, showStatus]
  );

  const formatTime = useCallback((dateStr: string) => {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);

    if (minutes < 1) return "刚刚";
    if (minutes < 60) return `${minutes} 分钟前`;
    if (hours < 24) return `${hours} 小时前`;
    if (days < 7) return `${days} 天前`;

    return date.toLocaleString("zh-CN", {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }, []);

  useEffect(() => {
    let unlistenHistory: UnlistenFn | undefined;
    let unlistenClipboard: UnlistenFn | undefined;

    const setup = async () => {
      await loadHistory();
      unlistenHistory = await listen<number>("history-updated", () => {
        const keyword = searchRef.current.trim();
        if (keyword) {
          void searchHistory(keyword);
        } else {
          void loadHistory({ silent: true });
        }
      });
      unlistenClipboard = await listen("clipboard-changed", () => {
        const keyword = searchRef.current.trim();
        if (keyword) {
          void searchHistory(keyword);
        } else {
          void loadHistory({ silent: true });
        }
      });
    };

    setup();

    return () => {
      if (unlistenHistory) {
        unlistenHistory();
      }
      if (unlistenClipboard) {
        unlistenClipboard();
      }
    };
  }, [loadHistory, searchHistory]);

  useEffect(() => {
    searchRef.current = searchQuery;
  }, [searchQuery]);

  useEffect(() => {
    const combination = config?.hotkey?.trim();
    if (!combination) {
      void unregisterAllShortcuts().catch((error) => {
        console.error("Failed to unregister global shortcuts:", error);
      });
      return;
    }

    let disposed = false;

    const applyShortcut = async () => {
      try {
        await unregisterAllShortcuts();
        if (disposed) return;

        const normalized = normalizeShortcutForPlugin(combination);
        await registerShortcut(normalized, (event) => {
          if (event.state === "Pressed") {
            void toggleMainWindow();
          }
        });
      } catch (error) {
        console.error("Failed to register global shortcut:", error);
        if (!disposed) {
          showStatus("注册快捷键失败，请检查权限", 2400);
        }
      }
    };

    applyShortcut();

    return () => {
      disposed = true;
      void unregisterAllShortcuts().catch((error) => {
        console.error("Failed to unregister global shortcuts:", error);
      });
    };
  }, [config?.hotkey, toggleMainWindow, showStatus]);

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | null = null;

    void (async () => {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const currentWindow = getCurrentWindow();
      const off = await currentWindow.onFocusChanged(({ payload }: { payload: boolean }) => {
        if (payload) {
          void loadHistory({ silent: true });
        }
      });

      if (disposed) {
        off();
      } else {
        unlisten = off;
      }
    })();

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [loadHistory]);

  useEffect(() => {
    const timer = setTimeout(() => {
      searchHistory(searchQuery);
    }, SEARCH_DEBOUNCE);

    return () => clearTimeout(timer);
  }, [searchQuery, searchHistory]);

  const filteredItems = useMemo(() => {
    if (favoritesOnly) {
      return items.filter((item) => item.is_favorite);
    }
    return items;
  }, [favoritesOnly, items]);

  const renderTypeLabel = useCallback((type: string) => {
    switch (type) {
      case "text":
        return "文本";
      case "file":
        return "文件";
      case "image":
        return "图片";
      default:
        return type;
    }
  }, []);

  return (
    <div className="app-shell">
      <div className="window">
        <header className="window__header">
          <div className="brand">
            <div className="brand__icon">
              <CatIcon className="svg-icon" />
            </div>
            <div className="brand__text">
              <h1>Cat History</h1>
              <span>猫猫剪贴板 · 轻盈陪伴你的灵感</span>
            </div>
          </div>

          <div className="utility">
            <div className="search">
              <input
                type="text"
                placeholder="搜索剪贴板内容、标签或类型"
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
              />
              <button
                className={`icon-btn favorites-toggle ${favoritesOnly ? "is-active" : ""}`}
                onClick={() =>
                  setFavoritesOnly((prev) => {
                    const next = !prev;
                    showStatus(next ? "仅显示收藏内容" : "显示全部历史", 1400);
                    return next;
                  })
                }
                title={favoritesOnly ? "显示全部" : "只看收藏"}
                aria-label={favoritesOnly ? "显示全部" : "只看收藏"}
              >
                <StarIcon filled={favoritesOnly} className="svg-icon" />
              </button>
            </div>
            <div className="utility__actions">
              <button className="btn" onClick={() => void loadHistory()}>
                刷新
              </button>
              <button className="btn btn--ghost" onClick={() => void clearHistory()}>
                清空
              </button>
              <button
                className="btn btn--ghost"
                onClick={() => setSettingsOpen(true)}
                aria-label="打开设置"
              >
                设置
              </button>
            </div>
          </div>

          {statusMessage && <div className="status-banner">{statusMessage}</div>}
        </header>

        <main className="history">
          {loading ? (
            <div className="history__fallback">
              <div className="spinner" aria-label="加载中" />
              <p>正在读取剪贴板历史…</p>
            </div>
          ) : filteredItems.length === 0 ? (
            <div className="history__fallback">
              <div className="empty-icon">
                <ClipboardIcon className="svg-icon" />
              </div>
              <p>
                {favoritesOnly
                  ? "暂无收藏内容"
                  : searchQuery
                  ? "没有找到匹配内容"
                  : "剪贴板目前是空的"}
              </p>
            </div>
          ) : (
            <ul className="history__list">
              {filteredItems.map((item) => {
                const previewText = item.content_type === "text" ? item.content : item.preview;

                return (
                  <li
                    key={item.id}
                    className={`history-card ${item.is_favorite ? "history-card--favorite" : ""}`}
                    onClick={() => void handleCopy(item)}
                  >
                  <div className="history-card__header">
                    <span className={`pill pill--${item.content_type}`}>
                      {renderTypeLabel(item.content_type)}
                    </span>
                    <div className="history-card__actions">
                      <button
                        className={`icon-btn ${item.is_favorite ? "is-active" : ""}`}
                        onClick={(event) => {
                          event.stopPropagation();
                          void toggleFavorite(item.id);
                        }}
                        title={item.is_favorite ? "取消收藏" : "收藏"}
                        aria-label={item.is_favorite ? "取消收藏" : "收藏"}
                      >
                        <StarIcon filled={item.is_favorite} className="svg-icon" />
                      </button>
                      <button
                        className="icon-btn"
                        onClick={(event) => {
                          event.stopPropagation();
                          setSelectedItem((prev) => (prev === item.id ? null : item.id));
                        }}
                        title="管理标签"
                        aria-label="管理标签"
                      >
                        <TagIcon className="svg-icon" />
                      </button>
                      <button
                        className="icon-btn"
                        onClick={(event) => {
                          event.stopPropagation();
                          void deleteItem(item.id);
                        }}
                        title="删除记录"
                        aria-label="删除记录"
                      >
                        <TrashIcon className="svg-icon" />
                      </button>
                    </div>
                  </div>

                  <pre className="history-card__preview">{previewText}</pre>

                  <footer className="history-card__meta">
                    <div className="history-card__tags">
                      {item.tags.map((tag) => (
                        <span
                          key={`${item.id}-${tag}`}
                          className="tag-chip"
                          onClick={(event) => {
                            event.stopPropagation();
                            void removeTag(item.id, tag);
                          }}
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                    <time className="history-card__time">{formatTime(item.created_at)}</time>
                  </footer>

                  {selectedItem === item.id && (
                    <div className="tag-editor" onClick={(event) => event.stopPropagation()}>
                      <input
                        type="text"
                        placeholder="输入标签并回车"
                        value={tagInput}
                        onChange={(event) => setTagInput(event.target.value)}
                        onKeyDown={(event) => {
                          if (event.key === "Enter") {
                            void addTag(item.id, tagInput);
                          } else if (event.key === "Escape") {
                            setSelectedItem(null);
                          }
                        }}
                        autoFocus
                      />
                    </div>
                  )}
                </li>
                );
              })}
            </ul>
          )}
        </main>

        {settingsOpen && config && (
          <div
            className="settings-overlay"
            role="dialog"
            aria-modal="true"
            onClick={() => setSettingsOpen(false)}
          >
            <div className="settings-panel" onClick={(event) => event.stopPropagation()}>
              <header className="settings-panel__header">
                <div>
                  <h2>应用设置</h2>
                  <p>调整猫猫剪贴板的主题与行为</p>
                </div>
                <button
                  className="icon-btn close-btn"
                  onClick={() => setSettingsOpen(false)}
                  aria-label="关闭设置"
                >
                  ×
                </button>
              </header>

              <section className="settings-group">
                <div className="settings-group__heading">
                  <h3>主题模式</h3>
                  <span>可随时切换浅色、深色或跟随系统</span>
                </div>
                <div className="theme-options">
                  <button
                    className={`theme-option ${config.theme === "auto" ? "is-active" : ""}`}
                    onClick={() => void handleThemeModeChange("auto")}
                  >
                    <span className="theme-option__icon">🪄</span>
                    <span className="theme-option__label">跟随</span>
                  </button>
                  <button
                    className={`theme-option ${config.theme === "light" ? "is-active" : ""}`}
                    onClick={() => void handleThemeModeChange("light")}
                  >
                    <SunIcon className="svg-icon" />
                    <span className="theme-option__label">浅色</span>
                  </button>
                  <button
                    className={`theme-option ${config.theme === "dark" ? "is-active" : ""}`}
                    onClick={() => void handleThemeModeChange("dark")}
                  >
                    <MoonIcon className="svg-icon" />
                    <span className="theme-option__label">深色</span>
                  </button>
                </div>
              </section>

              <section className="settings-group">
                <div className="settings-group__heading">
                  <h3>开机自启动</h3>
                  <span>让 Cat History 随系统一起醒来</span>
                </div>
                <div className="toggle-row">
                  <span>{config.auto_start ? "已开启" : "已关闭"}</span>
                  <button
                    className={`switch ${config.auto_start ? "is-on" : ""}`}
                    disabled={autostartBusy}
                    onClick={() => void updateAutostart(!config.auto_start)}
                    aria-label="切换开机自启动"
                  >
                    <span className="switch__thumb" />
                  </button>
                </div>
                {autostartBusy && <p className="settings-hint">正在应用开机启动设置…</p>}
              </section>

              <section className="settings-group">
                <div className="settings-group__heading">
                  <h3>历史容量</h3>
                  <span>超过上限时最早的记录会自动清理</span>
                </div>
                <div className="settings-row">
                  <input
                    type="number"
                    min={MAX_HISTORY_MIN}
                    max={MAX_HISTORY_MAX}
                    value={maxHistoryDraft}
                    onChange={(event) => setMaxHistoryDraft(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") {
                        void handleMaxHistorySave();
                      }
                    }}
                    placeholder="例如 200"
                    aria-label="剪贴板历史上限"
                  />
                  <button
                    className="btn"
                    onClick={() => void handleMaxHistorySave()}
                    disabled={savingConfig}
                  >
                    {savingConfig ? "保存中…" : "保存"}
                  </button>
                </div>
                <p className="settings-hint">
                  支持 {MAX_HISTORY_MIN} ~ {MAX_HISTORY_MAX} 条记录，达到上限后会移除最早复制的内容。
                </p>
              </section>

              <section className="settings-group">
                <div className="settings-group__heading">
                  <h3>快捷键</h3>
                  <span>用于快速打开历史面板</span>
                </div>
                <div className="settings-row">
                  <input
                    type="text"
                    value={hotkeyDraft}
                    onChange={(event) => setHotkeyDraft(event.target.value)}
                    placeholder="例如 Ctrl+Shift+V"
                  />
                  <button
                    className="btn"
                    onClick={() => void handleHotkeySave()}
                    disabled={savingConfig}
                  >
                    {savingConfig ? "保存中…" : "保存"}
                  </button>
                </div>
                <p className="settings-hint">使用 + 连接组合键，支持 Ctrl、Alt、Shift、Win 等键位。</p>
              </section>

              <section className="settings-group settings-group--danger">
                <div className="settings-group__heading">
                  <h3>重置应用</h3>
                  <span>清空历史记录与设置，恢复到初始状态</span>
                </div>
                <p className="settings-hint">此操作不可撤销，将立即删除所有剪贴板数据与标签。</p>
                <button
                  className="btn btn--danger"
                  onClick={() => void handleResetApplication()}
                  disabled={resettingApp}
                >
                  {resettingApp ? "重置中…" : "恢复初始设置"}
                </button>
              </section>

              <footer className="settings-panel__footer">
                <button className="btn btn--ghost" onClick={() => setSettingsOpen(false)}>
                  完成
                </button>
              </footer>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
