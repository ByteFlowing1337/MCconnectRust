import React from 'react';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger';
  className?: string;
}

export const Button: React.FC<ButtonProps> = ({ 
  children, 
  variant = 'primary', 
  className = '', 
  ...props 
}) => {
  const baseStyles = `
    px-6 py-3 rounded-2xl font-semibold 
    transition-all duration-300 ease-out
    flex items-center justify-center gap-2 
    active:scale-[0.98] 
    disabled:opacity-50 disabled:cursor-not-allowed
    backdrop-blur-sm
  `;
  
  const variants = {
    primary: `
      bg-gradient-to-br from-blue-500/90 to-blue-600/90 
      hover:from-blue-400/90 hover:to-blue-500/90
      text-white 
      shadow-lg shadow-blue-500/30 
      border border-white/20
      hover:shadow-xl hover:shadow-blue-500/40
    `,
    secondary: `
      bg-white/10 hover:bg-white/15 
      text-white 
      backdrop-blur-md 
      border border-white/20
      shadow-lg shadow-black/10
      hover:border-white/30
    `,
    danger: `
      bg-gradient-to-br from-red-500/90 to-red-600/90 
      hover:from-red-400/90 hover:to-red-500/90
      text-white 
      shadow-lg shadow-red-500/30 
      border border-white/20
      hover:shadow-xl hover:shadow-red-500/40
    `,
  };

  return (
    <button 
      className={`${baseStyles} ${variants[variant]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};
