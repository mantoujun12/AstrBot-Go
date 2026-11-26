## English

### Security

- Uninstalling a plugin now validates the data-directory name taken from its `metadata.yaml` before deleting `plugin_data`, so a plugin that rewrites its own metadata can no longer redirect the recursive delete outside the plugin data directory.
- Harden external link handling: only `http`/`https` URLs can be opened or embedded; other URI schemes are rejected.

## 中文

### 安全

- 卸载插件时会先校验从其 `metadata.yaml` 读取的数据目录名再删除 `plugin_data`，插件重写自身元数据也无法让递归删除越过插件数据目录。
- 强化外部链接处理：仅可打开或嵌入 `http`/`https` 地址，其他 URI 协议将被拒绝。
