import { Routes, Route, Navigate } from 'react-router-dom';
import { useEffect } from 'react';
import { useAuthStore } from '@/store/authStore';
import { ProtectedRoute } from '@/components/ProtectedRoute';
import Login from '@/pages/Login';
import ModeSelect from '@/pages/ModeSelect';
import PersonalMode from '@/pages/PersonalMode';
import BusinessMode from '@/pages/BusinessMode';

export default function App() {
  const { isAuthenticated, checkAuth } = useAuthStore();

  useEffect(() => {
    checkAuth();
  }, [checkAuth]);

  return (
    <div className="h-screen w-screen overflow-hidden bg-wa-bg dark:bg-wa-dark-bg">
      <Routes>
        <Route
          path="/login"
          element={isAuthenticated ? <Navigate to="/mode-select" replace /> : <Login />}
        />
        <Route
          path="/mode-select"
          element={
            <ProtectedRoute>
              <ModeSelect />
            </ProtectedRoute>
          }
        />
        <Route
          path="/personal/*"
          element={
            <ProtectedRoute>
              <PersonalMode />
            </ProtectedRoute>
          }
        />
        <Route
          path="/business/*"
          element={
            <ProtectedRoute>
              <BusinessMode />
            </ProtectedRoute>
          }
        />
        <Route path="*" element={<Navigate to={isAuthenticated ? '/mode-select' : '/login'} replace />} />
      </Routes>
    </div>
  );
}
