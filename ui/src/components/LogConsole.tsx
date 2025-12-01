import React, { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { AnsiUp } from 'ansi_up';

const ansi_up = new AnsiUp();

interface LogEntry {
  level: number;
  message: string;
}

export const LogConsole: React.FC = () => {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [isPaused, setIsPaused] = useState(false);

  useEffect(() => {
    const setupListener = async () => {
      const unlisten = await listen<LogEntry>('log://log', (event) => {
        if (!isPaused) {
          setLogs(prevLogs => [...prevLogs, event.payload]);
        }
      });
      return unlisten;
    };

    const unlistenPromise = setupListener();

    return () => {
      unlistenPromise.then(unlisten => unlisten());
    };
  }, [isPaused]);

  useEffect(() => {
    if (!isPaused && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs, isPaused]);
  
  const getLevelClass = (level: number) => {
    switch (level) {
      case 1: return 'text-red-400'; // ERROR
      case 2: return 'text-yellow-400'; // WARN
      case 3: return 'text-sky-400'; // INFO
      case 4: return 'text-purple-400'; // DEBUG
      case 5: return 'text-gray-400'; // TRACE
      default: return 'text-gray-500';
    }
  };

  const getLevelLabel = (level: number) => {
    switch (level) {
      case 1: return 'ERROR';
      case 2: return 'WARN ';
      case 3: return 'INFO ';
      case 4: return 'DEBUG';
      case 5: return 'TRACE';
      default: return 'LOG';
    }
  }

  return (
    <div className="flex flex-col h-72 w-full">
      <div className="flex-shrink-0 flex items-center justify-between bg-black/20 p-2 border-b border-white/10 rounded-t-2xl">
        <h3 className="text-sm font-semibold text-white/80 px-2">日志控制台</h3>
        <div className="flex items-center space-x-2">
            <button
                onClick={() => setIsPaused(!isPaused)}
                className={`px-3 py-1 text-xs rounded-lg ${isPaused ? 'bg-amber-500/80 text-white' : 'bg-white/10 text-white/70'} transition-all`}
            >
                {isPaused ? '暂停滚动' : '自动滚动'}
            </button>
            <button
                onClick={() => setLogs([])}
                className="px-3 py-1 text-xs rounded-lg bg-red-500/20 text-red-300 hover:bg-red-500/30 transition-all"
            >
                清空
            </button>
        </div>
      </div>
      <div
        ref={scrollRef}
        className="flex-grow p-4 bg-black/30 backdrop-blur-sm rounded-b-2xl overflow-y-auto font-mono text-xs leading-relaxed"
      >
        {logs.map((log, index) => (
          <div key={index} className="flex items-start">
            <span className={`w-14 flex-shrink-0 font-bold ${getLevelClass(log.level)}`}>
              [{getLevelLabel(log.level)}]
            </span>
            <span className="text-white/80" dangerouslySetInnerHTML={{ __html: ansi_up.ansi_to_html(log.message) }} />
          </div>
        ))}
        {logs.length === 0 && (
            <div className="flex items-center justify-center h-full text-white/40">
                等待日志消息...
            </div>
        )}
      </div>
    </div>
  );
};
