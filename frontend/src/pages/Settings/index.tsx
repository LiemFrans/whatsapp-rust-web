import { useState, useEffect, useCallback } from 'react';
import {
  Users,
  Wifi,
  WifiOff,
  Trash2,
  Plus,
  UserPlus,
  Shield,
  ToggleLeft,
  ToggleRight,
  RefreshCw,
  X,
  Save,
  Loader2,
} from 'lucide-react';
import { useAuthStore } from '@/store/authStore';
import { useChatStore } from '@/store/chatStore';
import { usersApi } from '@/services/api';
import type { User, WhatsAppSession } from '@/types';

interface SettingsProps {
  onConnectSession: () => void;
}

export default function SettingsPanel({ onConnectSession }: SettingsProps) {
  const { user: currentUser } = useAuthStore();
  const { sessions, fetchSessions, disconnectSession, deleteSession } = useChatStore();
  const [activeSection, setActiveSection] = useState<'sessions' | 'users' | 'profile'>('sessions');
  const [users, setUsers] = useState<User[]>([]);
  const [showCreateUser, setShowCreateUser] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const isAdmin = currentUser?.role === 'admin';

  const loadUsers = useCallback(async () => {
    if (!isAdmin) return;
    try {
      const res = await usersApi.list();
      setUsers(res.data.users);
    } catch (err) {
      console.error('Failed to load users:', err);
    }
  }, [isAdmin]);

  useEffect(() => {
    fetchSessions();
    loadUsers();
  }, [fetchSessions, loadUsers]);

  const handleDisconnect = async (sessionId: string) => {
    try {
      await disconnectSession(sessionId);
      fetchSessions();
    } catch (err) {
      console.error('Failed to disconnect session:', err);
    }
  };

  const handleDelete = async (sessionId: string) => {
    if (!confirm('Delete this session? All associated chats and messages will be removed.')) return;
    try {
      await deleteSession(sessionId);
    } catch (err) {
      console.error('Failed to delete session:', err);
    }
  };

  const handleToggleActive = async (userId: string) => {
    try {
      await usersApi.toggleActive(userId);
      loadUsers();
    } catch (err) {
      console.error('Failed to toggle user active:', err);
    }
  };

  const sections = [
    { id: 'sessions' as const, label: 'WhatsApp Sessions', icon: <Wifi size={18} /> },
    ...(isAdmin ? [{ id: 'users' as const, label: 'User Management', icon: <Users size={18} /> }] : []),
    { id: 'profile' as const, label: 'My Profile', icon: <Shield size={18} /> },
  ];

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-14 items-center border-b border-gray-200 bg-wa-header px-4 dark:border-gray-700 dark:bg-wa-dark-header">
        <h2 className="text-lg font-semibold text-white">Settings</h2>
      </div>

      {/* Section Tabs */}
      <div className="flex border-b border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800">
        {sections.map((section) => (
          <button
            key={section.id}
            onClick={() => setActiveSection(section.id)}
            className={`flex items-center gap-1.5 px-3 py-2.5 text-xs font-medium transition-colors ${
              activeSection === section.id
                ? 'border-b-2 border-wa-green text-wa-green'
                : 'text-gray-500 hover:text-gray-700 dark:hover:text-gray-300'
            }`}
          >
            {section.icon}
            {section.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto">
        {activeSection === 'sessions' && (
          <SessionsSection
            sessions={sessions}
            onConnect={onConnectSession}
            onDisconnect={handleDisconnect}
            onDelete={handleDelete}
            onRefresh={fetchSessions}
          />
        )}
        {activeSection === 'users' && isAdmin && (
          <UsersSection
            users={users}
            currentUserId={currentUser?.id || ''}
            onToggleActive={handleToggleActive}
            showCreateUser={showCreateUser}
            setShowCreateUser={setShowCreateUser}
            onUserCreated={loadUsers}
            isLoading={isLoading}
            setIsLoading={setIsLoading}
          />
        )}
        {activeSection === 'profile' && <ProfileSection user={currentUser} />}
      </div>
    </div>
  );
}

// ─── Sessions Section ───────────────────────────────────────────

function SessionsSection({
  sessions,
  onConnect,
  onDisconnect,
  onDelete,
  onRefresh,
}: {
  sessions: WhatsAppSession[];
  onConnect: () => void;
  onDisconnect: (id: string) => void;
  onDelete: (id: string) => void;
  onRefresh: () => void;
}) {
  const statusColors: Record<string, string> = {
    connected: 'text-green-500',
    connecting: 'text-yellow-500',
    disconnected: 'text-red-400',
    qr_code: 'text-blue-500',
  };

  return (
    <div className="p-3">
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300">
          WhatsApp Sessions ({sessions.length})
        </h3>
        <div className="flex gap-2">
          <button
            onClick={onRefresh}
            className="rounded-lg p-1.5 text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-700"
            title="Refresh"
          >
            <RefreshCw size={14} />
          </button>
          <button
            onClick={onConnect}
            className="flex items-center gap-1 rounded-lg bg-wa-green px-3 py-1.5 text-xs font-medium text-white hover:bg-wa-green/90"
          >
            <Plus size={12} />
            New Session
          </button>
        </div>
      </div>

      {sessions.length === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-xl border border-dashed border-gray-300 py-10 dark:border-gray-600">
          <WifiOff size={36} className="mb-2 text-gray-300" />
          <p className="text-sm text-gray-500">No sessions configured</p>
          <button
            onClick={onConnect}
            className="mt-3 flex items-center gap-1 rounded-lg bg-wa-green px-4 py-2 text-xs font-medium text-white"
          >
            <Plus size={14} />
            Connect WhatsApp
          </button>
        </div>
      ) : (
        <div className="space-y-2">
          {sessions.map((session) => (
            <div
              key={session.id}
              className="flex items-center justify-between rounded-xl border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-800"
            >
              <div className="flex items-center gap-3">
                <div className={statusColors[session.status] || 'text-gray-400'}>
                  {session.status === 'connected' ? (
                    <Wifi size={18} />
                  ) : (
                    <WifiOff size={18} />
                  )}
                </div>
                <div>
                  <p className="text-sm font-medium text-gray-900 dark:text-white">
                    {session.session_name}
                  </p>
                  <p className="text-xs text-gray-500">
                    {session.phone_number || session.status}
                    {session.last_active_at && (
                      <> · Last active: {new Date(session.last_active_at).toLocaleDateString()}</>
                    )}
                  </p>
                </div>
              </div>
              {session.status !== 'disconnected' && (
                <button
                  onClick={() => onDisconnect(session.id)}
                  className="rounded-lg p-1.5 text-yellow-500 hover:bg-yellow-50 dark:hover:bg-yellow-900/20"
                  title="Disconnect"
                >
                  <WifiOff size={14} />
                </button>
              )}
              <button
                onClick={() => onDelete(session.id)}
                className="rounded-lg p-1.5 text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20"
                title="Delete session"
              >
                <Trash2 size={14} />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Users Section ──────────────────────────────────────────────

function UsersSection({
  users,
  currentUserId,
  onToggleActive,
  showCreateUser,
  setShowCreateUser,
  onUserCreated,
  isLoading,
  setIsLoading,
}: {
  users: User[];
  currentUserId: string;
  onToggleActive: (id: string) => void;
  showCreateUser: boolean;
  setShowCreateUser: (v: boolean) => void;
  onUserCreated: () => void;
  isLoading: boolean;
  setIsLoading: (v: boolean) => void;
}) {
  const roleColors: Record<string, string> = {
    admin: 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400',
    agent: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
    user: 'bg-gray-100 text-gray-700 dark:bg-gray-900/30 dark:text-gray-400',
  };

  return (
    <div className="p-3">
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300">
          Users ({users.length})
        </h3>
        <button
          onClick={() => setShowCreateUser(!showCreateUser)}
          className="flex items-center gap-1 rounded-lg bg-wa-green px-3 py-1.5 text-xs font-medium text-white hover:bg-wa-green/90"
        >
          {showCreateUser ? <X size={12} /> : <UserPlus size={12} />}
          {showCreateUser ? 'Cancel' : 'Add User'}
        </button>
      </div>

      {showCreateUser && (
        <CreateUserForm
          isLoading={isLoading}
          setIsLoading={setIsLoading}
          onCreated={() => {
            setShowCreateUser(false);
            onUserCreated();
          }}
        />
      )}

      <div className="space-y-2">
        {users.map((user) => (
          <div
            key={user.id}
            className="flex items-center justify-between rounded-xl border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-800"
          >
            <div className="flex items-center gap-3">
              <div className="flex h-8 w-8 items-center justify-center rounded-full bg-wa-green text-xs font-bold text-white">
                {user.username[0]?.toUpperCase()}
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <p className="text-sm font-medium text-gray-900 dark:text-white">
                    {user.display_name || user.username}
                  </p>
                  <span className={`rounded-full px-2 py-0.5 text-[10px] font-medium ${roleColors[user.role] || ''}`}>
                    {user.role}
                  </span>
                  {!user.is_active && (
                    <span className="rounded-full bg-red-100 px-2 py-0.5 text-[10px] font-medium text-red-700 dark:bg-red-900/30 dark:text-red-400">
                      inactive
                    </span>
                  )}
                </div>
                <p className="text-xs text-gray-500">{user.email}</p>
              </div>
            </div>
            {user.id !== currentUserId && (
              <button
                onClick={() => onToggleActive(user.id)}
                className={`rounded-lg p-1.5 transition-colors ${
                  user.is_active
                    ? 'text-green-500 hover:bg-green-50 dark:hover:bg-green-900/20'
                    : 'text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20'
                }`}
                title={user.is_active ? 'Deactivate user' : 'Activate user'}
              >
                {user.is_active ? <ToggleRight size={20} /> : <ToggleLeft size={20} />}
              </button>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

// ─── Create User Form ───────────────────────────────────────────

function CreateUserForm({
  isLoading,
  setIsLoading,
  onCreated,
}: {
  isLoading: boolean;
  setIsLoading: (v: boolean) => void;
  onCreated: () => void;
}) {
  const [form, setForm] = useState({
    username: '',
    email: '',
    password: '',
    display_name: '',
    role: 'agent',
  });
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsLoading(true);
    try {
      await usersApi.create({
        username: form.username,
        email: form.email,
        password: form.password,
        display_name: form.display_name || undefined,
        role: form.role,
      });
      onCreated();
    } catch (err: unknown) {
      const message =
        (err as { response?: { data?: { error?: string } } })?.response?.data?.error ||
        'Failed to create user';
      setError(message);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="mb-3 rounded-xl border border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-800">
      <div className="space-y-2">
        <input
          type="text"
          placeholder="Username"
          required
          value={form.username}
          onChange={(e) => setForm((f) => ({ ...f, username: e.target.value }))}
          className="w-full rounded-lg border border-gray-300 bg-gray-50 px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white"
        />
        <input
          type="email"
          placeholder="Email"
          required
          value={form.email}
          onChange={(e) => setForm((f) => ({ ...f, email: e.target.value }))}
          className="w-full rounded-lg border border-gray-300 bg-gray-50 px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white"
        />
        <input
          type="password"
          placeholder="Password"
          required
          minLength={6}
          value={form.password}
          onChange={(e) => setForm((f) => ({ ...f, password: e.target.value }))}
          className="w-full rounded-lg border border-gray-300 bg-gray-50 px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white"
        />
        <input
          type="text"
          placeholder="Display Name (optional)"
          value={form.display_name}
          onChange={(e) => setForm((f) => ({ ...f, display_name: e.target.value }))}
          className="w-full rounded-lg border border-gray-300 bg-gray-50 px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white"
        />
        <select
          value={form.role}
          onChange={(e) => setForm((f) => ({ ...f, role: e.target.value }))}
          className="w-full rounded-lg border border-gray-300 bg-gray-50 px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white"
        >
          <option value="agent">Agent</option>
          <option value="admin">Admin</option>
          <option value="user">User</option>
        </select>
      </div>
      {error && <p className="mt-2 text-xs text-red-500">{error}</p>}
      <button
        type="submit"
        disabled={isLoading}
        className="mt-3 flex w-full items-center justify-center gap-2 rounded-lg bg-wa-green py-2 text-sm font-medium text-white hover:bg-wa-green/90 disabled:opacity-50"
      >
        {isLoading ? <Loader2 size={14} className="animate-spin" /> : <Save size={14} />}
        Create User
      </button>
    </form>
  );
}

// ─── Profile Section ────────────────────────────────────────────

function ProfileSection({ user }: { user: User | null }) {
  if (!user) return null;

  return (
    <div className="p-3">
      <div className="rounded-xl border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800">
        <div className="flex items-center gap-4">
          <div className="flex h-16 w-16 items-center justify-center rounded-full bg-wa-green text-xl font-bold text-white">
            {user.username[0]?.toUpperCase()}
          </div>
          <div>
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
              {user.display_name || user.username}
            </h3>
            <p className="text-sm text-gray-500">@{user.username}</p>
          </div>
        </div>

        <div className="mt-4 space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-500">Email</span>
            <span className="text-sm text-gray-900 dark:text-white">{user.email}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-500">Role</span>
            <span className="rounded-full bg-wa-green/10 px-3 py-0.5 text-xs font-medium text-wa-green">
              {user.role}
            </span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-500">Status</span>
            <span className={`text-sm ${user.is_active ? 'text-green-500' : 'text-red-400'}`}>
              {user.is_active ? 'Active' : 'Inactive'}
            </span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-500">Member since</span>
            <span className="text-sm text-gray-900 dark:text-white">
              {new Date(user.created_at).toLocaleDateString()}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
