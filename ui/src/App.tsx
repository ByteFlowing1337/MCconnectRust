import { useState, useEffect } from 'react';
import { Home } from './views/Home';
import { Host } from './views/Host';
import { Client } from './views/Client';
import { invoke } from '@tauri-apps/api/core';

type View = 'home' | 'host' | 'client';

// 全局连接状态
interface ConnectionState {
  type: 'host' | 'client' | null;
  lobbyId: string | null;
  port: number | null;
}

function App() {
  const [view, setView] = useState<View>('home');
  const [steamName, setSteamName] = useState('');
  const [connectionState, setConnectionState] = useState<ConnectionState>({
    type: null,
    lobbyId: null,
    port: null,
  });

  const checkActiveConnection = async () => {
    try {
      const lobbyId = await invoke<number | null>('get_lobby_id');
      if (lobbyId) {
        // 有活跃的连接，保持之前的状态
        setConnectionState(prev => ({
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
      console.error('Failed to check connection:', e);
    }
  };

  useEffect(() => {
    invoke<string>('get_steam_name')
      .then(setSteamName)
      .catch(console.error);
    
    // 检查是否有活跃的连接
    checkActiveConnection();
    
    // 定期检查连接状态
    const interval = setInterval(() => {
      checkActiveConnection();
    }, 2000);
    
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-800 to-black text-white font-sans selection:bg-blue-500/30">
      <div className="fixed inset-0 bg-[url('https://images.unsplash.com/photo-1614728853913-1e22ba0e982b?q=80&w=2574&auto=format&fit=crop')] bg-cover bg-center opacity-20 pointer-events-none" />
      
      <main className="relative z-10 container mx-auto px-4 py-8 min-h-screen flex flex-col">
        <div className="flex-1 flex flex-col min-h-0">
          {view === 'home' && (
            <Home 
              onNavigate={setView} 
              steamName={steamName}
              connectionState={connectionState}
            />
          )}
          {view === 'host' && (
            <Host 
              onBack={() => {
                // 返回主页时保持连接状态
                checkActiveConnection();
                setView('home');
              }}
              connectionState={connectionState}
              onConnectionChange={(lobbyId, port) => {
                setConnectionState({
                  type: 'host',
                  lobbyId,
                  port,
                });
              }}
            />
          )}
          {view === 'client' && (
            <Client 
              onBack={() => {
                // 返回主页时保持连接状态
                checkActiveConnection();
                setView('home');
              }}
              connectionState={connectionState}
              onConnectionChange={(lobbyId) => {
                setConnectionState({
                  type: 'client',
                  lobbyId,
                  port: null,
                });
              }}
            />
          )}
        </div>
        
        <footer className="text-center text-white/20 text-xs py-4 mt-auto flex-shrink-0">
          MC Connect Rust v0.1.0
        </footer>
      </main>
    </div>
  );
}

export default App;
