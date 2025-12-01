import React, { useState, useEffect } from 'react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { PerformancePanel } from '../components/PerformancePanel';
import { ArrowLeft, LogIn, Loader2, CheckCircle, Globe, Radio, Server, Wifi, XCircle } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

interface ConnectionState {
  type: 'host' | 'client' | null;
  lobbyId: string | null;
  port: number | null;
}

interface ClientProps {
  onBack: () => void;
  connectionState?: ConnectionState;
  onConnectionChange?: (lobbyId: string | null) => void;
}

interface PerformanceMetrics {
  packets_sent: number;
  packets_received: number;
  bytes_sent: number;
  bytes_received: number;
  packets_dropped: number;
  send_rate_mbps: number;
  recv_rate_mbps: number;
  send_rate_pps: number;
  recv_rate_pps: number;
  latency_ms?: number | null;
}

interface LanServer {
  ip: string;
  port: number;
  motd: string;
  latency_ms: number;
}

export const Client: React.FC<ClientProps> = ({ onBack, connectionState, onConnectionChange }) => {
  const [lobbyId, setLobbyId] = useState('');
  const [password, setPassword] = useState('');
  const [status, setStatus] = useState<'idle' | 'connecting' | 'connected' | 'error'>('idle');
  const [message, setMessage] = useState('');
  const [metrics, setMetrics] = useState<PerformanceMetrics | null>(null);
  const [lanServer, setLanServer] = useState<LanServer | null>(null);
  const [discoveryStatus, setDiscoveryStatus] = useState<'idle' | 'scanning' | 'found' | 'not_found'>('idle');


  // 检查是否有活跃的连接，恢复状态
  useEffect(() => {
    const restoreState = async () => {
      try {
        const id = await invoke<number | null>('get_lobby_id');
        if (id) {
          const idStr = id.toString();
          setLobbyId(idStr);
          setStatus('connected');
          setMessage('');
          
          // 通知父组件连接状态
          if (onConnectionChange) {
            onConnectionChange(idStr);
          }
        } else if (connectionState?.lobbyId && connectionState.type === 'client') {
          // 如果后端没有连接但全局状态有，可能是状态不同步，尝试恢复UI
          setLobbyId(connectionState.lobbyId);
          setStatus('connected');
          setMessage('');
        }
      } catch (e) {
        console.error('Failed to check connection:', e);
      }
    };
    restoreState();
  }, [connectionState, onConnectionChange]);

  const handleJoin = async () => {
    if (!lobbyId) return;
    setStatus('connecting');
    setMessage('正在加入房间...');
    try {
      await invoke('join_lobby', { 
        lobbyIdStr: lobbyId,
        password: password.trim() || null
      });
      setStatus('connected');
      setMessage(''); // 清空消息，使用专门的连接成功提示
      // 通知父组件连接状态变化
      if (onConnectionChange) {
        onConnectionChange(lobbyId);
      }
    } catch (e) {
      setStatus('error');
      setMessage(`连接失败: ${e}`);
      if (onConnectionChange) {
        onConnectionChange(null);
      }
    }
  };

  const handleDiscover = async () => {
    setDiscoveryStatus('scanning');
    setLanServer(null);
    try {
      const server = await invoke<LanServer | null>('detect_minecraft_server');
      if (server) {
        setLanServer(server);
        setDiscoveryStatus('found');
      } else {
        setDiscoveryStatus('not_found');
      }
    } catch (e) {
      console.error('Failed to discover LAN server:', e);
      setDiscoveryStatus('not_found');
    }
  };

  // Poll performance metrics every 2 seconds when connected
  useEffect(() => {
    if (status !== 'connected') return;
    
    // 立即获取一次性能指标
    const fetchMetrics = async () => {
      try {
        const data = await invoke<PerformanceMetrics>('get_performance_metrics');
        setMetrics(data);
      } catch (e) {
        console.error('Failed to get metrics:', e);
      }
    };
    fetchMetrics();
    
    const interval = setInterval(fetchMetrics, 2000);

    return () => clearInterval(interval);
  }, [status]);

  return (
    <div className="flex flex-col items-center justify-center min-h-full w-full py-8 animate-fade-in">
      <Card className="w-full max-w-lg space-y-8">
        {/* Header */}
        <div className="flex items-center justify-between">
          <button 
            onClick={onBack} 
            className="
              p-2 rounded-2xl 
              bg-white/5 hover:bg-white/10 
              backdrop-blur-sm border border-white/10
              text-white/70 hover:text-white 
              transition-all duration-200
              active:scale-95
            "
          >
            <ArrowLeft size={20} />
          </button>
          <h2 className="text-3xl font-bold text-white/95 tracking-tight">加入房间</h2>
          <div className="w-10" /> {/* Spacer for centering */}
        </div>

        <div className="space-y-6">
          {/* Lobby ID Input */}
          {status !== 'connected' && (
            <>
              {/* LAN Discovery Section */}
              <div className="space-y-4">
                <Button
                  onClick={handleDiscover}
                  disabled={discoveryStatus === 'scanning'}
                  variant="secondary"
                  className="w-full"
                >
                  {discoveryStatus === 'scanning' ? (
                    <>
                      <Loader2 size={20} className="animate-spin" />
                      <span>正在扫描...</span>
                    </>
                  ) : (
                    <>
                      <Radio size={20} />
                      <span>扫描局域网服务器</span>
                    </>
                  )}
                </Button>

                {discoveryStatus === 'found' && lanServer && (
                  <div className="bg-white/10 p-4 rounded-2xl animate-fade-in border border-white/20">
                    <div className="flex items-center space-x-4">
                      <div className="p-2 bg-green-500/20 rounded-lg">
                        <Server size={22} className="text-green-400" />
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="font-bold text-white/90 truncate">{lanServer.motd}</p>
                        <p className="text-sm text-white/70">{lanServer.ip}:{lanServer.port}</p>
                      </div>
                      <div className="text-right">
                        <div className="flex items-center space-x-1 text-green-400">
                          <Wifi size={14} />
                          <span className="font-medium text-sm">{lanServer.latency_ms.toFixed(0)} ms</span>
                        </div>
                      </div>
                    </div>
                  </div>
                )}

                {discoveryStatus === 'not_found' && (
                  <div className="bg-red-500/10 p-4 rounded-2xl flex items-center space-x-3 text-red-300 animate-fade-in border border-red-500/20">
                    <XCircle size={20} />
                    <p className="text-sm font-medium">未找到局域网服务器</p>
                  </div>
                )}
              </div>
              
              <div className="relative flex items-center justify-center">
                <div className="absolute inset-0 flex items-center" aria-hidden="true">
                  <div className="w-full border-t border-white/10"></div>
                </div>
                <div className="relative flex justify-center">
                  <span className="bg-white/5 px-3 text-sm font-medium text-white/50 backdrop-blur-sm rounded-full">或</span>
                </div>
              </div>

              <div>
                <label className="block text-sm font-semibold text-white/70 mb-3 tracking-wide">
                  房间号 (Lobby ID)
                </label>
                <input
                  type="text"
                  value={lobbyId}
                  onChange={(e) => setLobbyId(e.target.value)}
                  className="
                    w-full 
                    bg-white/5 backdrop-blur-xl
                    border border-white/20 
                    rounded-2xl px-5 py-4 
                    text-white text-lg font-medium
                    placeholder:text-white/30
                    focus:outline-none 
                    focus:ring-2 focus:ring-purple-500/50 
                    focus:border-purple-500/50
                    transition-all duration-200
                    disabled:opacity-50 disabled:cursor-not-allowed
                    shadow-lg shadow-black/10
                  "
                  placeholder="输入好友分享的房间号"
                  disabled={status === 'connecting'}
                />
              </div>
              
              <div>
                <label className="block text-sm font-semibold text-white/70 mb-3 tracking-wide">
                  房间密码 <span className="text-xs text-white/40 font-normal">(如有)</span>
                </label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  className="
                    w-full 
                    bg-white/5 backdrop-blur-xl
                    border border-white/20 
                    rounded-2xl px-5 py-4 
                    text-white text-lg font-medium
                    placeholder:text-white/30
                    focus:outline-none 
                    focus:ring-2 focus:ring-purple-500/50 
                    focus:border-purple-500/50
                    transition-all duration-200
                    disabled:opacity-50 disabled:cursor-not-allowed
                    shadow-lg shadow-black/10
                  "
                  placeholder="如果房间有密码，请输入"
                  disabled={status === 'connecting'}
                />
              </div>
            </>
          )}

          {/* Connection Status Message */}
          {message && (
            <div className={`
              backdrop-blur-xl border rounded-2xl p-5
              ${status === 'error' 
                ? 'bg-red-500/10 border-red-500/30 text-red-300' 
                : 'bg-white/5 border-white/10 text-white/80'
              }
              shadow-lg shadow-black/10
            `}>
              <p className="text-sm font-medium leading-relaxed">{message}</p>
            </div>
          )}

          {/* Connection Success Card */}
          {status === 'connected' && (
            <div className="
              relative overflow-hidden
              bg-gradient-to-br from-green-500/15 via-emerald-500/15 to-teal-500/15
              backdrop-blur-xl border border-green-500/30
              rounded-2xl p-6
              shadow-lg shadow-green-500/20
              animate-fade-in
            ">
              <div className="flex items-start space-x-4">
                <div className="p-3 rounded-2xl bg-green-500/20 border border-green-500/30 flex-shrink-0">
                  <CheckCircle size={28} className="text-green-400" />
                </div>
                <div className="flex-1 min-w-0">
                  <h3 className="text-lg font-semibold text-white/95 mb-2 flex items-center space-x-2">
                    <span>连接成功</span>
                  </h3>
                  <div className="
                    mt-4 p-4 rounded-xl
                    bg-white/5 backdrop-blur-sm
                    border border-white/10
                    space-y-3
                  ">
                    <div className="flex items-center space-x-3">
                      <div className="p-2 rounded-lg bg-purple-500/20 border border-purple-500/30">
                        <Globe size={20} className="text-purple-300" />
                      </div>
                      <div className="flex-1">
                        <p className="text-sm font-medium text-white/70 mb-1">请在 MC 中连接</p>
                        <p className="text-lg font-bold text-white/95 tracking-wide">
                          LAN world
                        </p>
                        <p className="text-xs text-white/50 mt-1">(虚拟局域网)</p>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
              
              {/* Animated background effect */}
              <div className="absolute inset-0 opacity-10">
                <div className="absolute top-0 left-0 w-full h-full bg-gradient-to-r from-transparent via-white/10 to-transparent animate-shimmer" />
              </div>
            </div>
          )}

          {/* Performance Panel */}
          {status === 'connected' && <PerformancePanel metrics={metrics} />}

          {/* Join Button */}
          {status !== 'connected' && (
            <Button 
              onClick={handleJoin} 
              disabled={status === 'connecting' || !lobbyId}
              variant="primary"
              className="w-full py-4 text-lg bg-gradient-to-br from-purple-500/90 to-purple-600/90 hover:from-purple-400/90 hover:to-purple-500/90 shadow-purple-500/30"
            >
              {status === 'connecting' ? (
                <>
                  <Loader2 size={22} className="animate-spin" />
                  <span>连接中...</span>
                </>
              ) : (
                <>
                  <LogIn size={22} />
                  <span>加入房间</span>
                </>
              )}
            </Button>
          )}
        </div>
      </Card>
    </div>
  );
};