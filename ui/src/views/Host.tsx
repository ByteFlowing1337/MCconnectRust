import React, { useState } from 'react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { ArrowLeft, Play, Loader2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

interface HostProps {
  onBack: () => void;
}

export const Host: React.FC<HostProps> = ({ onBack }) => {
  const [port, setPort] = useState('25565');
  const [status, setStatus] = useState<'idle' | 'running' | 'error'>('idle');
  const [message, setMessage] = useState('');

  const handleStart = async () => {
    setStatus('running');
    setMessage('正在启动主机...');
    try {
      await invoke('start_host', { port: parseInt(port) });
      setMessage('主机运行中... 请将房间号分享给好友');
    } catch (e) {
      setStatus('error');
      setMessage(`启动失败: ${e}`);
    }
  };

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

          <div className="bg-black/20 rounded-lg p-4 min-h-[100px] text-sm text-white/80 font-mono">
            {message || '准备就绪，点击开始创建房间...'}
          </div>

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
