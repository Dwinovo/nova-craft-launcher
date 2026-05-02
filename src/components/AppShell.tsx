import { NavLink, Outlet, useLocation } from "react-router-dom";
import "./AppShell.css";

const navItems = [
  { to: "/", label: "首页", end: true },
  { to: "/instances", label: "实例" },
  { to: "/mods", label: "Mod" },
  { to: "/java", label: "Java" },
  { to: "/settings", label: "设置" },
];

const subTitles: Record<string, string> = {
  "/": "Nova Craft",
  "/instances": "实例",
  "/mods": "Mod",
  "/java": "Java",
  "/settings": "设置",
};

export function AppShell() {
  const { pathname } = useLocation();
  const subTitle = subTitles[pathname] ?? "Nova Craft";

  return (
    <div className="app-shell">
      <nav className="global-nav">
        <div className="global-nav-inner">
          <span className="brand">
            <span className="brand-glyph" aria-hidden>◆</span>
            <span>Nova Craft Launcher</span>
          </span>
          <ul className="global-nav-links">
            {navItems.map((it) => (
              <li key={it.to}>
                <NavLink
                  to={it.to}
                  end={it.end}
                  className={({ isActive }) =>
                    isActive ? "global-nav-link active" : "global-nav-link"
                  }
                >
                  {it.label}
                </NavLink>
              </li>
            ))}
          </ul>
          <span className="global-nav-meta">v0.1.0 · Sprint 5 · alpha</span>
        </div>
      </nav>

      <div className="sub-nav">
        <div className="sub-nav-inner">
          <span className="t-tagline">{subTitle}</span>
        </div>
      </div>

      <main className="main-area">
        <Outlet />
      </main>
    </div>
  );
}
