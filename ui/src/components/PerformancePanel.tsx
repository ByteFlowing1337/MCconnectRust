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
}

interface PerformancePanelProps {
  metrics: PerformanceMetrics | null;
}

export const PerformancePanel: React.FC<PerformancePanelProps> = ({ metrics }) => {
  if (!metrics) {
    return (
      <div className="bg-gradient-to-br from-gray-800/50 to-gray-900/50 backdrop-blur-sm border border-white/10 rounded-lg p-4">
        <div className="flex items-center space-x-2 text-white/50">
          <Activity size={16} className="animate-pulse" />
          <span className="text-sm">等待数据...</span>
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
    <div className="bg-gradient-to-br from-gray-800/50 to-gray-900/50 backdrop-blur-sm border border-white/10 rounded-lg p-4 space-y-3">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center space-x-2">
          <Activity size={16} className="text-blue-400" />
          <span className="text-sm font-semibold text-white">性能指标</span>
        </div>
        {metrics.packets_dropped > 0 && (
          <div className="flex items-center space-x-1 text-yellow-400">
            <AlertTriangle size={14} />
            <span className="text-xs">丢包: {metrics.packets_dropped}</span>
          </div>
        )}
      </div>

      <div className="grid grid-cols-2 gap-3">
        {/* Upload */}
        <div className="bg-black/20 rounded-lg p-3 space-y-2">
          <div className="flex items-center space-x-2 text-green-400">
            <ArrowUp size={14} />
            <span className="text-xs font-medium">发送</span>
          </div>
          <div className="space-y-1">
            <div className="text-white text-sm font-mono">
              {metrics.send_rate_mbps.toFixed(2)} <span className="text-xs text-white/60">MB/s</span>
            </div>
            <div className="text-white/70 text-xs font-mono">
              {metrics.send_rate_pps.toFixed(0)} <span className="text-white/50">pkt/s</span>
            </div>
            <div className="text-white/50 text-xs">
              总计: {formatBytes(metrics.bytes_sent)}
            </div>
          </div>
        </div>

        {/* Download */}
        <div className="bg-black/20 rounded-lg p-3 space-y-2">
          <div className="flex items-center space-x-2 text-blue-400">
            <ArrowDown size={14} />
            <span className="text-xs font-medium">接收</span>
          </div>
          <div className="space-y-1">
            <div className="text-white text-sm font-mono">
              {metrics.recv_rate_mbps.toFixed(2)} <span className="text-xs text-white/60">MB/s</span>
            </div>
            <div className="text-white/70 text-xs font-mono">
              {metrics.recv_rate_pps.toFixed(0)} <span className="text-white/50">pkt/s</span>
            </div>
            <div className="text-white/50 text-xs">
              总计: {formatBytes(metrics.bytes_received)}
            </div>
          </div>
        </div>
      </div>

      {/* Progress bars */}
      <div className="space-y-2 pt-2 border-t border-white/5">
        <div>
          <div className="flex justify-between text-xs text-white/60 mb-1">
            <span>上传</span>
            <span>{metrics.packets_sent} 包</span>
          </div>
          <div className="h-1.5 bg-black/20 rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-green-500 to-green-400 transition-all duration-300"
              style={{ width: `${Math.min((metrics.send_rate_mbps / 10) * 100, 100)}%` }}
            />
          </div>
        </div>
        <div>
          <div className="flex justify-between text-xs text-white/60 mb-1">
            <span>下载</span>
            <span>{metrics.packets_received} 包</span>
          </div>
          <div className="h-1.5 bg-black/20 rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-blue-500 to-blue-400 transition-all duration-300"
              style={{ width: `${Math.min((metrics.recv_rate_mbps / 10) * 100, 100)}%` }}
            />
          </div>
        </div>
      </div>
    </div>
  );
};
