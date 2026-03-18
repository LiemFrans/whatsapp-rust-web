import { useState } from "react";
import { Icon } from "../components/Icons";
import { useToast } from "../components/Toast";
import { useWhatsApp } from "../hooks/useWhatsApp";

interface Campaign {
  id: number;
  date: string;
  message: string;
  recipients: number;
  status: "sent" | "failed" | "queued";
}

export default function BroadcastsPage() {
  const { showToast } = useToast();
  const { sendMessage: waSend } = useWhatsApp();
  const [messageBody, setMessageBody] = useState("");
  const [phoneNumbers, setPhoneNumbers] = useState("");
  const [campaigns, setCampaigns] = useState<Campaign[]>([]);

  const handleSendCampaign = () => {
    const msg = messageBody.trim();
    const phones = phoneNumbers.trim();
    if (!msg) {
      showToast("Please enter a message body");
      return;
    }
    if (!phones) {
      showToast("Please enter at least one phone number");
      return;
    }
    const recipientCount = phones.split("\n").filter((l) => l.trim()).length;
    const recipients = phones.split("\n").map((l) => l.trim()).filter(Boolean);

    // Send each message via the WhatsApp backend
    for (const phone of recipients) {
      const rawNumber = phone.replace(/[\s\-\+\(\)]/g, "");
      waSend(rawNumber, msg);
    }

    const newCampaign: Campaign = {
      id: Date.now(),
      date: new Date().toISOString().replace("T", " ").slice(0, 16),
      message: msg.length > 60 ? msg.slice(0, 60) + "…" : msg,
      recipients: recipientCount,
      status: "queued",
    };
    setCampaigns((prev) => [newCampaign, ...prev]);
    setMessageBody("");
    setPhoneNumbers("");
    showToast(`Campaign queued for ${recipientCount} recipient${recipientCount > 1 ? "s" : ""}`);
  };

  return (
    <div className="broadcasts-page" data-testid="broadcasts-page">
      <div className="broadcasts-header">
        <h2>Broadcasts</h2>
        <p>Send bulk WhatsApp messages to multiple contacts at once.</p>
      </div>

      <div className="broadcasts-content">
        {/* ── Compose Campaign ──────────── */}
        <div className="broadcast-card compose-card" data-testid="compose-card">
          <h3><Icon.Broadcast /> New Campaign</h3>
          <div className="broadcast-field">
            <label>Message Body</label>
            <textarea
              className="broadcast-textarea"
              placeholder="Type your broadcast message…"
              rows={4}
              value={messageBody}
              onChange={(e) => setMessageBody(e.target.value)}
              data-testid="broadcast-message"
            />
          </div>
          <div className="broadcast-field">
            <label>Phone Numbers <span className="field-hint">(one per line)</span></label>
            <textarea
              className="broadcast-textarea phones"
              placeholder={"+62 812-3456-7890\n+1 555-0123\n+44 7700-900123"}
              rows={4}
              value={phoneNumbers}
              onChange={(e) => setPhoneNumbers(e.target.value)}
              data-testid="broadcast-phones"
            />
          </div>
          <button className="settings-btn primary" onClick={handleSendCampaign} data-testid="send-campaign-btn">
            <Icon.Send /> Send Campaign
          </button>
        </div>

        {/* ── Recent Campaigns Table ────── */}
        <div className="broadcast-card" data-testid="campaigns-table">
          <h3><Icon.Calendar /> Recent Campaigns</h3>
          <div className="table-wrapper">
            <table className="campaigns-table">
              <thead>
                <tr>
                  <th>Date</th>
                  <th>Message</th>
                  <th>Recipients</th>
                  <th>Status</th>
                </tr>
              </thead>
              <tbody>
                {campaigns.map((c) => (
                  <tr key={c.id}>
                    <td className="td-date">{c.date}</td>
                    <td className="td-message">{c.message}</td>
                    <td className="td-recipients">
                      <Icon.Users /> {c.recipients}
                    </td>
                    <td>
                      <span className={"campaign-badge badge-" + c.status}>{c.status}</span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}
