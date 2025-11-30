import React from 'react';

interface CardProps {
  children: React.ReactNode;
  className?: string;
  title?: string;
}

export const Card: React.FC<CardProps> = ({ children, className = '', title }) => {
  return (
    <div className={`bg-black/40 backdrop-blur-xl border border-white/10 rounded-2xl p-6 shadow-xl ${className}`}>
      {title && (
        <h3 className="text-lg font-semibold text-white/90 mb-4">{title}</h3>
      )}
      {children}
    </div>
  );
};
