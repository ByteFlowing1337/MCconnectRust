import React from 'react';

interface CardProps {
  children: React.ReactNode;
  className?: string;
  title?: string;
}

export const Card: React.FC<CardProps> = ({ children, className = '', title }) => {
  return (
    <div className={`
      bg-white/5 backdrop-blur-2xl 
      border border-white/20 
      rounded-3xl p-8 
      shadow-2xl shadow-black/20
      ring-1 ring-white/10
      ${className}
    `}>
      {title && (
        <h3 className="text-xl font-semibold text-white/95 mb-6 tracking-tight">{title}</h3>
      )}
      {children}
    </div>
  );
};
