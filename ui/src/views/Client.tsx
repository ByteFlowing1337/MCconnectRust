import React, { useState, useEffect } from 'react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { PerformancePanel } from '../components/PerformancePanel';
import { ArrowLeft, LogIn, Loader2, CheckCircle, Globe } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

interface ClientProps {
  onBack: () => void;
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
}

export const Client: React.FC<ClientProps> = ({ onBack }) => {
  const [lobbyId, setLobbyId] = useState('');
  const [status, setStatus] = useState<'idle' | 'connecting' | 'connected' | 'error'>('idle');
  const [message, setMessage] = useState('');
  const [metrics, setMetrics] = useState<PerformanceMetrics | null>(null);

  const handleJoin = async () => {
    if (!lobbyId) return;
    setStatus('connecting');
    setMessage('正在加入房间...');
    try {
      await invoke('join_lobby', { lobbyIdStr: lobbyId });
      setStatus('connected');
      setMessage(''); // 清空消息，使用专门的连接成功提示
    } catch (e) {
      setStatus('error');
      setMessage(`连接失败: ${e}`);
    }
  };

  // Poll performance metrics every 2 seconds when connected
  useEffect(() => {
    if (status !== 'connected') return;
    
    const interval = setInterval(async () => {
      try {
        const data = await invoke<PerformanceMetrics>('get_performance_metrics');
        setMetrics(data);
      } catch (e) {
        console.error('Failed to get metrics:', e);
      }
    }, 2000);

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
