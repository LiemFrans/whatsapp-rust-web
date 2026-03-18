import { NavLink, Outlet } from "react-router-dom";
import { Icon } from "./components/Icons";
import { Modal } from "./components/Modal";
import { useToast } from "./components/Toast";
import { useState } from "react";
import "./App.css";

const NAV_ITEMS: { to: string; label: string; icon: JSX.Element }[] = [
  { to: "/dashboard", label: "Dashboard", icon: <Icon.Dashboard /> },
  { to: "/", label: "Inbox", icon: <Icon.Inbox /> },
  { to: "/broadcasts", label: "Broadcasts", icon: <Icon.Broadcast /> },
  { to: "/settings", label: "Settings", icon: <Icon.Settings /> },
];

export default function App() {
  const { showToast } = useToast();
  const [profileOpen, setProfileOpen] = useState(false);

  return (
    <div className="dashboard">
      {/* ── Sidebar ──────────────────────── */}
      <aside className="sidebar">
        <div className="sidebar-logo">HD</div>

        <nav className="sidebar-nav">
          {NAV_ITEMS.map(({ to, label, icon }) => (
            <NavLink
              key={to}
              to={to}
              end={to === "/"}
              className={({ isActive }) =>
                `sidebar-btn ${isActive ? "active" : ""}`
              }
            >
              {icon}
              {label}
            </NavLink>
          ))}
        </nav>

        <div className="sidebar-spacer" />
        <div
          className="sidebar-avatar"
          onClick={() => setProfileOpen(true)}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => e.key === "Enter" && setProfileOpen(true)}
        >
          FD
        </div>
      </aside>

      {/* ── Main content (routed) ────────── */}
      <Outlet />

      {/* ── My Profile Modal ─────────────── */}
      <Modal title="My Profile" open={profileOpen} onClose={() => setProfileOpen(false)}>
        <div className="modal-profile-card">
          <div className="modal-profile-avatar" style={{ background: "#7c3aed" }}>
            FD
          </div>
          <div className="modal-profile-name">Frans D.</div>
          <div className="modal-profile-detail">Support Agent</div>
          <div className="modal-profile-detail">frans@helpdesk.io</div>
        </div>
      </Modal>
    </div>
  );
}
