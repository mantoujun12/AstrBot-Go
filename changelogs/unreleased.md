## English

### Added

- Instances store the AstrBot WebUI login (username and password) used by the launcher's built-in chat; the credentials can be replaced or cleared from the instance form.
- Show the detected runtime version of each dependency (Python 3.10 / 3.12, Node.js, uv) on the Versions page and split the components section into one card per dependency; an installed runtime whose version cannot be detected shows an "Unknown" badge.
- Split the Tasks page into History and Scheduled sections; each scheduled task can start, stop, back up or clear the cache of an instance once, on an interval, daily or weekly while the app is running.
- Manage an instance's personas, scheduled jobs, MCP servers, config files and knowledge bases directly in the launcher, including copying them to other instances.
- Add the Extensions page with a card grid, reachable from a new sidebar entry.
- Add a standalone chat window opened from the entry at the top of the sidebar; it opens directly into the conversation, with the target instance and model chosen inside the window.
- The chat window's "Instance & Model" sidebar panel switches the target instance and picks the model (also selectable at the bottom-left of the composer), and the window uses a custom title bar on Windows and Linux with minimize, maximize/restore and close controls that stay usable while its settings dialog is open (macOS keeps the native traffic lights).
- Rename and delete chat sessions from the sidebar; when the target instance is not running or has no WebUI login configured, the window states the reason instead of a generic load error and blocks sending until the instance is ready.

### Changed

- The chat window title bar no longer shows the current session title.
- Theme and language picked in the chat window now follow the system when chosen and stay in sync with the launcher window.
- Chat code blocks highlight a fixed whitelist of common languages; fences for other languages render as plain text.
- Update frontend and Rust dependencies, including rusqlite (0.37 to 0.43), shiki (3 to 4), Tauri libraries and the release workflow actions.

## 中文

### Added

- 实例可保存 AstrBot WebUI 的登录账号（用户名与密码），供启动器内置聊天使用；可在实例表单中更换或清除。
- 版本页现在显示每个依赖的检测版本（Python 3.10 / 3.12、Node.js、uv），并将组件区按依赖拆分为独立卡片；已安装但无法检测版本的运行时显示"未知"标记。
- 任务页拆分为“历史记录”与“定时任务”两部分：定时任务可按一次性、间隔、每天或每周运行，执行启动、停止、备份或清除实例缓存，仅在应用运行时生效。
- 在启动器中直接管理实例的人格、定时任务、MCP 服务器、配置文件与知识库，并支持复制到其他实例。
- 新增扩展功能页面，以卡片网格呈现，侧边栏增加对应入口。
- 新增独立聊天窗口，入口位于侧边栏顶部；打开后直接进入聊天界面，目标实例与模型在窗口内选择。
- 聊天窗口侧边栏新增“实例与模型”面板，可切换目标实例并选择模型（输入框底部左侧亦可选择）；Windows 和 Linux 上改用自定义标题栏，提供最小化、最大化/还原和关闭按钮，设置对话框打开时按钮仍可点击（macOS 保留原生红绿灯按钮）。
- 侧边栏支持重命名和删除会话；目标实例未运行或未配置 WebUI 登录凭据时，窗口会直接说明原因，不再显示通用的加载失败提示，并在实例就绪前禁用发送。

### Changed

- 聊天窗口标题栏不再显示当前会话标题。
- 聊天窗口中选择的主题与语言支持跟随系统，并与启动器窗口双向同步。
- 聊天代码块只对固定的常用语言白名单进行语法高亮，其他语言的代码块以纯文本显示。
- 更新前端与 Rust 依赖，包括 rusqlite（0.37 至 0.43）、shiki（3 至 4）、Tauri 相关库及发布工作流 Actions。
