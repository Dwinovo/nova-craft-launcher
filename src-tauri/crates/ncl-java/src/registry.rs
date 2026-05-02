//! Windows 注册表 Java 扫描。
//!
//! 历史上 Oracle JRE/JDK 安装会写入 HKLM\SOFTWARE\JavaSoft\*；
//! Adoptium 写 HKLM\SOFTWARE\Eclipse Adoptium\*；其他厂商也类似。
//!
//! 我们读取这些键的 `JavaHome` 值得到 JDK 根目录。

use crate::model::JavaSource;
use std::path::PathBuf;

#[cfg(not(target_os = "windows"))]
pub fn scan_registry() -> Vec<(PathBuf, JavaSource)> {
    Vec::new()
}

#[cfg(target_os = "windows")]
pub fn scan_registry() -> Vec<(PathBuf, JavaSource)> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
    use winreg::RegKey;

    let mut out: Vec<(PathBuf, JavaSource)> = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    // 同时尝试 64-bit 和 32-bit 注册表视图
    let views = [KEY_WOW64_64KEY, KEY_WOW64_32KEY];

    let entries: &[(&str, JavaSource)] = &[
        // Oracle / OpenJDK 系
        (r"SOFTWARE\JavaSoft\Java Runtime Environment", JavaSource::Registry),
        (r"SOFTWARE\JavaSoft\Java Development Kit", JavaSource::Registry),
        (r"SOFTWARE\JavaSoft\JDK", JavaSource::Registry),
        (r"SOFTWARE\JavaSoft\JRE", JavaSource::Registry),
        // Adoptium / Temurin
        (r"SOFTWARE\Eclipse Adoptium\JDK", JavaSource::Adoptium),
        (r"SOFTWARE\Eclipse Adoptium\JRE", JavaSource::Adoptium),
        (r"SOFTWARE\Eclipse Foundation\JDK", JavaSource::Adoptium),
        // Zulu
        (r"SOFTWARE\Azul Systems\Zulu", JavaSource::Zulu),
        // Liberica
        (r"SOFTWARE\BellSoft\Liberica", JavaSource::Liberica),
        // Microsoft
        (r"SOFTWARE\Microsoft\JDK", JavaSource::Microsoft),
    ];

    for view in views {
        for (path, source) in entries {
            let Ok(parent) = hklm.open_subkey_with_flags(path, KEY_READ | view) else {
                continue;
            };
            // 父键下每个版本是一个子键：`21.0.2` / `1.8.0_312` 等
            let Ok(versions) = parent.enum_keys().collect::<std::result::Result<Vec<_>, _>>() else {
                continue;
            };
            for ver in versions {
                let Ok(version_key) = parent.open_subkey_with_flags(&ver, KEY_READ | view) else {
                    continue;
                };
                let Ok(java_home): std::result::Result<String, _> = version_key.get_value("JavaHome") else {
                    continue;
                };
                let exe = PathBuf::from(java_home).join("bin").join("java.exe");
                if exe.is_file() {
                    out.push((exe, *source));
                }
            }
        }
    }

    out
}
