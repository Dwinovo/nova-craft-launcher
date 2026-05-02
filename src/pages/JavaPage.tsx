import { useEffect, useState } from "react";
import {
  recommendMemory,
  scanJava,
  type JavaInfo,
  type MemoryRecommendation,
} from "../lib/api";

export function JavaPage() {
  const [list, setList] = useState<JavaInfo[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [memVanilla, setMemVanilla] = useState<MemoryRecommendation | null>(null);
  const [memLoaderLight, setMemLoaderLight] = useState<MemoryRecommendation | null>(null);
  const [memLoaderHeavy, setMemLoaderHeavy] = useState<MemoryRecommendation | null>(null);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      const [data, mv, ml, mh] = await Promise.all([
        scanJava(),
        recommendMemory(false, 0),
        recommendMemory(true, 30),
        recommendMemory(true, 200),
      ]);
      setList(data);
      setMemVanilla(mv);
      setMemLoaderLight(ml);
      setMemLoaderHeavy(mh);
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
        多源扫描：JAVA_HOME · PATH · Adoptium / Microsoft / Zulu / Liberica
        默认目录 · Windows 注册表 · Mojang JRE（<code>./data/runtime/</code>）。
        优先读 <code>release</code> 文件，回退 <code>java -version</code>。
      </p>

      {error && <p style={{ color: "#d44" }}>错误：{error}</p>}

      {memVanilla && (
        <section style={{ marginTop: 16 }}>
          <h2 style={{ fontSize: 16, marginBottom: 8 }}>
            内存推荐{" "}
            <span style={{ fontSize: 12, color: "#6e6e76", fontWeight: 400 }}>
              （系统总内存 {Math.round(memVanilla.systemTotalMb / 1024)} GB ≈{" "}
              {memVanilla.systemTotalMb} MB）
            </span>
          </h2>
          <table className="paths-table">
            <thead>
              <tr style={{ textAlign: "left", color: "#6e6e76", fontSize: 12 }}>
                <th>场景</th>
                <th>-Xms</th>
                <th>-Xmx</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td className="path-key">原版（无 mod）</td>
                <td>
                  <code>{memVanilla.minMb} MB</code>
                </td>
                <td>
                  <code>{memVanilla.maxMb} MB</code>
                </td>
              </tr>
              <tr>
                <td className="path-key">Loader + 30 mods</td>
                <td>
                  <code>{memLoaderLight?.minMb} MB</code>
                </td>
                <td>
                  <code>{memLoaderLight?.maxMb} MB</code>
                </td>
              </tr>
              <tr>
                <td className="path-key">Loader + 200 mods</td>
                <td>
                  <code>{memLoaderHeavy?.minMb} MB</code>
                </td>
                <td>
                  <code>{memLoaderHeavy?.maxMb} MB</code>
                </td>
              </tr>
            </tbody>
          </table>
        </section>
      )}

      <h2 style={{ fontSize: 16, marginTop: 24, marginBottom: 8 }}>已识别 Java</h2>
      {list && list.length === 0 && (
        <p>未发现 Java 运行时。请安装 JDK 21（推荐 Adoptium / Microsoft Build of OpenJDK）后重新扫描。</p>
      )}
      {list && list.length > 0 && (
        <table className="paths-table">
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
