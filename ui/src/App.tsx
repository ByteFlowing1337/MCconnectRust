import { useState, useEffect } from "react";
import { Home } from "./views/Home";
import { Host } from "./views/Host";
import { Client } from "./views/Client";
import { LogPanel, LogButton } from "./components/LogPanel";
import { invoke } from "@tauri-apps/api/core";

type View = "home" | "host" | "client";

// 全局连接状态
interface ConnectionState {
  type: "host" | "client" | null;
  lobbyId: string | null;
  port: number | null;
}

function App() {
  const [view, setView] = useState<View>("home");
  const [steamName, setSteamName] = useState("");
  const [showLogs, setShowLogs] = useState(false);
  const [connectionState, setConnectionState] = useState<ConnectionState>({
    type: null,
    lobbyId: null,
    port: null,
  });

  const checkActiveConnection = async () => {
    try {
      const lobbyId = await invoke<number | null>("get_lobby_id");
      if (lobbyId) {
        // 有活跃的连接，保持之前的状态
        setConnectionState((prev) => ({
          ...prev,
          lobbyId: lobbyId.toString(),
          // 如果之前没有类型，保持null（不推断）
        }));
      } else {
        // 没有活跃连接，清除状态
        setConnectionState({
          type: null,
          lobbyId: null,
          port: null,
        });
      }
    } catch (e) {
      console.error("Failed to check connection:", e);
    }
  };

  useEffect(() => {
    invoke<string>("get_steam_name")
      .then(setSteamName)
      .catch((e) => {
        console.error("Failed to get Steam name:", e);
        setSteamName("Steam 未连接");
      });

    // 检查是否有活跃的连接
    checkActiveConnection();

    // 定期检查连接状态
    const interval = setInterval(() => {
      checkActiveConnection();
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  return (
    <>
      <div className="min-h-screen w-screen bg-transparent flex flex-col">
        <div className="flex-1 bg-[#0f172a] text-white font-sans selection:bg-blue-500/30 rounded-xl border border-white/10 shadow-2xl flex flex-col relative m-2 min-h-0">
          {/* Modern Background Effects */}
          <div className="absolute top-0 left-0 w-full h-full overflow-hidden pointer-events-none rounded-xl">
            <div className="absolute top-[-10%] left-[-10%] w-[40%] h-[40%] bg-blue-600/20 rounded-full blur-[100px] animate-pulse" />
            <div
              className="absolute bottom-[-10%] right-[-10%] w-[40%] h-[40%] bg-purple-600/20 rounded-full blur-[100px] animate-pulse"
              style={{ animationDelay: "1s" }}
            />
            <div className="absolute top-[40%] left-[50%] transform -translate-x-1/2 -translate-y-1/2 w-[60%] h-[60%] bg-indigo-900/20 rounded-full blur-[120px]" />
          </div>

          <div className="absolute inset-0 bg-[url('https://images.unsplash.com/photo-1614728853913-1e22ba0e982b?q=80&w=2574&auto=format&fit=crop')] bg-cover bg-center opacity-5 pointer-events-none mix-blend-overlay rounded-xl" />

          <main className="relative z-10 container mx-auto px-4 py-6 flex-1 flex flex-col overflow-y-auto custom-scrollbar">
            <div className="flex-1 flex flex-col">
              {view === "home" && (
                <Home
                  onNavigate={setView}
                  steamName={steamName}
                  connectionState={connectionState}
                />
              )}
              {view === "host" && (
                <Host
                  onBack={() => {
                    // 返回主页时保持连接状态
                    checkActiveConnection();
                    setView("home");
                  }}
                  connectionState={connectionState}
                  onConnectionChange={(lobbyId, port) => {
                    setConnectionState({
                      type: "host",
                      lobbyId,
                      port,
                    });
                  }}
                />
              )}
              {view === "client" && (
                <Client
                  onBack={() => {
                    // 返回主页时保持连接状态
                    checkActiveConnection();
                    setView("home");
                  }}
                  connectionState={connectionState}
                  onConnectionChange={(lobbyId) => {
                    setConnectionState({
                      type: "client",
                      lobbyId,
                      port: null,
                    });
                  }}
                />
              )}
            </div>
          </main>

          <footer className="relative z-10 text-center text-white/20 text-xs py-3 flex-shrink-0">
            MC Connect Rust v0.1.0
          </footer>
        </div>
      </div>

      {/* 日志按钮和面板 - 放在最外层，独立于主内容 */}
      <LogButton onClick={() => setShowLogs(true)} />
      <LogPanel isOpen={showLogs} onClose={() => setShowLogs(false)} />
    </>
  );
}

export default App;
