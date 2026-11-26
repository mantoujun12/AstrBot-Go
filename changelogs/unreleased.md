## English

### Security

- Uninstalling a plugin now validates the data-directory name taken from its `metadata.yaml` before deleting `plugin_data`, so a plugin that rewrites its own metadata can no longer redirect the recursive delete outside the plugin data directory.
- Harden external link handling: only `http`/`https` URLs can be opened or embedded; other URI schemes are rejected.
- Tab-shortcut bridge messages are now accepted only from windows of live embedded instance-WebUI frames, so other frames (for example model-generated HTML panels) can no longer switch, reorder, or close launcher tabs.

### Fixed

- Component runtime installs (Python, Node.js, uv) now download and extract into a staging directory before replacing the installed runtime, so a failed download or extraction no longer breaks the existing runtime.
- Aligned switch, alert, progress bar, and radio option styling with the project control conventions, and added an accessible label to the reference sidebar close button.

## 中文

### 安全

- 卸载插件时会先校验从其 `metadata.yaml` 读取的数据目录名再删除 `plugin_data`，插件重写自身元数据也无法让递归删除越过插件数据目录。
- 强化外部链接处理：仅可打开或嵌入 `http`/`https` 地址，其他 URI 协议将被拒绝。
- 标签页快捷键桥接消息现在仅接受来自已嵌入的实例 WebUI 框架窗口，其他框架（例如模型生成的 HTML 面板）不再能切换、重排或关闭启动器标签页。

### 修复

- 组件运行时（Python、Node.js、uv）安装现先在暂存目录中完成下载与解压，再替换现有运行时，下载或解压失败不会再破坏已安装的运行时。
- 统一开关、提示条、进度条和单选项的样式规范，并为引用侧栏的关闭按钮补充无障碍标签。
