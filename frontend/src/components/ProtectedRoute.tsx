import React from 'react';
import { Navigate } from 'react-router-dom';
import { useAuthStore } from '@/store/authStore';

interface ProtectedRouteProps {
  children: React.ReactNode;
  requiredRole?: 'admin' | 'agent' | 'user';
}

export function ProtectedRoute({ children, requiredRole }: ProtectedRouteProps) {
  const { isAuthenticated, user } = useAuthStore();

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  if (requiredRole && user) {
    if (requiredRole === 'admin' && user.role !== 'admin') {
      return <Navigate to="/mode-select" replace />;
    }
    if (requiredRole === 'agent' && user.role === 'user') {
      return <Navigate to="/mode-select" replace />;
    }
  }

  return <>{children}</>;
}
