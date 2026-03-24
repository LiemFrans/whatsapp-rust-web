import {
  MessageSquare,
  Users,
  Ticket,
  BarChart3,
  Zap,
  Settings,
  LogOut,
  Moon,
  Sun,
  Wifi,
  WifiOff,
  ChevronLeft,
} from 'lucide-react';
import { useAuthStore } from '@/store/authStore';
import { useState } from 'react';

export type SidebarTab =
  | 'chats'
  | 'queue'
  | 'my-chats'
  | 'tickets'
  | 'analytics'
  | 'quick-replies'
  | 'settings';

interface SidebarProps {
  activeTab: SidebarTab;
  onTabChange: (tab: SidebarTab) => void;
  mode: 'personal' | 'business';
  isConnected?: boolean;
  onBack?: () => void;
}

export default function Sidebar({
  activeTab,
  onTabChange,
  mode,
  isConnected,
  onBack,
}: SidebarProps) {
  const { user, logout } = useAuthStore();
  const [dark, setDark] = useState(document.documentElement.classList.contains('dark'));

  const toggleDark = () => {
    document.documentElement.classList.toggle('dark');
    setDark(!dark);
  };

  const personalTabs: { id: SidebarTab; icon: React.ReactNode; label: string }[] = [
    { id: 'chats', icon: <MessageSquare size={20} />, label: 'Chats' },
    { id: 'settings', icon: <Settings size={20} />, label: 'Settings' },
  ];

  const businessTabs: { id: SidebarTab; icon: React.ReactNode; label: string }[] = [
    { id: 'queue', icon: <Users size={20} />, label: 'Queue' },
    { id: 'my-chats', icon: <MessageSquare size={20} />, label: 'My Chats' },
    { id: 'tickets', icon: <Ticket size={20} />, label: 'Tickets' },
    { id: 'analytics', icon: <BarChart3 size={20} />, label: 'Analytics' },
    { id: 'quick-replies', icon: <Zap size={20} />, label: 'Quick Replies' },
    { id: 'settings', icon: <Settings size={20} />, label: 'Settings' },
  ];

  const tabs = mode === 'personal' ? personalTabs : businessTabs;

  return (
    <div className="flex h-full w-16 flex-col items-center border-r border-gray-200 bg-white py-4 dark:border-gray-700 dark:bg-wa-dark-sidebar">
      {/* Back button */}
      {onBack && (
        <button
          onClick={onBack}
          className="mb-4 flex h-10 w-10 items-center justify-center rounded-lg text-gray-500 transition-colors hover:bg-gray-100 dark:hover:bg-gray-700"
          title="Back to mode select"
        >
          <ChevronLeft size={22} />
        </button>
      )}

      {/* Connection indicator */}
      <div className="mb-4" title={isConnected ? 'Connected' : 'Disconnected'}>
        {isConnected ? (
          <Wifi size={18} className="text-wa-green" />
        ) : (
          <WifiOff size={18} className="text-red-400" />
        )}
      </div>

      {/* Tabs */}
      <div className="flex flex-1 flex-col gap-2">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => onTabChange(tab.id)}
            className={`flex h-10 w-10 items-center justify-center rounded-lg transition-colors ${
              activeTab === tab.id
                ? 'bg-wa-green/10 text-wa-green'
                : 'text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-700'
            }`}
            title={tab.label}
          >
            {tab.icon}
          </button>
        ))}
      </div>

      {/* Bottom actions */}
      <div className="flex flex-col items-center gap-2">
        <button
          onClick={toggleDark}
          className="flex h-10 w-10 items-center justify-center rounded-lg text-gray-500 transition-colors hover:bg-gray-100 dark:hover:bg-gray-700"
          title={dark ? 'Light mode' : 'Dark mode'}
        >
          {dark ? <Sun size={18} /> : <Moon size={18} />}
        </button>

        {/* User avatar */}
        <div className="flex h-10 w-10 items-center justify-center rounded-full bg-wa-green text-sm font-bold text-white">
          {user?.username?.[0]?.toUpperCase() || 'U'}
        </div>

        <button
          onClick={logout}
          className="flex h-10 w-10 items-center justify-center rounded-lg text-red-400 transition-colors hover:bg-red-50 dark:hover:bg-red-900/20"
          title="Logout"
        >
          <LogOut size={18} />
        </button>
      </div>
    </div>
  );
}
