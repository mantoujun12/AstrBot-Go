## English

### Security

- Uninstalling a plugin now validates the data-directory name taken from its `metadata.yaml` before deleting `plugin_data`, so a plugin that rewrites its own metadata can no longer redirect the recursive delete outside the plugin data directory.
- Harden external link handling: only `http`/`https` URLs can be opened or embedded; other URI schemes are rejected.

### Fixed

- Component runtime installs (Python, Node.js, uv) now download and extract into a staging directory before replacing the installed runtime, so a failed download or extraction no longer breaks the existing runtime.

## 中文

### 安全

- 卸载插件时会先校验从其 `metadata.yaml` 读取的数据目录名再删除 `plugin_data`，插件重写自身元数据也无法让递归删除越过插件数据目录。
- 强化外部链接处理：仅可打开或嵌入 `http`/`https` 地址，其他 URI 协议将被拒绝。

### 修复

- 组件运行时（Python、Node.js、uv）安装现先在暂存目录中完成下载与解压，再替换现有运行时，下载或解压失败不会再破坏已安装的运行时。
