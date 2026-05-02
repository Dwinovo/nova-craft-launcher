import { useEffect, useState } from "react";
import { scanJava, type JavaInfo } from "../lib/api";

export function JavaPage() {
  const [list, setList] = useState<JavaInfo[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      const data = await scanJava();
      setList(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <h1>Java 配置</h1>
        <button onClick={refresh} disabled={loading} style={refreshBtn(loading)}>
          {loading ? "扫描中…" : "重新扫描"}
        </button>
      </div>
      <p style={{ color: "#6e6e76" }}>
        Sprint 1 MVP：仅扫 <code>JAVA_HOME</code> + <code>PATH</code>。Sprint 5
        会补全注册表 / Adoptium / Microsoft / Zulu / Mojang JRE。
      </p>
      {error && <p style={{ color: "#d44" }}>错误：{error}</p>}
      {list && list.length === 0 && (
        <p>未发现 Java 运行时。请安装 JDK 21（Adoptium / Microsoft Build of OpenJDK）后重新扫描。</p>
      )}
      {list && list.length > 0 && (
        <table className="paths-table" style={{ marginTop: 16 }}>
          <thead>
            <tr style={{ textAlign: "left", color: "#6e6e76", fontSize: 12 }}>
              <th>主版本</th>
              <th>完整版本</th>
              <th>厂商</th>
              <th>架构</th>
              <th>来源</th>
              <th>路径</th>
            </tr>
          </thead>
          <tbody>
            {list.map((j) => (
              <tr key={j.path}>
                <td className="path-key">
                  <strong>{j.versionMajor}</strong>
                </td>
                <td>{j.versionFull}</td>
                <td>{j.vendor}</td>
                <td>{j.arch}</td>
                <td>{j.source}</td>
                <td className="path-val">
                  <code>{j.path}</code>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

function refreshBtn(disabled: boolean): React.CSSProperties {
  return {
    padding: "6px 14px",
    background: disabled ? "#a8a8b0" : "#396cd8",
    color: "white",
    border: "none",
    borderRadius: 6,
    cursor: disabled ? "not-allowed" : "pointer",
    fontSize: 13,
  };
}
