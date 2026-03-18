import { useMemo } from "react";
import { Icon } from "../components/Icons";
import { useWhatsApp } from "../hooks/useWhatsApp";

/** Turn a JID into a display phone/name */
function jidToDisplay(jid: string): string {
  const user = jid.split("@")[0] ?? jid;
  if (jid.includes("@lid")) return user;
  return "+" + user;
}

export default function DashboardPage() {
  const { connectionState, messages } = useWhatsApp();

  const totalMessages = messages.length;

  const activeConversations = useMemo(() => {
    const jids = new Set(messages.map((m) => m.chat));
    return jids.size;
  }, [messages]);

  const recentMessages = useMemo(() => {
    return [...messages].reverse().slice(0, 5);
  }, [messages]);

  const statusColor =
    connectionState === "connected"
      ? "#3fb950"
      : connectionState === "scanning"
        ? "#d29922"
        : "#f85149";

  const statusLabel =
    connectionState === "connected"
      ? "Connected"
      : connectionState === "scanning"
        ? "Scanning QR…"
        : "Disconnected";

  return (
    <div className="dashboard-page" data-testid="dashboard-page">
      <div className="dashboard-header">
        <h2>Dashboard</h2>
        <p>Real-time overview of your WhatsApp helpdesk.</p>
      </div>

      {/* ── Stat Cards ──────────────────── */}
      <div className="stat-cards" data-testid="stat-cards">
        <div className="stat-card" data-testid="stat-connection">
          <div className="stat-icon-wrap" style={{ background: "rgba(124, 58, 237, 0.12)" }}>
            <Icon.Signal />
          </div>
          <div className="stat-body">
            <span className="stat-label">Connection Status</span>
            <span className="stat-value" data-testid="stat-connection-value">
              <span className="status-dot" style={{ background: statusColor }} />
              {statusLabel}
            </span>
          </div>
        </div>

        <div className="stat-card" data-testid="stat-messages">
          <div className="stat-icon-wrap" style={{ background: "rgba(14, 165, 233, 0.12)" }}>
            <Icon.Chat />
          </div>
          <div className="stat-body">
            <span className="stat-label">Total Messages</span>
            <span className="stat-value" data-testid="stat-messages-value">{totalMessages}</span>
          </div>
        </div>

        <div className="stat-card" data-testid="stat-conversations">
          <div className="stat-icon-wrap" style={{ background: "rgba(16, 185, 129, 0.12)" }}>
            <Icon.Users />
          </div>
          <div className="stat-body">
            <span className="stat-label">Active Conversations</span>
            <span className="stat-value" data-testid="stat-conversations-value">{activeConversations}</span>
          </div>
        </div>

        <div className="stat-card" data-testid="stat-uptime">
          <div className="stat-icon-wrap" style={{ background: "rgba(210, 153, 34, 0.12)" }}>
            <Icon.Clock />
          </div>
          <div className="stat-body">
            <span className="stat-label">System Uptime</span>
            <span className="stat-value" data-testid="stat-uptime-value">99.9%</span>
          </div>
        </div>
      </div>

      {/* ── Recent Activity Table ───────── */}
      <div className="dashboard-card" data-testid="activity-table">
        <h3><Icon.Calendar /> Recent Activity</h3>
        {recentMessages.length === 0 ? (
          <div className="table-empty" data-testid="activity-empty">
            <Icon.Inbox />
            <p>No messages yet. Activity will appear here in real time.</p>
          </div>
        ) : (
          <div className="table-wrapper">
            <table className="activity-table">
              <thead>
                <tr>
                  <th>Time</th>
                  <th>Sender</th>
                  <th>Message</th>
                  <th>Type</th>
                </tr>
              </thead>
              <tbody>
                {recentMessages.map((m) => (
                  <tr key={m.id}>
                    <td className="td-date">
                      {new Date(m.timestamp * 1000).toLocaleTimeString([], {
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </td>
                    <td>{m.fromName || jidToDisplay(m.from)}</td>
                    <td className="td-message">{m.text}</td>
                    <td>
                      <span
                        className={`campaign-badge ${m.isFromMe ? "badge-sent" : "badge-incoming"}`}
                      >
                        {m.isFromMe ? "Outgoing" : "Incoming"}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
