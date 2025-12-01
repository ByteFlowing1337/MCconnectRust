import React from 'react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { LogConsole } from '../components/LogConsole';
import { Server, Users, Wifi, WifiOff } from 'lucide-react';

interface ConnectionState {
  type: 'host' | 'client' | null;
  lobbyId: string | null;
  port: number | null;
}

interface HomeProps {
  onNavigate: (view: 'host' | 'client') => void;
  steamName: string;
  connectionState: ConnectionState;
}

export const Home: React.FC<HomeProps> = ({ onNavigate, steamName, connectionState }) => {
  const isConnected = connectionState.lobbyId !== null;

  return (
    <div className="flex flex-col items-center justify-center w-full py-8 space-y-8 animate-fade-in">
      <div className="text-center space-y-2">
        <h1 className="text-4xl font-bold bg-gradient-to-r from-blue-400 to-purple-500 bg-clip-text text-transparent">
          MC Connect
        </h1>
        <p className="text-white/60">Steam P2P Minecraft 联机工具</p>
      </div>

      <Card className="w-full max-w-md space-y-6">
        <div className="text-center pb-4 border-b border-white/10">
          <p className="text-sm text-white/50">当前 Steam 用户</p>
          <p className="text-xl font-medium text-white">{steamName || '加载中...'}</p>
        </div>

        {/* 连接状态指示 */}
        {isConnected && (
          <div className={`
            relative overflow-hidden
            bg-gradient-to-br 
            ${connectionState.type === 'host' 
              ? 'from-blue-500/15 via-blue-500/10 to-blue-500/15' 
              : 'from-purple-500/15 via-purple-500/10 to-purple-500/15'
            }
            backdrop-blur-xl border 
            ${connectionState.type === 'host' 
              ? 'border-blue-500/30' 
              : 'border-purple-500/30'
            }
            rounded-2xl p-4
            shadow-lg
            ${connectionState.type === 'host' 
              ? 'shadow-blue-500/10' 
              : 'shadow-purple-500/10'
            }
          `}>
            <div className="flex items-center space-x-3">
              <div className={`
                p-2 rounded-xl
                ${connectionState.type === 'host' 
                  ? 'bg-blue-500/20 border border-blue-500/30' 
                  : 'bg-purple-500/20 border border-purple-500/30'
                }
              `}>
                {isConnected ? (
                  <Wifi size={20} className={connectionState.type === 'host' ? 'text-blue-400' : 'text-purple-400'} />
                ) : (
                  <WifiOff size={20} className="text-white/40" />
                )}
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-semibold text-white/90">
                  {connectionState.type === 'host' ? '✓ 主机运行中' : '✓ 已连接到房间'}
                </p>
                {connectionState.lobbyId && (
                  <p className="text-xs text-white/60 mt-1 font-mono">
                    房间号: {connectionState.lobbyId}
                  </p>
                )}
                <p className="text-xs text-white/50 mt-1">
                  连接在后台保持运行，可随时返回查看
                </p>
              </div>
            </div>
          </div>
        )}

        <div className="grid grid-cols-2 gap-4">
          <Button 
            onClick={() => onNavigate('host')}
            className="h-32 flex flex-col items-center justify-center space-y-3 bg-gradient-to-br from-blue-600/20 to-blue-800/20 hover:from-blue-600/30 hover:to-blue-800/30 border border-blue-500/30"
          >
            <Server size={32} className="text-blue-400" />
            <span className="text-lg">我是房主</span>
          </Button>

          <Button 
            onClick={() => onNavigate('client')}
            className="h-32 flex flex-col items-center justify-center space-y-3 bg-gradient-to-br from-purple-600/20 to-purple-800/20 hover:from-purple-600/30 hover:to-purple-800/30 border border-purple-500/30"
          >
            <Users size={32} className="text-purple-400" />
            <span className="text-lg">加入房间</span>
          </Button>
        </div>
      </Card>

      <div className="w-full max-w-md">
        <LogConsole />
      </div>
    </div>
  );
};