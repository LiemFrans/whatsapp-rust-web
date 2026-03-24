import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { MessageCircle, Eye, EyeOff, Loader2, AlertCircle } from 'lucide-react';
import { useAuthStore } from '@/store/authStore';

export default function Login() {
  const navigate = useNavigate();
  const { login, isLoading } = useAuthStore();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError('');

    if (!username.trim() || !password.trim()) {
      setError('Please fill in all fields');
      return;
    }

    try {
      await login(username.trim(), password);
      navigate('/mode-select');
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Login failed';
      setError(msg);
    }
  };

  return (
    <div className="flex h-full items-center justify-center bg-gradient-to-br from-wa-green/5 via-white to-wa-teal/5 dark:from-gray-900 dark:via-gray-900 dark:to-gray-800">
      {/* Decorative header */}
      <div className="absolute left-0 right-0 top-0 h-56 bg-wa-green dark:bg-wa-dark-header" />

      <div className="relative z-10 w-full max-w-md px-4">
        <div className="rounded-2xl bg-white p-8 shadow-2xl dark:bg-gray-800">
          {/* Logo */}
          <div className="mb-8 flex flex-col items-center">
            <div className="mb-4 flex h-20 w-20 items-center justify-center rounded-full bg-wa-green shadow-lg">
              <MessageCircle size={40} className="text-white" />
            </div>
            <h1 className="text-2xl font-bold text-gray-800 dark:text-white">WhatsApp Web</h1>
            <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
              Personal & Business Mode
            </p>
          </div>

          {/* Error */}
          {error && (
            <div className="mb-4 flex items-center gap-2 rounded-lg bg-red-50 px-4 py-3 text-sm text-red-600 dark:bg-red-900/20 dark:text-red-400">
              <AlertCircle size={16} className="shrink-0" />
              {error}
            </div>
          )}

          {/* Form */}
          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label htmlFor="username" className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">
                Username
              </label>
              <input
                id="username"
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="Enter your username"
                autoComplete="username"
                className="w-full rounded-lg border border-gray-300 bg-white px-4 py-2.5 text-sm text-gray-800 placeholder-gray-400 outline-none transition-colors focus:border-wa-green focus:ring-2 focus:ring-wa-green/20 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-200"
              />
            </div>

            <div>
              <label htmlFor="password" className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">
                Password
              </label>
              <div className="relative">
                <input
                  id="password"
                  type={showPassword ? 'text' : 'password'}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="Enter your password"
                  autoComplete="current-password"
                  className="w-full rounded-lg border border-gray-300 bg-white px-4 py-2.5 pr-10 text-sm text-gray-800 placeholder-gray-400 outline-none transition-colors focus:border-wa-green focus:ring-2 focus:ring-wa-green/20 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-200"
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
                >
                  {showPassword ? <EyeOff size={18} /> : <Eye size={18} />}
                </button>
              </div>
            </div>

            <button
              type="submit"
              disabled={isLoading}
              className="flex w-full items-center justify-center gap-2 rounded-lg bg-wa-green py-3 text-sm font-semibold text-white shadow-md transition-all hover:bg-wa-green/90 hover:shadow-lg disabled:opacity-60"
            >
              {isLoading ? (
                <>
                  <Loader2 size={18} className="animate-spin" />
                  Signing in...
                </>
              ) : (
                'Sign In'
              )}
            </button>
          </form>

          <div className="mt-6 text-center text-xs text-gray-400 dark:text-gray-500">
            <p>Default admin: <span className="font-mono">admin / admin123</span></p>
          </div>
        </div>
      </div>
    </div>
  );
}
