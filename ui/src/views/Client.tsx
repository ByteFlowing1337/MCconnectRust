import React, { useState, useEffect } from 'react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { PerformancePanel } from '../components/PerformancePanel';
import { ArrowLeft, LogIn, Loader2 } from 'lucide-react';
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
      setMessage('已连接! 请在 MC 中连接 127.0.0.1:55555');
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
    <div className="flex flex-col items-center justify-center h-full animate-fade-in">
      <Card className="w-full max-w-md space-y-6">
        <div className="flex items-center space-x-4 mb-6">
          <button onClick={onBack} className="text-white/50 hover:text-white transition-colors">
            <ArrowLeft size={24} />
          </button>
          <h2 className="text-2xl font-bold text-white">加入房间</h2>
        </div>

        <div className="space-y-4">
          <div>
            <label className="block text-sm text-white/60 mb-2">房间号 (Lobby ID)</label>
            <input
              type="text"
              value={lobbyId}
              onChange={(e) => setLobbyId(e.target.value)}
              className="w-full bg-white/5 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-purple-500 transition-colors"
              placeholder="输入好友分享的房间号"
            />
          </div>

          <div className="bg-black/20 rounded-lg p-4 min-h-[100px] text-sm text-white/80 font-mono">
            {message || '请输入房间号并点击加入...'}
          </div>

          {status === 'connected' && <PerformancePanel metrics={metrics} />}

          <Button 
            onClick={handleJoin} 
            disabled={status === 'connecting' || status === 'connected'}
            variant="primary"
            className="w-full bg-purple-600 hover:bg-purple-500 shadow-purple-500/30"
          >
            {status === 'connecting' ? (
              <>
                <Loader2 size={20} className="animate-spin" />
                <span>连接中...</span>
              </>
            ) : (
              <>
                <LogIn size={20} />
                <span>加入房间</span>
              </>
            )}
          </Button>
        </div>
      </Card>
    </div>
  );
};
