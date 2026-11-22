## English

### Changed
- The instances page no longer flashes the empty state while external instances are still being discovered.
- Status chips (`v-chip`) now use capsule (pill) corners, and Markdown tables and the log search highlight gained rounded corners.
- The instances page now has a sticky toolbar with search, state/source filters, sorting, and a switchable table/card view (view and sort preference are remembered); the page title was removed.
- The instances list keeps starting/stopping rows visible under their target state filter, shows a loading bar in cards view, and the sort selector and Beta chip are now labeled and localized.
- The instances toolbar controls now shrink smoothly at narrow window widths instead of wrapping early.
- The instances page refresh now runs its three reloads as one coordinated operation with a single busy state.

### Fixed
- The directory scan no longer descends into symlinks and Windows junctions, silencing access-denied log noise; a deployment behind a link can be added as a scan path instead.
- Exiting the app now destroys all webview windows before terminating, removing the Chromium `Chrome_WidgetWin_0` unregister error printed at shutdown.

## 中文

### 变更

- 实例页面在发现外部实例完成前不再闪现空状态。
- 状态徽章（`v-chip`）改为胶囊形圆角，Markdown 表格与日志搜索高亮增加了圆角。
- 实例管理页面新增固定工具栏，支持搜索、状态与来源筛选、排序，并可在表格与卡片视图间切换（视图与排序偏好会被记住）；移除了页面标题。
- 实例列表在状态筛选下保留启动中/停止中的实例，卡片视图显示加载进度条，排序下拉与 Beta 徽章完成标注与本地化。
- 实例工具栏控件在窄窗口下平滑收缩，不再过早换行。
- 实例页面刷新改为一次协调执行三项重载，并统一显示忙碌状态。

### 修复

- 目录扫描不再进入符号链接与 Windows junction，消除拒绝访问的日志噪声；链接后的部署可以改用添加扫描路径的方式发现。
- 退出应用时先销毁全部 webview 窗口再结束进程，消除退出时 Chromium 的 `Chrome_WidgetWin_0` 窗口类注销报错。
