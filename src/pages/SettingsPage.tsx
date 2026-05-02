import { useEffect, useState } from "react";
import { getPaths, type PathInfo } from "../lib/api";

export function SettingsPage() {
  const [paths, setPaths] = useState<PathInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getPaths()
      .then(setPaths)
      .catch((e) => setError(String(e)));
  }, []);

  const rows: [string, string | undefined][] = paths
    ? [
        ["模式", paths.mode],
        ["数据根目录", paths.dataRoot],
        ["实例", paths.instances],
        ["共享 assets", paths.sharedAssets],
        ["共享 libraries", paths.sharedLibraries],
        ["共享 versions", paths.sharedVersions],
        ["配置文件", paths.configFile],
        ["日志", paths.logs],
        ["缓存", paths.cache],
      ]
    : [];

  return (
    <div>
      <h1>设置</h1>
      <h2 style={{ fontSize: 16, marginTop: 24, marginBottom: 8 }}>
        目录布局
      </h2>
      {error && <p style={{ color: "#d44" }}>错误：{error}</p>}
      {!paths && !error && <p>加载中…</p>}
      {paths && (
        <table className="paths-table">
          <tbody>
            {rows.map(([k, v]) => (
              <tr key={k}>
                <td className="path-key">{k}</td>
                <td className="path-val">
                  <code>{v}</code>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
