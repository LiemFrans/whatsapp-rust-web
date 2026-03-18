import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createBrowserRouter, RouterProvider } from "react-router-dom";
import { ToastProvider } from "./components/Toast";
import { WhatsAppProvider } from "./hooks/useWhatsApp";
import App from "./App";
import InboxPage from "./pages/InboxPage";
import DashboardPage from "./pages/DashboardPage";
import BroadcastsPage from "./pages/BroadcastsPage";
import SettingsPage from "./pages/SettingsPage";
import "./index.css";

const router = createBrowserRouter([
  {
    path: "/",
    element: <App />,
    children: [
      { index: true, element: <InboxPage /> },
      { path: "dashboard", element: <DashboardPage /> },
      { path: "broadcasts", element: <BroadcastsPage /> },
      { path: "settings", element: <SettingsPage /> },
    ],
  },
]);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ToastProvider>
      <WhatsAppProvider>
        <RouterProvider router={router} />
      </WhatsAppProvider>
    </ToastProvider>
  </StrictMode>
);
