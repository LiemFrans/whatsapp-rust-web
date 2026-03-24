import { useNavigate } from 'react-router-dom';
import { MessageCircle, Building2, ArrowRight, Shield, Users, Zap } from 'lucide-react';
import { useAuthStore } from '@/store/authStore';

export default function ModeSelect() {
  const navigate = useNavigate();
  const { user, setMode } = useAuthStore();

  const selectMode = (mode: 'personal' | 'business') => {
    setMode(mode);
    navigate(mode === 'personal' ? '/personal' : '/business');
  };

  return (
    <div className="flex h-full items-center justify-center bg-gradient-to-br from-wa-green/5 via-white to-wa-teal/5 dark:from-gray-900 dark:via-gray-900 dark:to-gray-800">
      <div className="absolute left-0 right-0 top-0 h-56 bg-wa-green dark:bg-wa-dark-header" />

      <div className="relative z-10 w-full max-w-3xl px-4">
        <div className="mb-8 text-center">
          <h1 className="text-3xl font-bold text-white">Welcome, {user?.username}!</h1>
          <p className="mt-2 text-white/80">Choose how you want to use WhatsApp Web</p>
        </div>

        <div className="grid grid-cols-1 gap-6 md:grid-cols-2">
          {/* Personal Mode */}
          <button
            onClick={() => selectMode('personal')}
            className="group rounded-2xl bg-white p-8 text-left shadow-xl transition-all hover:-translate-y-1 hover:shadow-2xl dark:bg-gray-800"
          >
            <div className="mb-6 flex h-16 w-16 items-center justify-center rounded-2xl bg-wa-green/10 transition-colors group-hover:bg-wa-green/20">
              <MessageCircle size={32} className="text-wa-green" />
            </div>

            <h2 className="mb-2 text-xl font-bold text-gray-800 dark:text-white">Personal Mode</h2>
            <p className="mb-6 text-sm text-gray-500 dark:text-gray-400">
              Use WhatsApp Web for personal messaging. Connect your phone and chat with friends and family.
            </p>

            <ul className="mb-6 space-y-2">
              <li className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
                <Shield size={16} className="shrink-0 text-wa-green" />
                End-to-end encrypted messaging
              </li>
              <li className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
                <MessageCircle size={16} className="shrink-0 text-wa-green" />
                Send text, images, videos, documents
              </li>
              <li className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
                <Zap size={16} className="shrink-0 text-wa-green" />
                Real-time message sync
              </li>
            </ul>

            <div className="flex items-center gap-2 font-semibold text-wa-green transition-transform group-hover:translate-x-1">
              Get Started <ArrowRight size={18} />
            </div>
          </button>

          {/* Business Mode */}
          <button
            onClick={() => selectMode('business')}
            className="group rounded-2xl bg-white p-8 text-left shadow-xl transition-all hover:-translate-y-1 hover:shadow-2xl dark:bg-gray-800"
          >
            <div className="mb-6 flex h-16 w-16 items-center justify-center rounded-2xl bg-wa-teal/10 transition-colors group-hover:bg-wa-teal/20">
              <Building2 size={32} className="text-wa-teal" />
            </div>

            <h2 className="mb-2 text-xl font-bold text-gray-800 dark:text-white">Business Mode</h2>
            <p className="mb-6 text-sm text-gray-500 dark:text-gray-400">
              Manage customer conversations with your team. Multi-session support with ticket management.
            </p>

            <ul className="mb-6 space-y-2">
              <li className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
                <Users size={16} className="shrink-0 text-wa-teal" />
                Multi-agent assignment & routing
              </li>
              <li className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
                <Building2 size={16} className="shrink-0 text-wa-teal" />
                Ticket management & escalation
              </li>
              <li className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
                <Zap size={16} className="shrink-0 text-wa-teal" />
                Quick replies & analytics
              </li>
            </ul>

            <div className="flex items-center gap-2 font-semibold text-wa-teal transition-transform group-hover:translate-x-1">
              Get Started <ArrowRight size={18} />
            </div>
          </button>
        </div>

        {/* Role indicator */}
        {user && (
          <div className="mt-6 text-center text-sm text-gray-400">
            Logged in as <span className="font-medium text-gray-600 dark:text-gray-300">{user.username}</span>
            {' · '}
            <span className="rounded-full bg-gray-100 px-2 py-0.5 text-xs font-medium capitalize dark:bg-gray-700">
              {user.role}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
