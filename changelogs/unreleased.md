## English

### Security

- Uninstalling a plugin now validates the data-directory name taken from its `metadata.yaml` before deleting `plugin_data`, so a plugin that rewrites its own metadata can no longer redirect the recursive delete outside the plugin data directory.
- Harden external link handling: only `http`/`https` URLs can be opened or embedded; other URI schemes are rejected.
- Tab-shortcut bridge messages are now accepted only from windows of live embedded instance-WebUI frames, so other frames (for example model-generated HTML panels) can no longer switch, reorder, or close launcher tabs.
- Tightened Tauri window capabilities to least privilege: the main window no longer holds the broad `core:default` set (path, image, menu, tray, webview, event emit), and the chat and tray panel windows can no longer broadcast events.
- Applied a Content Security Policy to the webview that blocks injected inline and remote scripts, as defense-in-depth for chat markdown, generated HTML previews, and embedded WebUI pages.

### Fixed

- Component runtime installs (Python, Node.js, uv) now download and extract into a staging directory before replacing the installed runtime, so a failed download or extraction no longer breaks the existing runtime.
- Aligned switch, alert, progress bar, and radio option styling with the project control conventions, and added an accessible label to the reference sidebar close button.
- Localized the hardcoded English copy in the chat island's HTML GenUI panel and Python tool block, and corrected Japanese/Chinese punctuation in chat island locale copy.

## 中文

### 安全

- 卸载插件时会先校验从其 `metadata.yaml` 读取的数据目录名再删除 `plugin_data`，插件重写自身元数据也无法让递归删除越过插件数据目录。
- 强化外部链接处理：仅可打开或嵌入 `http`/`https` 地址，其他 URI 协议将被拒绝。
- 标签页快捷键桥接消息现在仅接受来自已嵌入的实例 WebUI 框架窗口，其他框架（例如模型生成的 HTML 面板）不再能切换、重排或关闭启动器标签页。
- 收紧 Tauri 窗口能力至最小权限：主窗口不再持有宽泛的 `core:default` 权限集（路径、图片、菜单、托盘、Web 视图、事件广播），聊天窗口与托盘面板窗口也不再允许广播事件。
- 为 webview 启用内容安全策略（CSP），阻止注入的内联或远程脚本在应用内执行，作为聊天 Markdown、生成的 HTML 预览与内嵌 WebUI 页面的纵深防御。

### 修复

- 组件运行时（Python、Node.js、uv）安装现先在暂存目录中完成下载与解压，再替换现有运行时，下载或解压失败不会再破坏已安装的运行时。
- 统一开关、提示条、进度条和单选项的样式规范，并为引用侧栏的关闭按钮补充无障碍标签。
- 本地化聊天岛屿 HTML GenUI 面板与 Python 工具块中硬编码的英文文案，并修正聊天岛屿多语言文案中的日文/中文标点。
