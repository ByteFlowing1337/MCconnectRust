import React from 'react';
import { Activity, ArrowUp, ArrowDown, AlertTriangle } from 'lucide-react';

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

interface PerformancePanelProps {
  metrics: PerformanceMetrics | null;
}

export const PerformancePanel: React.FC<PerformancePanelProps> = ({ metrics }) => {
  if (!metrics) {
    return (
      <div className="
        bg-white/5 backdrop-blur-xl
        border border-white/20 
        rounded-2xl p-5
        shadow-lg shadow-black/10
      ">
        <div className="flex items-center space-x-2 text-white/50">
          <Activity size={18} className="animate-pulse text-blue-400" />
          <span className="text-sm font-medium">等待数据...</span>
        </div>
      </div>
    );
  }

  const formatBytes = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  };

  return (
    <div className="
      bg-white/5 backdrop-blur-xl
      border border-white/20 
      rounded-2xl p-6
      shadow-lg shadow-black/10
      space-y-4
    ">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center space-x-2">
          <div className="p-2 rounded-xl bg-blue-500/20 border border-blue-500/30">
            <Activity size={18} className="text-blue-400" />
          </div>
          <span className="text-base font-semibold text-white/95">性能指标</span>
        </div>
        <div className="flex items-center space-x-2">
          {metrics.latency_ms !== null && metrics.latency_ms !== undefined && (
            <div className="flex items-center space-x-1.5 px-3 py-1.5 rounded-xl bg-purple-500/20 border border-purple-500/30">
              <span className="text-xs font-medium text-purple-300">
                延迟: {metrics.latency_ms}ms
              </span>
            </div>
          )}
          {metrics.packets_dropped > 0 && (
            <div className="flex items-center space-x-2 px-3 py-1.5 rounded-xl bg-yellow-500/20 border border-yellow-500/30">
              <AlertTriangle size={14} className="text-yellow-400" />
              <span className="text-xs font-medium text-yellow-300">丢包: {metrics.packets_dropped}</span>
            </div>
          )}
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        {/* Upload */}
        <div className="
          bg-white/5 backdrop-blur-sm
          border border-white/10
          rounded-xl p-4 
          space-y-2
          shadow-inner
        ">
          <div className="flex items-center space-x-2 text-green-400">
            <div className="p-1.5 rounded-lg bg-green-500/20">
              <ArrowUp size={14} />
            </div>
            <span className="text-xs font-semibold">发送</span>
          </div>
          <div className="space-y-1.5">
            <div className="text-white text-base font-mono font-semibold">
              {metrics.send_rate_mbps.toFixed(2)} <span className="text-xs text-white/60 font-normal">MB/s</span>
            </div>
            <div className="text-white/70 text-xs font-mono">
              {metrics.send_rate_pps.toFixed(0)} <span className="text-white/50">pkt/s</span>
            </div>
            <div className="text-white/50 text-xs pt-1">
              总计: {formatBytes(metrics.bytes_sent)}
            </div>
          </div>
        </div>

        {/* Download */}
        <div className="
          bg-white/5 backdrop-blur-sm
          border border-white/10
          rounded-xl p-4 
          space-y-2
          shadow-inner
        ">
          <div className="flex items-center space-x-2 text-blue-400">
            <div className="p-1.5 rounded-lg bg-blue-500/20">
              <ArrowDown size={14} />
            </div>
            <span className="text-xs font-semibold">接收</span>
          </div>
          <div className="space-y-1.5">
            <div className="text-white text-base font-mono font-semibold">
              {metrics.recv_rate_mbps.toFixed(2)} <span className="text-xs text-white/60 font-normal">MB/s</span>
            </div>
            <div className="text-white/70 text-xs font-mono">
              {metrics.recv_rate_pps.toFixed(0)} <span className="text-white/50">pkt/s</span>
            </div>
            <div className="text-white/50 text-xs pt-1">
              总计: {formatBytes(metrics.bytes_received)}
            </div>
          </div>
        </div>
      </div>

      {/* Progress bars */}
      <div className="space-y-3 pt-3 border-t border-white/10">
        <div>
          <div className="flex justify-between text-xs font-medium text-white/70 mb-2">
            <span>上传</span>
            <span className="font-mono">{metrics.packets_sent} 包</span>
          </div>
          <div className="h-2 bg-white/5 rounded-full overflow-hidden shadow-inner">
            <div
              className="h-full bg-gradient-to-r from-green-500/80 to-green-400/80 transition-all duration-300 rounded-full shadow-lg shadow-green-500/30"
              style={{ width: `${Math.min((metrics.send_rate_mbps / 10) * 100, 100)}%` }}
            />
          </div>
        </div>
        <div>
          <div className="flex justify-between text-xs font-medium text-white/70 mb-2">
            <span>下载</span>
            <span className="font-mono">{metrics.packets_received} 包</span>
          </div>
          <div className="h-2 bg-white/5 rounded-full overflow-hidden shadow-inner">
            <div
              className="h-full bg-gradient-to-r from-blue-500/80 to-blue-400/80 transition-all duration-300 rounded-full shadow-lg shadow-blue-500/30"
              style={{ width: `${Math.min((metrics.recv_rate_mbps / 10) * 100, 100)}%` }}
            />
          </div>
        </div>
      </div>
    </div>
  );
};
