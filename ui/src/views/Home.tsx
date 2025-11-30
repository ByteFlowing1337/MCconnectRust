import React from 'react';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { Server, Users } from 'lucide-react';

interface HomeProps {
  onNavigate: (view: 'host' | 'client') => void;
  steamName: string;
}

export const Home: React.FC<HomeProps> = ({ onNavigate, steamName }) => {
  return (
    <div className="flex flex-col items-center justify-center h-full space-y-8 animate-fade-in">
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
    </div>
  );
};
