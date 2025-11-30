import { useState, useEffect } from 'react';
import { Home } from './views/Home';
import { Host } from './views/Host';
import { Client } from './views/Client';
import { invoke } from '@tauri-apps/api/core';

type View = 'home' | 'host' | 'client';

function App() {
  const [view, setView] = useState<View>('home');
  const [steamName, setSteamName] = useState('');

  useEffect(() => {
    invoke<string>('get_steam_name')
      .then(setSteamName)
      .catch(console.error);
  }, []);

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-800 to-black text-white font-sans selection:bg-blue-500/30">
      <div className="fixed inset-0 bg-[url('https://images.unsplash.com/photo-1614728853913-1e22ba0e982b?q=80&w=2574&auto=format&fit=crop')] bg-cover bg-center opacity-20 pointer-events-none" />
      
      <main className="relative z-10 container mx-auto px-4 py-8 h-screen flex flex-col">
        <div className="flex-1 flex flex-col justify-center">
          {view === 'home' && (
            <Home onNavigate={setView} steamName={steamName} />
          )}
          {view === 'host' && (
            <Host onBack={() => setView('home')} />
          )}
          {view === 'client' && (
            <Client onBack={() => setView('home')} />
          )}
        </div>
        
        <footer className="text-center text-white/20 text-xs py-4">
          MC Connect Rust v0.1.0
        </footer>
      </main>
    </div>
  );
}

export default App;
