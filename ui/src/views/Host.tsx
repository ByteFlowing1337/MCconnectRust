import React, { useState, useEffect, useRef } from 'react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { PerformancePanel } from '../components/PerformancePanel';
import { ArrowLeft, Play, Loader2, Copy, Check, Radio, Sparkles } from 'lucide-react';
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

interface MinecraftServerInfo {
  port: number;
  motd: string;
}

export const Host: React.FC<HostProps> = ({ onBack }) => {
  const [port, setPort] = useState('25565');
  const [status, setStatus] = useState<'idle' | 'running' | 'error'>('idle');
  const [message, setMessage] = useState('正在自动检测 Minecraft 服务器...');
  const [lobbyId, setLobbyId] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [metrics, setMetrics] = useState<PerformanceMetrics | null>(null);
  const [detecting, setDetecting] = useState(true);
  const [detectedServer, setDetectedServer] = useState<MinecraftServerInfo | null>(null);
  const [detectionAttempts, setDetectionAttempts] = useState(0);
  const detectionIntervalRef = useRef<NodeJS.Timeout | null>(null);

  // 自动轮询检测 Minecraft 服务器
  useEffect(() => {
    // 如果已经检测到服务器或正在运行，不进行检测
    if (detectedServer || status === 'running') {
      return;
    }

    const performDetection = async () => {
      setDetecting(true);
      setDetectionAttempts(prev => {
        const newAttempts = prev + 1;
        // 更新消息显示当前尝试次数
        setMessage(`正在搜索本地 Minecraft 服务器... (尝试 ${newAttempts})`);
        return newAttempts;
      });
      
      try {
        const server = await invoke<MinecraftServerInfo | null>('detect_minecraft_server');
        
        if (server) {
          // 检测到服务器，自动停止轮询
          setPort(server.port.toString());
          setDetectedServer(server);
          setDetecting(false);
          setMessage(`✓ 已自动检测到服务器: ${server.motd}`);
          
          // 清除轮询
          if (detectionIntervalRef.current) {
            clearInterval(detectionIntervalRef.current);
            detectionIntervalRef.current = null;
          }
        }
      } catch (e) {
        console.error('检测失败:', e);
        setMessage(`检测失败，继续尝试...`);
      }
      // 注意：这里不设置 setDetecting(false)，因为要继续轮询直到检测到服务器
    };

    // 立即执行一次检测
    performDetection();

    // 每 3 秒轮询一次（与后端超时时间匹配）
    detectionIntervalRef.current = setInterval(performDetection, 3000);

    // 清理函数
    return () => {
      if (detectionIntervalRef.current) {
        clearInterval(detectionIntervalRef.current);
        detectionIntervalRef.current = null;
      }
    };
  }, [detectedServer, status]); // 当检测到服务器或状态改变时重新运行

  const handleStart = async () => {
    // 停止检测
    if (detectionIntervalRef.current) {
      clearInterval(detectionIntervalRef.current);
      detectionIntervalRef.current = null;
    }
    setDetecting(false);
    
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
          <h2 className="text-3xl font-bold text-white/95 tracking-tight">创建房间</h2>
          <div className="w-10" /> {/* Spacer for centering */}
        </div>

        <div className="space-y-6">
          {/* Server Detection Status */}
          <div className="
            relative overflow-hidden
            bg-gradient-to-br from-blue-500/10 via-purple-500/10 to-pink-500/10
            backdrop-blur-xl border border-white/20
            rounded-2xl p-6
            shadow-lg shadow-blue-500/10
          ">
            <div className="flex items-start space-x-4">
              <div className={`
                p-3 rounded-2xl
                ${detecting 
                  ? 'bg-blue-500/20 animate-pulse' 
                  : detectedServer 
                    ? 'bg-green-500/20' 
                    : 'bg-white/10'
                }
                transition-all duration-300
              `}>
                {detecting ? (
                  <Loader2 size={24} className="text-blue-400 animate-spin" />
                ) : detectedServer ? (
                  <Check size={24} className="text-green-400" />
                ) : (
                  <Radio size={24} className="text-white/60" />
                )}
              </div>
              <div className="flex-1 min-w-0">
                <h3 className="text-sm font-semibold text-white/60 mb-1">服务器检测</h3>
                <p className="text-base text-white/90 font-medium leading-relaxed">
                  {detecting 
                    ? `正在搜索本地 Minecraft 服务器... (尝试 ${detectionAttempts})`
                    : detectedServer 
                      ? `已检测到: ${detectedServer.motd}`
                      : '未检测到服务器'
                  }
                </p>
                {detectedServer && (
                  <p className="text-xs text-white/50 mt-2">
                    端口: <span className="font-mono font-semibold text-white/70">{detectedServer.port}</span>
                  </p>
                )}
              </div>
            </div>
            
            {/* Animated background effect */}
            {detecting && (
              <div className="absolute inset-0 opacity-20">
                <div className="absolute top-0 left-0 w-full h-full bg-gradient-to-r from-transparent via-white/10 to-transparent animate-shimmer" />
              </div>
            )}
          </div>

          {/* Port Input */}
          <div>
            <label className="block text-sm font-semibold text-white/70 mb-3 tracking-wide">
              本地 MC 服务器端口
            </label>
            <div className="relative">
              <input
                type="text"
                value={port}
                onChange={(e) => setPort(e.target.value)}
                className="
                  w-full 
                  bg-white/5 backdrop-blur-xl
                  border border-white/20 
                  rounded-2xl px-5 py-4 
                  text-white text-lg font-medium
                  placeholder:text-white/30
                  focus:outline-none 
                  focus:ring-2 focus:ring-blue-500/50 
                  focus:border-blue-500/50
                  transition-all duration-200
                  disabled:opacity-50 disabled:cursor-not-allowed
                  shadow-lg shadow-black/10
                "
                placeholder="25565"
                disabled={status === 'running'}
              />
              {detectedServer && (
                <div className="absolute right-4 top-1/2 -translate-y-1/2">
                  <div className="flex items-center space-x-2 px-3 py-1.5 rounded-xl bg-green-500/20 border border-green-500/30">
                    <Check size={16} className="text-green-400" />
                    <span className="text-xs font-medium text-green-300">已检测</span>
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Lobby ID Display */}
          {lobbyId && (
            <div className="
              bg-gradient-to-br from-blue-500/15 via-purple-500/15 to-pink-500/15
              backdrop-blur-xl border border-white/20
              rounded-2xl p-6
              shadow-lg shadow-purple-500/10
            ">
              <div className="flex items-center space-x-2 mb-3">
                <Sparkles size={18} className="text-purple-400" />
                <label className="text-sm font-semibold text-purple-300">房间号 (Lobby ID)</label>
              </div>
              <div className="flex items-center space-x-3">
                <code className="
                  flex-1 
                  bg-black/30 backdrop-blur-sm
                  border border-white/10
                  px-4 py-3 rounded-xl 
                  text-white font-mono text-xl font-semibold
                  tracking-wider
                  shadow-inner
                ">
                  {lobbyId}
                </code>
                <button
                  onClick={copyLobbyId}
                  className="
                    p-3 rounded-xl
                    bg-white/10 hover:bg-white/15
                    backdrop-blur-sm border border-white/20
                    transition-all duration-200
                    active:scale-95
                    shadow-lg shadow-black/10
                  "
                  title="复制房间号"
                >
                  {copied ? (
                    <Check size={20} className="text-green-400" />
                  ) : (
                    <Copy size={20} className="text-white/70" />
                  )}
                </button>
              </div>
              <p className="text-xs text-white/50 mt-3 leading-relaxed">
                分享此房间号给好友以便他们加入
              </p>
            </div>
          )}

          {/* Status Message */}
          <div className="
            bg-white/5 backdrop-blur-xl
            border border-white/10
            rounded-2xl p-5
            min-h-[80px] 
            text-sm text-white/80 font-medium
            leading-relaxed
            shadow-inner
          ">
            {message || '准备就绪，点击开始创建房间...'}
          </div>

          {/* Performance Panel */}
          {status === 'running' && <PerformancePanel metrics={metrics} />}

          {/* Start Button */}
          <Button 
            onClick={handleStart} 
            disabled={status === 'running' || !port}
            className="w-full py-4 text-lg"
          >
            {status === 'running' ? (
              <>
                <Loader2 size={22} className="animate-spin" />
                <span>运行中</span>
              </>
            ) : (
              <>
                <Play size={22} />
                <span>启动主机</span>
              </>
            )}
          </Button>
        </div>
      </Card>
    </div>
  );
};
