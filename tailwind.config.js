/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx,js,jsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        bg: "#0d1117",
        panel: "#161b22",
        panel2: "#1c232c",
        border: "#30363d",
        text: "#e6edf3",
        muted: "#8b949e",
        primary: "#7c3aed",
        primaryHover: "#8b5cf6",
        success: "#52c41a",
        warn: "#ffa940",
        danger: "#ff4d4f",
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
    },
  },
  plugins: [],
};
