import { NavLink, Outlet } from "react-router-dom";
import "./AppShell.css";

const navItems = [
  { to: "/", label: "概览", icon: "◇", end: true },
  { to: "/instances", label: "实例", icon: "▦" },
  { to: "/mods", label: "Mod", icon: "◉" },
  { to: "/java", label: "Java", icon: "☕" },
  { to: "/settings", label: "设置", icon: "⚙" },
];

export function AppShell() {
  return (
    <div className="app-shell">
      {/* ── Sidebar (PCL-style vertical nav, Notion visual language) ───── */}
      <aside className="sidebar">
        <header className="sidebar-brand">
          <span className="sidebar-brand-glyph">N</span>
          <span className="sidebar-brand-text">
            <span className="sidebar-brand-name">Nova Craft</span>
            <span className="sidebar-brand-sub">Launcher</span>
          </span>
        </header>

        <nav className="sidebar-nav">
          {navItems.map((it) => (
            <NavLink
              key={it.to}
              to={it.to}
              end={it.end}
              className={({ isActive }) =>
                isActive ? "sidebar-link active" : "sidebar-link"
              }
            >
              <span className="sidebar-link-icon" aria-hidden>
                {it.icon}
              </span>
              <span>{it.label}</span>
            </NavLink>
          ))}
        </nav>

        <footer className="sidebar-footer">
          <div className="sidebar-footer-row">
            <span className="sidebar-footer-version">v0.1.0 · alpha</span>
          </div>
          <div className="sidebar-footer-stage">阶段 1 — 已完成</div>
        </footer>
      </aside>

      {/* ── Main scrollable area ────────────────────────────────────── */}
      <main className="main-area">
        <Outlet />
      </main>
    </div>
  );
}
