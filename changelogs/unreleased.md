## English

### Added

- Instance list columns are now customizable: the toolbar gains a column menu to show or hide the optional version and port columns, and the choice persists across sessions.
- Instances can now be selected in batches (table and card views, launcher instances only) and started, restarted, or stopped together, with a confirmation dialog and a result summary.

### Security

- The dashboard proxy now rejects paths containing percent-encoded dot segments (`%2e%2e`), backslashes, encoded slashes (`%2f`), or fragment identifiers, closing traversal vectors that could bypass the allowlisted route check.

## 中文

### 新增

- 实例列表列展示可自定义：工具栏新增列菜单，可选择显示或隐藏可选的版本与端口列，选择会跨会话保留。
- 实例支持批量选择（表格与卡片视图，仅限启动器管理的实例），并可批量启动、重启或停止，附带确认对话框与结果汇总。

### 安全

- 仪表板代理现在会拒绝包含百分号编码点段（`%2e%2e`）、反斜杠、编码斜杠（`%2f`）或片段标识符的路径，修复了可绕过路由白名单的路径遍历漏洞。
