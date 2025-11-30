import React, { useState, useEffect } from 'react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { PerformancePanel } from '../components/PerformancePanel';
import { ArrowLeft, Play, Loader2, Copy, Check } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

interface HostProps {
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

export const Host: React.FC<HostProps> = ({ onBack }) => {
  const [port, setPort] = useState('25565');
  const [status, setStatus] = useState<'idle' | 'running' | 'error'>('idle');
  const [message, setMessage] = useState('');
  const [lobbyId, setLobbyId] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [metrics, setMetrics] = useState<PerformanceMetrics | null>(null);

  const handleStart = async () => {
    setStatus('running');
    setMessage('正在启动主机...');
    try {
      await invoke('start_host', { port: parseInt(port) });
      setMessage('主机运行中... 正在获取房间号...');
      
      // Wait a bit for lobby creation, then fetch lobby ID once
      setTimeout(async () => {
        try {
          const id = await invoke<number | null>('get_lobby_id');
          if (id) {
            setLobbyId(id.toString());
            setMessage('主机运行中... 请将房间号分享给好友');
          }
        } catch (e) {
          console.error('Failed to get lobby ID:', e);
        }
      }, 1000);
    } catch (e) {
      setStatus('error');
      setMessage(`启动失败: ${e}`);
    }
  };

  const copyLobbyId = () => {
    if (lobbyId) {
      navigator.clipboard.writeText(lobbyId);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  // Poll performance metrics every 2 seconds when running
  useEffect(() => {
    if (status !== 'running') return;
    
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
          <h2 className="text-2xl font-bold text-white">创建房间</h2>
        </div>

        <div className="space-y-4">
          <div>
            <label className="block text-sm text-white/60 mb-2">本地 MC 服务器端口</label>
            <input
              type="text"
              value={port}
              onChange={(e) => setPort(e.target.value)}
              className="w-full bg-white/5 border border-white/10 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-blue-500 transition-colors"
              placeholder="25565"
            />
          </div>

          {lobbyId && (
            <div className="bg-gradient-to-r from-blue-500/10 to-purple-500/10 border border-blue-500/30 rounded-lg p-4">
              <label className="block text-xs text-blue-300 mb-2">房间号 (Lobby ID)</label>
              <div className="flex items-center space-x-2">
                <code className="flex-1 bg-black/30 px-3 py-2 rounded text-white font-mono text-lg">
                  {lobbyId}
                </code>
                <button
                  onClick={copyLobbyId}
                  className="px-3 py-2 bg-blue-500/20 hover:bg-blue-500/30 border border-blue-500/50 rounded transition-colors"
                  title="复制房间号"
                >
                  {copied ? <Check size={20} className="text-green-400" /> : <Copy size={20} className="text-blue-400" />}
                </button>
              </div>
              <p className="text-xs text-white/50 mt-2">分享此房间号给好友以便他们加入</p>
            </div>
          )}

          <div className="bg-black/20 rounded-lg p-4 min-h-[100px] text-sm text-white/80 font-mono">
            {message || '准备就绪，点击开始创建房间...'}
          </div>

          {status === 'running' && <PerformancePanel metrics={metrics} />}

          <Button 
            onClick={handleStart} 
            disabled={status === 'running'}
            className="w-full"
          >
            {status === 'running' ? (
              <>
                <Loader2 size={20} className="animate-spin" />
                <span>运行中</span>
              </>
            ) : (
              <>
                <Play size={20} />
                <span>启动主机</span>
              </>
            )}
          </Button>
        </div>
      </Card>
    </div>
  );
};
