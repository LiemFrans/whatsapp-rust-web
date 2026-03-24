/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        wa: {
          green: '#00a884',
          'green-dark': '#008069',
          'green-light': '#d9fdd3',
          teal: '#128c7e',
          'teal-dark': '#075e54',
          'blue-check': '#53bdeb',
          'bg-light': '#efeae2',
          'bg-dark': '#0b141a',
          'panel-light': '#f0f2f5',
          'panel-dark': '#202c33',
          'bubble-out': '#d9fdd3',
          'bubble-in': '#ffffff',
          'bubble-out-dark': '#005c4b',
          'bubble-in-dark': '#202c33',
          'header-light': '#008069',
          'header-dark': '#202c33',
        },
      },
      fontFamily: {
        sans: ['Segoe UI', 'Helvetica Neue', 'Helvetica', 'Lucida Grande', 'Arial', 'sans-serif'],
      },
    },
  },
  plugins: [],
};
