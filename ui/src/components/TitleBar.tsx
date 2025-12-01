import React, { useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { X, Minus, Maximize2 } from 'lucide-react';

export const TitleBar: React.FC = () => {
  const [isHovered, setIsHovered] = useState(false);
  const appWindow = getCurrentWindow();

  const handleMinimize = () => appWindow.minimize();
  const handleMaximize = async () => {
    const isMaximized = await appWindow.isMaximized();
    if (isMaximized) {
      appWindow.unmaximize();
    } else {
      appWindow.maximize();
    }
  };
  const handleClose = () => appWindow.close();

  return (
    <div 
      data-tauri-drag-region 
      className="h-10 flex items-center justify-between px-4 select-none fixed top-0 left-0 right-0 z-50 bg-transparent cursor-move"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <div className="flex items-center space-x-2 group cursor-default">
        {/* Close Button */}
        <button
          onClick={handleClose}
          className="w-3 h-3 rounded-full bg-[#FF5F56] border border-[#E0443E] flex items-center justify-center hover:bg-[#FF5F56]/80 transition-colors focus:outline-none cursor-pointer"
        >
          <X size={8} className={`text-black/60 ${isHovered ? 'opacity-100' : 'opacity-0'} transition-opacity`} strokeWidth={3} />
        </button>

        {/* Minimize Button */}
        <button
          onClick={handleMinimize}
          className="w-3 h-3 rounded-full bg-[#FFBD2E] border border-[#DEA123] flex items-center justify-center hover:bg-[#FFBD2E]/80 transition-colors focus:outline-none cursor-pointer"
        >
          <Minus size={8} className={`text-black/60 ${isHovered ? 'opacity-100' : 'opacity-0'} transition-opacity`} strokeWidth={3} />
        </button>

        {/* Maximize Button */}
        <button
          onClick={handleMaximize}
          className="w-3 h-3 rounded-full bg-[#27C93F] border border-[#1AAB29] flex items-center justify-center hover:bg-[#27C93F]/80 transition-colors focus:outline-none cursor-pointer"
        >
          <Maximize2 size={8} className={`text-black/60 ${isHovered ? 'opacity-100' : 'opacity-0'} transition-opacity`} strokeWidth={3} />
        </button>
      </div>
      
      {/* Drag Region - Takes up most of the title bar */}
      <div className="flex-1 h-full cursor-move" data-tauri-drag-region />
    </div>
  );
};
