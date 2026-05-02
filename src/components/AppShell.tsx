import { NavLink, Outlet } from "react-router-dom";
import "./AppShell.css";

const links = [
  { to: "/", label: "首页", end: true },
  { to: "/instances", label: "实例" },
  { to: "/mods", label: "Mod" },
  { to: "/java", label: "Java" },
  { to: "/settings", label: "设置" },
];

export function AppShell() {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-logo">🚀</span>
          <span className="brand-text">Nova Craft</span>
        </div>
        <nav>
          {links.map((l) => (
            <NavLink
              key={l.to}
              to={l.to}
              end={l.end}
              className={({ isActive }) =>
                isActive ? "nav-item active" : "nav-item"
              }
            >
              {l.label}
            </NavLink>
          ))}
        </nav>
        <div className="sidebar-footer">v0.1.0 · Sprint 0</div>
      </aside>
      <main className="main-area">
        <Outlet />
      </main>
    </div>
  );
}
