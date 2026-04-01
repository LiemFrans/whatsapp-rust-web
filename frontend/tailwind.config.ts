import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./app/**/*.{js,ts,jsx,tsx,mdx}",
    "./components/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        wa: {
          teal: "#00a884",
          "teal-dark": "#008069",
          "teal-header": "#008069",
          green: "#25d366",
          "bg": "#f0f2f5",
          "bg-chat": "#efeae2",
          panel: "#ffffff",
          "bubble-out": "#d9fdd3",
          "bubble-in": "#ffffff",
          text: "#111b21",
          "text-secondary": "#667781",
          "input-bg": "#f0f2f5",
          "sidebar-hover": "#f5f6f6",
          border: "#e9edef",
          "icon": "#54656f",
        },
      },
    },
  },
  plugins: [],
};

export default config;
