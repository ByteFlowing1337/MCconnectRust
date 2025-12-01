import React, { useState, useEffect, useRef } from "react";
import {
  X,
  Terminal,
  Trash2,
  Download,
  ChevronDown,
  ChevronUp,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";

interface LogEntry {
  id: number;
  timestamp: string;
  level: string;
  message: string;
}

interface LogPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

// 全局日志存储，不会因组件卸载而丢失
let globalLogs: LogEntry[] = [];
let logIdCounter = 0;
let listenerInitialized = false;

// 初始化日志监听器（在模块加载时执行）
const initLogListener = async () => {
  if (listenerInitialized) return;
  listenerInitialized = true;

  try {
    await listen<{ level: string; message: string }>("log://log", (event) => {
      const newLog: LogEntry = {
        id: logIdCounter++,
        timestamp: new Date().toLocaleTimeString(),
        level: event.payload.level || "INFO",
        message: event.payload.message,
      };
      globalLogs = [...globalLogs.slice(-500), newLog];
    });
  } catch (e) {
    console.error("Failed to init log listener:", e);
  }
};

// 延迟初始化，避免阻塞
setTimeout(initLogListener, 100);

export const LogPanel: React.FC<LogPanelProps> = ({ isOpen, onClose }) => {
  const [logs, setLogs] = useState<LogEntry[]>(globalLogs);
  const [filter, setFilter] = useState<string>("all");
  const [autoScroll, setAutoScroll] = useState(true);
  const [isMinimized, setIsMinimized] = useState(false);
  const logContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // 定期同步全局日志到组件状态
    const interval = setInterval(() => {
      setLogs([...globalLogs]);
    }, 500);

    return () => clearInterval(interval);
  }, []);

  // 自动滚动到底部
  useEffect(() => {
    if (autoScroll && logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
    }
  }, [logs, autoScroll, isOpen]);

  const clearLogs = () => {
    globalLogs = [];
    setLogs([]);
  };

  const downloadLogs = () => {
    const logText = logs
      .map((log) => `[${log.timestamp}] [${log.level}] ${log.message}`)
      .join("\n");
    const blob = new Blob([logText], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `mcconnect-logs-${new Date().toISOString().slice(0, 10)}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const filteredLogs =
    filter === "all"
      ? logs
      : logs.filter((log) => log.level.toUpperCase() === filter.toUpperCase());

  const getLevelColor = (level: string) => {
    switch (level.toUpperCase()) {
      case "ERROR":
        return "text-red-400";
      case "WARN":
        return "text-yellow-400";
      case "INFO":
        return "text-blue-400";
      case "DEBUG":
        return "text-gray-400";
      default:
        return "text-white/70";
    }
  };

  const getLevelBg = (level: string) => {
    switch (level.toUpperCase()) {
      case "ERROR":
        return "bg-red-500/20";
      case "WARN":
        return "bg-yellow-500/20";
      case "INFO":
        return "bg-blue-500/20";
      case "DEBUG":
        return "bg-gray-500/20";
      default:
        return "bg-white/10";
    }
  };

  if (!isOpen) return null;

  return (
    <div
      style={{
        position: "fixed",
        bottom: 0,
        left: 0,
        right: 0,
        height: isMinimized ? "3rem" : "20rem",
        zIndex: 99999,
        backgroundColor: "#0a0f1a",
        borderTop: "1px solid rgba(255, 255, 255, 0.1)",
        boxShadow: "0 -10px 40px rgba(0, 0, 0, 0.5)",
        transition: "height 0.3s ease",
        pointerEvents: "auto",
        isolation: "isolate",
        contain: "layout paint",
        willChange: "height",
      }}
    >
      {/* 标题栏 */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-white/10 bg-black/20">
        <div className="flex items-center space-x-3">
          <Terminal size={18} className="text-green-400" />
          <span className="text-sm font-semibold text-white/90">调试日志</span>
          <span className="text-xs text-white/50">({logs.length} 条)</span>
        </div>

        <div className="flex items-center space-x-2">
          {/* 过滤器 */}
          <select
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="text-xs bg-white/10 border border-white/20 rounded px-2 py-1 text-white/80 focus:outline-none"
          >
            <option value="all">全部</option>
            <option value="ERROR">错误</option>
            <option value="WARN">警告</option>
            <option value="INFO">信息</option>
            <option value="DEBUG">调试</option>
          </select>

          {/* 自动滚动 */}
          <button
            onClick={() => setAutoScroll(!autoScroll)}
            className={`p-1.5 rounded text-xs ${
              autoScroll
                ? "bg-green-500/20 text-green-400"
                : "bg-white/10 text-white/50"
            }`}
            title="自动滚动"
          >
            自动滚动
          </button>

          {/* 下载日志 */}
          <button
            onClick={downloadLogs}
            className="p-1.5 rounded bg-white/10 text-white/70 hover:bg-white/20 transition-colors"
            title="下载日志"
          >
            <Download size={14} />
          </button>

          {/* 清空日志 */}
          <button
            onClick={clearLogs}
            className="p-1.5 rounded bg-white/10 text-white/70 hover:bg-red-500/30 hover:text-red-400 transition-colors"
            title="清空日志"
          >
            <Trash2 size={14} />
          </button>

          {/* 最小化/展开 */}
          <button
            onClick={() => setIsMinimized(!isMinimized)}
            className="p-1.5 rounded bg-white/10 text-white/70 hover:bg-white/20 transition-colors"
            title={isMinimized ? "展开" : "最小化"}
          >
            {isMinimized ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
          </button>

          {/* 关闭 */}
          <button
            onClick={onClose}
            className="p-1.5 rounded bg-white/10 text-white/70 hover:bg-red-500/30 hover:text-red-400 transition-colors"
            title="关闭"
          >
            <X size={14} />
          </button>
        </div>
      </div>

      {/* 日志内容 */}
      {!isMinimized && (
        <div
          ref={logContainerRef}
          className="h-[calc(100%-48px)] overflow-y-auto font-mono text-xs custom-scrollbar"
        >
          {filteredLogs.length === 0 ? (
            <div className="flex items-center justify-center h-full text-white/30">
              暂无日志
            </div>
          ) : (
            <div className="p-2 space-y-1">
              {filteredLogs.map((log) => (
                <div
                  key={log.id}
                  className={`flex items-start space-x-2 px-2 py-1 rounded ${getLevelBg(
                    log.level
                  )}`}
                >
                  <span className="text-white/40 flex-shrink-0">
                    {log.timestamp}
                  </span>
                  <span
                    className={`font-semibold flex-shrink-0 w-12 ${getLevelColor(
                      log.level
                    )}`}
                  >
                    [{log.level.toUpperCase().slice(0, 4)}]
                  </span>
                  <span className="text-white/80 break-all">{log.message}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

// 日志按钮组件，用于打开日志面板
export const LogButton: React.FC<{
  onClick: () => void;
  hasError?: boolean;
}> = ({ onClick, hasError }) => {
  return (
    <button
      onClick={onClick}
      style={{
        position: "fixed",
        bottom: "1rem",
        right: "1rem",
        padding: "0.75rem",
        borderRadius: "9999px",
        backgroundColor: "#0a0f1a",
        border: hasError
          ? "1px solid rgba(239, 68, 68, 0.5)"
          : "1px solid rgba(255, 255, 255, 0.2)",
        boxShadow: "0 10px 15px -3px rgba(0, 0, 0, 0.3)",
        cursor: "pointer",
        transition: "all 0.2s ease",
        zIndex: 99998,
        isolation: "isolate",
      }}
      title="打开调试日志"
    >
      <Terminal size={20} style={{ color: hasError ? "#f87171" : "#4ade80" }} />
    </button>
  );
};
