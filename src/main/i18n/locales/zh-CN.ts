const zhCN = {
  "common": {
    "actions": {
      "confirm": "确认",
      "cancel": "取消",
      "close": "关闭",
      "save": "保存",
      "delete": "删除",
      "edit": "编辑",
      "create": "创建",
      "refresh": "刷新",
      "retry": "重试",
      "copy": "复制",
      "open": "打开",
      "install": "安装",
      "reinstall": "重新安装",
      "uninstall": "卸载",
      "enable": "启用",
      "disable": "禁用"
    },
    "operationComplete": "操作完成",
    "processing": "处理中…",
    "instanceState": {
      "running": "运行中",
      "starting": "启动中",
      "stopping": "停止中",
      "stopped": "已停止"
    },
    "duration": {
      "hours": "{hours} 小时",
      "hoursMinutes": "{hours} 小时 {minutes} 分",
      "minutes": "{minutes} 分",
      "lessThanMinute": "< 1 分"
    }
  },
  "launcherUpdate": {
    "title": "发现新版本 {version}",
    "update": "立即更新",
    "remindLater": "稍后提醒",
    "ignore": "忽略本次版本",
    "noNotes": "此版本暂无更新说明"
  },
  "trayPanel": {
    "title": "AstrBot Go",
    "openMainWindow": "打开主窗口",
    "openSettings": "打开设置",
    "quit": "退出",
    "empty": "暂无实例",
    "loading": "正在加载实例…",
    "loadFailed": "实例加载失败",
    "actions": {
      "open": "打开 {name}",
      "start": "启动 {name}",
      "stop": "停止 {name}",
      "confirmStop": "再次点击确认停止 {name}",
      "confirmQuit": "再次点击确认退出",
      "restart": "重启 {name}"
    }
  },
  "settings": {
    "appearance": {
      "language": "语言",
      "followSystem": "跟随系统",
      "theme": "主题",
      "light": "浅色",
      "dark": "深色",
      "themeColor": "主题色",
      "primary": "主色",
      "secondary": "次色",
      "reset": "重置",
      "reduceMotion": "减少动态",
      "reduceMotionOn": "开启",
      "reduceMotionOff": "关闭",
      "reduceMotionHint": "减弱或关闭页面切换与界面过渡动画效果"
    },
    "title": "设置",
    "loading": "正在加载设置…",
    "loadFailed": "设置加载失败",
    "saved": "已保存",
    "instances": {
      "advanced": "高级",
      "refresh": "刷新",
      "count": "{count} 个实例",
      "sources": {
        "launcher": "启动器部署",
        "docker": "Docker 部署",
        "uv_tool": "uv tool（astrbot 命令行）",
        "dir_scan": "目录扫描"
      },
      "scanPaths": {
        "title": "扫描目录",
        "add": "添加目录",
        "remove": "移除目录"
      },
    },
    "nav": {
      "general": "常规",
      "appearance": "外观",
      "instances": "实例",
      "network": "网络",
      "data": "数据与维护",
      "about": "关于",
      "ariaLabel": "设置分区"
    },
    "general": {
      "autostart": "开机自启",
      "autostartUnavailable": "当前平台不可用",
      "autostartReadFailed": "无法读取开机自启状态：{detail}",
      "autostartHint": "登录系统时自动运行启动器",
      "toggles": {
        "closeToTray": "关闭时最小化到托盘",
        "closeToTrayHint": "点击关闭按钮时隐藏主窗口到系统托盘，保持后台运行",
        "autostartMinimizeToTray": "开机启动后最小化到托盘",
        "autostartMinimizeToTrayHint": "通过开机自启启动时不弹出主窗口，直接在系统托盘后台运行",
        "checkInstanceUpdate": "检查实例更新",
        "checkInstanceUpdateHint": "自动检测已创建的实例是否有可用的 AstrBot 新版本，并在实例列表提供升级入口",
        "persistInstanceState": "记住实例运行状态",
        "persistInstanceStateHint": "退出启动器时记录正在运行的实例，并在下次启动时自动恢复运行",
        "ignoreExternalPath": "忽略外部路径",
        "ignoreExternalPathHint": "运行实例与安装依赖时仅使用启动器内置组件，不继承系统 PATH 环境变量",
        "useShortInstanceId": "使用短实例 ID",
        "useShortInstanceIdHint": "新建实例时使用 16 位短 ID 代替 UUID 作为目录名，缩短文件路径长度（仅对新建实例生效）",
        "lockCheckExtensionWhitelist": "文件锁扩展名白名单",
        "lockCheckExtensionWhitelistHint": "检测文件占用时仅扫描代码、配置和数据库等关键文件，提升操作前检查速度（仅 Windows）",
        "useUvForDeps": "使用 uv 安装依赖",
        "useUvForDepsHint": "部署和启动实例时使用 uv 代替 pip 同步 Python 依赖以加快安装速度（需先安装 uv 组件）"
      }
    },
    "tray": {
      "style": "托盘样式",
      "panel": "面板",
      "nativeMenu": "菜单"
    },
    "network": {
      "githubProxy": "GitHub 代理",
      "httpProxy": "HTTP 代理",
      "fields": {
        "url": "地址",
        "port": "端口",
        "username": "用户名",
        "password": "密码"
      },
      "saveHttpProxy": "保存 HTTP 代理",
      "pypiMirror": "PyPI 镜像",
      "nodejsMirror": "Node.js 镜像",
      "npmRegistry": "npm 镜像源",
      "pythonMirror": "Python 运行时镜像",
      "uvMirror": "uv 镜像",
      "allowLocalDnsProbe": "允许本地 DNS 重定向探测",
      "allowLocalDnsProbeHint": "仅影响网络源连通性测试，允许解析到本地或私有地址的域名；直连本地地址和 localhost 仍会被拒绝。",
      "sources": {
        "default": "默认源",
        "followGithub": "跟随 GitHub 代理",
        "custom": "自定义源",
        "customPlaceholder": "https://example.com/",
        "test": "测速",
        "testing": "测试中 {done}/{total}",
        "testHint": "点击：串行测试全部源。\nCtrl+点击：仅测试当前选中源；Shift+点击：并行测试全部源。",
        "testHttp": "测试代理",
        "httpTestHint": "使用当前表单内容测试 HTTP 代理的延迟和下载速度",
        "latency": "延迟",
        "speed": "速度",
        "unavailable": "不可用",
        "failed": "失败",
        "invalidUrl": "请输入有效的 http(s) 地址",
        "tuna": "清华大学 TUNA",
        "ustc": "中国科学技术大学",
        "aliyun": "阿里云",
        "cernet": "CERNET",
        "nju": "南京大学",
        "npmmirror": "npmmirror",
        "tencent": "腾讯云",
        "huawei": "华为云"
      }
    },
    "data": {
      "dataDirectory": "数据目录",
      "dataDirectoryHint": "存放实例数据、已下载版本、运行组件和备份文件的根目录",
      "changeDirectory": "更改数据目录",
      "change": "更改",
      "changeDirectoryMessage": "新目录：{path}\n重启启动器后生效。",
      "restartNotice": "数据目录已更改，重启启动器后生效。",
      "rebuildManifest": "重建实例清单",
      "rebuildManifestHint": "扫描当前数据目录并重新生成实例与已安装版本记录",
      "rebuildManifestMessage": "将根据磁盘内容重建实例清单。",
      "rebuild": "重建",
      "rebuildComplete": "已重建 {instances} 个实例，{versions} 个版本",
      "uvCache": "uv 缓存",
      "uvCacheHint": "清理 uv 下载和构建 Python 包时产生的本地缓存文件",
      "clearUvCacheAria": "清理 uv 缓存",
      "clear": "清理",
      "uvCacheCleared": "uv 缓存已清理"
    },
    "about": {
      "appName": "应用名称",
      "version": "版本",
      "launcherRepoDescription": "AstrBot 桌面启动器主仓库。",
      "currentRepoDescription": "当前桌面端仓库，基于 Tauri 2 与 Vue 3 实现。",
      "astrbotRepoDescription": "AstrBot 核心项目上游仓库。",
      "openLauncherRepo": "打开 astrbot-launcher 主仓库",
      "openCurrentRepo": "打开 astrbot-go 当前仓库",
      "openAstrbotRepo": "打开 AstrBot 仓库",
      "openRepository": "打开仓库",
      "openLinkFailed": "无法打开链接：{detail}",
      "updater": {
        "title": "启动器更新",
        "check": "检查更新",
        "checking": "正在检查更新…",
        "unsupported": "此平台暂不支持自动更新",
        "upToDate": "已是最新版本",
        "available": "发现新版本 {version}",
        "download": "下载更新",
        "downloading": "正在下载更新…",
        "downloaded": "更新已下载完成",
        "installAndRestart": "重启安装",
        "installing": "正在准备安装…",
        "idle": "尚未检查更新",
        "autoCheck": "自动检查更新",
        "autoCheckHint": "启动时检查启动器是否有新版本",
        "autoUpdate": "自动更新",
        "autoUpdateHint": "发现新版本后自动下载并重启安装；有实例运行时改为提示更新",
        "autoUpdateRequiresCheck": "需先开启自动检查更新"
      },
    }
  },
  "errors": {
    "instanceNotFound": "实例不存在",
    "instanceRunning": "实例正在运行，请先停止",
    "instanceNotRunning": "实例未运行",
    "versionNotFound": "版本不存在或未安装",
    "versionInUse": "版本正被实例使用",
    "versionInUseWithInstance": "版本正被实例“{instance}”使用",
    "downloadCancelled": "下载已取消",
    "configSaveFailed": "配置保存失败",
    "configSaveFailedWithDetail": "配置保存失败：{detail}",
    "fileOperationFailed": "文件操作失败",
    "fileOperationFailedWithDetail": "文件操作失败：{detail}",
    "networkRequestFailed": "网络请求失败",
    "networkRequestFailedWithDetail": "网络请求失败：{detail}",
    "pythonRuntimeError": "Python 运行环境错误",
    "pythonRuntimeErrorWithDetail": "Python 运行环境错误：{detail}",
    "pythonNotInstalled": "Python 运行环境未安装",
    "processOperationFailed": "进程操作失败",
    "processOperationFailedWithDetail": "进程操作失败：{detail}",
    "portOccupied": "端口已被占用",
    "portOccupiedWithPort": "端口 {port} 已被占用",
    "startupTimeout": "实例启动超时",
    "invalidHost": "主机地址无效",
    "invalidHostWithDetail": "主机地址无效：{detail}",
    "backupFailed": "备份操作失败",
    "backupFailedWithDetail": "备份操作失败：{detail}",
    "githubFailed": "GitHub 请求失败",
    "updateFailed": "启动器更新失败，请稍后重试",
    "operationFailed": "操作失败",
    "operationFailedWithDetail": "操作失败：{detail}",
    "lock": {
      "title": "文件被占用",
      "detailLocked": "目标路径被占用，请关闭相关进程后重试",
      "detailCheckFailed": "目标路径占用状态检测失败",
      "detailMaxSessions": "系统 Restart Manager 会话数已达上限，无法检测占用状态",
      "continueAnyway": "仍然继续"
    }
  },
  "dialogs": {
    "pickDirectoryTitle": "选择数据目录"
  },
  "versions": {
    "title": "版本",
    "loading": "正在加载…",
    "sections": {
      "components": "组件管理",
      "installed": "已安装",
      "available": "可下载"
    },
    "components": {
      "empty": "暂无组件信息"
    },
    "status": {
      "installed": "已安装",
      "notInstalled": "未安装",
      "versionUnknown": "未知",
      "prerelease": "预发行"
    },
    "installed": {
      "empty": "暂无已安装版本",
      "unused": "未被实例使用"
    },
    "releases": {
      "loading": "正在获取版本列表…",
      "empty": "暂无可下载版本",
      "allInstalled": "所有版本均已安装"
    },
    "actions": {
      "download": "下载",
      "reinstall": "重新下载",
      "clearUvCache": "清理 uv 缓存",
      "openFolder": "打开所在文件夹",
      "details": "详情",
      "resume": "继续",
      "pause": "暂停",
      "stop": "停止"
    },
    "confirm": {
      "titles": {
        "reinstallComponent": "重新下载组件",
        "uninstallComponent": "卸载组件",
        "uninstallVersion": "卸载版本"
      },
      "messages": {
        "reinstallComponent": "将重新下载 {name}，现有安装会被覆盖。",
        "uninstallComponent": "确定要卸载 {name} 吗？",
        "uninstallVersion": "确定要卸载版本 {version} 吗？正在使用该版本的实例将无法启动。"
      }
    },
    "toasts": {
      "uvCacheCleared": "已清理 uv 缓存",
      "versionUninstalled": "已卸载 {version}",
      "versionInstalled": "已安装 {version}",
      "downloadStopped": "已停止下载 {version}"
    },
    "releaseNotes": {
      "title": "版本说明",
      "empty": "暂无版本说明"
    }
  },
  "components": {
    "name": {
      "python": "Python",
      "nodejs": "Node.js (LTS)",
      "uv": "uv"
    },
    "description": {
      "python": "Python 3.10 / 3.12 运行时",
      "nodejs": "Node.js 运行时",
      "uv": "uv / uvx 包管理工具"
    },
    "result": {
      "alreadyInstalled": "{name} 已安装",
      "installed": "已安装 {name}：{versions}",
      "installedWithoutVersions": "{name} 已安装",
      "uninstalled": "已卸载 {name}：{versions}",
      "uninstalledWithoutVersions": "已卸载 {name}",
      "notInstalled": "{name} 组件未安装",
      "reinstalled": "已重新安装 {name}：{versions}",
      "reinstalledWithoutVersions": "已重新安装 {name}"
    }
  },
  "progress": {
    "deploy": {
      "extractingCode": "正在解压代码…",
      "codeExtracted": "代码解压完成",
      "creatingVenv": "正在创建虚拟环境…",
      "venvCreated": "虚拟环境创建完成",
      "installingDeps": "正在安装依赖…",
      "depsInstalled": "依赖安装完成",
      "preparingRepair": "正在准备修复实例…",
      "restoringPreservedData": "正在恢复保留数据…",
      "preservedDataRestored": "保留数据恢复完成",
      "repairDone": "实例修复完成",
      "downloadingWebui": "正在下载 WebUI…",
      "extractingWebui": "正在解压 WebUI…",
      "webuiReplaced": "WebUI 替换完成",
      "startingInstance": "正在启动实例…",
      "instanceStarted": "实例已启动",
      "backingUpData": "正在备份数据…",
      "dataBackupDone": "数据备份完成",
      "restoringData": "正在还原数据…",
      "dataRestored": "数据还原完成",
      "updateDone": "更新完成"
    },
    "download": {
      "resuming": "继续下载",
      "cancelled": "下载已取消",
      "paused": "已暂停",
      "accelerationFallback": "加速下载失败，正在回退直连",
      "starting": "开始下载",
      "downloading": "下载中",
      "completed": "下载完成",
      "extracting": "正在解压",
      "installed": "安装完成"
    }
  },
  "tasks": {
    "tabs": {
      "history": "历史记录",
      "scheduled": "定时任务"
    },
    "scheduled": {
      "actions": {
        "create": "新建任务",
        "edit": "编辑",
        "runNow": "立即运行",
        "remove": "删除"
      },
      "fields": {
        "name": "名称",
        "instance": "实例",
        "action": "执行操作",
        "schedule": "调度",
        "nextRun": "下次运行",
        "lastRun": "上次运行"
      },
      "jobActions": {
        "startInstance": "启动实例",
        "stopInstance": "停止实例",
        "backup": "创建备份",
        "clearCache": "清除 Python 缓存"
      },
      "modes": {
        "once": "一次性",
        "interval": "间隔",
        "daily": "每天",
        "weekly": "每周"
      },
      "units": {
        "minutes": "分钟",
        "hours": "小时"
      },
      "weekdays": {
        "sun": "周日",
        "mon": "周一",
        "tue": "周二",
        "wed": "周三",
        "thu": "周四",
        "fri": "周五",
        "sat": "周六"
      },
      "desc": {
        "once": "{time} 执行一次",
        "interval": "每 {count} {unit}",
        "daily": "每天 {time}",
        "weekly": "每{weekday} {time}"
      },
      "dialog": {
        "createTitle": "新建定时任务",
        "editTitle": "编辑定时任务",
        "dateTime": "执行时间",
        "time": "执行时间",
        "intervalCount": "间隔",
        "intervalUnit": "单位",
        "weekday": "星期",
        "timeInPast": "执行时间必须晚于当前时间",
        "noInstances": "暂无实例，请先创建实例"
      },
      "result": {
        "never": "尚未运行",
        "succeeded": "成功",
        "failed": "失败"
      },
      "empty": "暂无定时任务",
      "hint": "仅在应用运行时执行；备份与清除缓存需实例处于停止状态",
      "confirm": {
        "title": "删除定时任务",
        "message": "将删除“{name}”，此操作不可撤销。"
      }
    },
    "filters": {
      "all": "全部",
      "download": "下载",
      "deploy": "部署",
      "backup": "备份",
      "repair": "修复"
    },
    "actions": {
      "clearFinished": "清除记录",
      "pause": "暂停任务",
      "resume": "继续任务",
      "cancel": "取消任务",
      "remove": "删除记录"
    },
    "fields": {
      "targetValue": "目标：{target}"
    },
    "empty": "暂无任务记录",
    "titles": {
      "downloadVersion": "下载版本",
      "deployInstance": "部署实例",
      "createBackup": "创建备份",
      "restoreBackup": "还原备份",
      "repairInstance": "修复实例"
    },
    "status": {
      "queued": "排队中",
      "running": "运行中",
      "paused": "已暂停",
      "cancelling": "正在取消",
      "cancelled": "已取消",
      "succeeded": "已完成",
      "failed": "失败",
      "interrupted": "已中断"
    },
    "errors": {
      "other": "操作失败",
      "config": "配置操作失败",
      "io": "文件操作失败",
      "network": "网络操作失败",
      "python": "Python 运行环境异常",
      "process": "进程操作失败",
      "process_locking": "目标文件被占用",
      "backup": "备份操作失败",
      "github": "GitHub 请求失败",
      "instance_not_found": "实例不存在",
      "instance_running": "实例正在运行",
      "instance_not_running": "实例未运行",
      "version_not_found": "版本不存在",
      "version_in_use": "版本正被实例使用",
      "python_not_installed": "Python 运行环境未安装",
      "port_occupied": "端口已被占用",
      "invalid_host": "主机地址无效",
      "startup_timeout": "实例启动超时",
      "download_cancelled": "下载已取消",
      "launcher_update": "启动器更新失败"
    }
  },
  "shell": {
    "nav": {
      "chat": "聊天",
      "overview": "概览",
      "instances": "实例",
      "versions": "版本",
      "backups": "备份",
      "tasks": "任务",
      "logs": "日志",
      "extensions": "扩展",
      "settings": "设置"
    },
    "sidebar": {
      "expand": "展开侧边栏",
      "collapse": "收起侧边栏"
    },
    "titleBar": {
      "label": "应用标题栏",
      "minimize": "最小化",
      "maximize": "最大化",
      "restore": "还原",
      "close": "关闭"
    },
    "tabs": {
      "label": "标签页",
      "home": "首页",
      "close": "关闭 {name}"
    },
    "main": {
      "openMenu": "打开菜单",
      "home": "首页"
    }
  },
  "overview": {
    "loading": "正在加载…",
    "stats": {
      "ariaLabel": "{label}：{value}",
      "instances": "实例总数",
      "running": "运行中",
      "versions": "已安装版本",
      "backups": "备份数量",
      "components": "运行组件"
    },
    "sections": {
      "instances": "实例",
      "backups": "备份"
    },
    "empty": {
      "instances": "暂无实例",
      "backups": "暂无备份",
      "components": "暂无组件"
    },
    "instances": {
      "listLabel": "实例列表",
      "donutAria": "实例状态分布：{running} {runningCount} 个，{starting} {startingCount} 个，{stopping} {stoppingCount} 个，{stopped} {stoppedCount} 个",
      "rowLabel": "{name}，{state}，版本 {version}，端口 {port}",
      "unknownVersion": "未知",
      "port": "端口 {port}"
    },
    "backups": {
      "rangeLabel": "备份统计范围",
      "rangeMonths": "{count} 个月",
      "series": "备份数量",
      "tooltip": "{label}：{value} 个",
      "chartAria": "近 {range} 个月备份数量，共 {total} 个",
      "composition": {
        "title": "备份构成",
        "manual": "手动",
        "auto": "自动",
        "corrupted": "损坏"
      },
      "recentTitle": "最近备份"
    },
    "components": {
      "title": "运行环境",
      "installed": "已安装",
      "notInstalled": "未安装"
    }
  },
  "instances": {
    "title": "实例",
    "loading": "正在加载…",
    "actions": {
      "create": "创建实例",
      "start": "启动",
      "portConflict": "端口 {port} 已被其他运行中的实例占用，无法启动",
      "stop": "停止",
      "restart": "重启",
      "openCoreFolder": "打开核心目录",
      "createBackup": "创建备份",
      "repair": "修复实例",
      "viewLogs": "查看日志",
      "openWebui": "打开 WebUI",
      "openInBrowser": "在浏览器中打开",
      "clearData": "清除数据",
      "clearVenv": "清除虚拟环境",
      "clearPycache": "清除 Python 缓存",
      "deleteInstance": "删除实例",
      "upgrade": "升级",
      "upgradeAll": "全部升级",
      "healthCheck": "健康诊断",
      "manageData": "管理数据"
    },
    "health": {
      "title": "{instance} · 实例体检",
      "grades": {
        "healthy": "状态良好",
        "warning": "需要关注",
        "critical": "发现问题"
      },
      "status": {
        "pass": "通过",
        "warning": "警告",
        "fail": "失败",
        "skipped": "未检查"
      },
      "checks": {
        "python": "Python 运行时",
        "venv": "虚拟环境",
        "dependencies": "依赖",
        "port": "端口",
        "disk_space": "磁盘空间",
        "configuration": "配置解析",
        "webui": "WebUI 连通性"
      },
      "details": {
        "runtime_ready": "Python {value} 可用。",
        "runtime_missing": "未找到此实例版本所需的 Python 运行时。",
        "runtime_failed": "Python 运行时无法正常启动。",
        "venv_ready": "虚拟环境中的 Python {value} 可用。",
        "venv_missing": "未找到完整的虚拟环境。",
        "venv_failed": "虚拟环境中的 Python 无法正常启动。",
        "dependencies_ready": "依赖检查通过。",
        "dependencies_issues": "发现 {value} 项依赖问题。",
        "dependencies_check_failed": "无法完成依赖检查。",
        "requirements_missing": "核心目录缺少 requirements.txt。",
        "pip_unavailable": "虚拟环境未安装 pip，无法核验依赖。",
        "venv_required": "需要先创建虚拟环境。",
        "dependencies_timeout": "依赖检查超时，请稍后重试。",
        "port_available": "端口 {value} 可用。",
        "port_occupied": "端口 {value} 已被占用。",
        "port_listening": "端口 {value} 正在监听。",
        "port_unreachable": "无法连接到端口 {value}。",
        "port_invalid": "实例端口无效。",
        "port_check_failed": "无法检查实例端口 {value}。",
        "space_sufficient": "可用空间：{value}",
        "space_low": "可用空间偏低：{value}",
        "space_critical": "可用空间不足：{value}",
        "space_unavailable": "无法获取磁盘剩余空间。",
        "config_valid": "YAML 与 JSON 配置均可解析。",
        "config_invalid": "配置文件缺失或格式无效。",
        "config_missing": "尚未生成配置文件。",
        "webui_reachable": "WebUI 已响应（HTTP {value}）。",
        "webui_unreachable": "无法连接到 WebUI。",
        "webui_not_running": "实例未运行，已跳过连通性检查。",
        "webui_disabled": "WebUI 已禁用。",
        "webui_host_unsupported": "仅检查本机地址，已跳过非本机 WebUI 地址。"
      }
    },
    "list": {
      "columns": {
        "name": "名称",
        "state": "状态",
        "actions": "操作"
      },
      "meta": "版本 {version} · 端口 {port}",
      "empty": "暂无实例",
      "pidWarning": "WebUI 未启用，无法跟踪进程状态",
      "lastExit": "上次退出：{reason}",
      "restartAttempts": "已自动重启 {attempt}/{max} 次",
      "aria": {
        "moreActions": "更多操作",
        "pidUnreliable": "进程状态可能不准确",
        "lastExit": "异常退出信息"
      }
    },
    "updates": {
      "stopFirst": "请先停止实例再升级。",
      "batchPaused": "批量升级已暂停，请处理问题后重试。",
      "batchComplete": "已升级 {count} 个实例",
      "confirm": {
        "title": "全部升级",
        "message": "将把 {count} 个实例升级到 {version}，升级会自动备份实例数据。",
        "skipped": "有 {count} 个实例未停止，已跳过。"
      }
    },
    "confirm": {
      "titles": {
        "delete": "删除实例",
        "backup": "创建备份",
        "clearData": "清除数据",
        "clearVenv": "清除虚拟环境",
        "clearPycache": "清除 Python 缓存",
        "upgrade": "变更版本"
      },
      "messages": {
        "delete": "确定删除实例“{name}”？实例数据将被移除，此操作不可恢复。",
        "backup": "将为实例“{name}”创建备份。",
        "clearData": "将删除实例“{name}”的数据目录，此操作不可恢复。",
        "clearVenv": "将删除实例“{name}”的虚拟环境，下次启动会重新安装依赖。",
        "clearPycache": "将删除实例“{name}”的 Python 字节码缓存。",
        "upgrade": "将把实例“{name}”切换到 {version}，升级会重新部署并自动备份数据。"
      },
      "labels": {
        "clear": "清除",
        "continue": "继续"
      }
    },
    "toasts": {
      "deleted": "实例已删除",
      "backupCreated": "备份已创建",
      "updated": "实例已更新",
      "upgraded": "实例已升级到 {version}",
      "busy": "操作进行中，请稍候",
      "created": "实例已创建",
      "missing": "实例不存在",
      "dataCleared": "数据已清除",
      "venvCleared": "虚拟环境已清除",
      "pycacheCleared": "Python 缓存已清除",
      "repairComplete": "修复完成",
      "started": "实例“{name}”已启动",
      "stopped": "实例已停止"
    },
    "deploy": {
      "title": "实例部署"
    },
    "restart": {
      "scheduled": "实例“{name}”异常退出，{seconds} 秒后自动重启（第 {attempt}/{max} 次）",
      "exhausted": "实例“{name}”多次异常退出，已停止自动重启"
    },
    "form": {
      "titles": {
        "edit": "编辑实例",
        "create": "创建实例"
      },
      "fields": {
        "name": "名称",
        "version": "版本",
        "host": "主机",
        "port": "端口",
        "checkUpdate": "检查更新",
        "autoRestart": "异常退出后自动重启",
        "autoRestartMaxRetries": "最大重试次数",
        "autoRestartNotify": "重启时发送通知",
        "dashboardUsername": "WebUI 用户名",
        "dashboardPassword": "WebUI 密码"
      },
      "emptyVersions": "暂无已安装版本",
      "goVersions": "前往版本页",
      "dashboardConfigured": "已配置，留空保持不变",
      "dashboardHint": "使用 AstrBot WebUI 的登录账号。",
      "dashboardClear": "清除已保存的登录信息",
      "dashboardKeep": "保留已保存的登录信息",
      "dashboardClearPending": "保存后将清除已保存的登录信息。",
      "validation": {
        "nameRequired": "请输入名称",
        "versionRequired": "请选择版本",
        "portRange": "端口需为 {min} 至 {max} 的整数",
        "portInUse": "端口已被占用"
      }
    },
    "repair": {
      "title": "修复实例",
      "hint": "选择保留范围，实例将重新部署。",
      "scopes": {
        "dataDirectory": "保留数据目录",
        "configAndDataFiles": "保留配置与数据库",
        "coreConfigAndDataFiles": "保留核心配置与数据库",
        "databaseOnly": "仅保留数据库"
      },
      "submit": "修复"
    },
    "credentials": {
      "title": "默认账号",
      "instance": "实例",
      "username": "用户名",
      "password": "密码",
      "copied": "已复制",
      "copyFailed": "复制失败",
      "aria": {
        "copyUsername": "复制用户名",
        "hidePassword": "隐藏密码",
        "showPassword": "显示密码",
        "copyPassword": "复制密码"
      }
    },
    "webui": {
      "aria": {
        "refresh": "刷新 WebUI"
      },
      "starting": "启动中…",
      "stopping": "停止中…",
      "loadTimeout": "WebUI 加载超时",
      "disabled": "WebUI 未启用",
      "notRunning": "实例未运行"
    }
  },
  "resources": {
    "title": "资源监控",
    "open": "查看资源监控",
    "stoppedHint": "实例运行后可采集资源数据。",
    "loadFailed": "数据获取失败",
    "metrics": {
      "cpu": "CPU",
      "memory": "内存",
      "disk": "磁盘"
    },
    "charts": {
      "cpu": "CPU 趋势",
      "storage": "内存与磁盘趋势",
      "empty": "暂无采样"
    },
    "thresholds": {
      "title": "阈值告警",
      "enabled": "启用告警",
      "cpu": "CPU 上限",
      "memory": "内存上限",
      "disk": "磁盘上限",
      "invalid": "请输入大于 0 的数值。",
      "saved": "阈值已保存"
    },
    "alerts": {
      "breached": "资源告警：{name} 的{metric}达到 {value}",
      "current": "当前超过阈值：{metrics}",
      "separator": "，"
    },
    "units": {
      "percent": "%",
      "mib": "MB",
      "gib": "GB"
    }
  },
  "backups": {
    "title": "备份",
    "createBackup": "创建备份",
    "createDialog": {
      "title": "创建备份",
      "instanceLabel": "实例",
      "noteLabel": "备注",
      "notePlaceholder": "可选",
      "noteTooLong": "备注不能超过 {max} 个字符。",
      "noInstances": "暂无实例",
      "noStoppedInstances": "没有已停止的实例"
    },
    "instanceFilter": "实例",
    "allInstances": "全部实例",
    "searchPlaceholder": "搜索文件名或备注",
    "loading": "正在加载…",
    "empty": "暂无备份",
    "noMatches": "没有符合条件的备份",
    "autoGenerated": "自动创建",
    "corrupted": "已损坏",
    "corruptedName": "（损坏备份）",
    "restore": "恢复",
    "restoreTitle": "恢复备份",
    "restoreMessage": "恢复将覆盖实例“{name}”的当前数据。",
    "restoreMessageGeneric": "恢复将覆盖目标实例的当前数据。",
    "deleteTitle": "删除备份",
    "deleteMessage": "确定删除备份“{filename}”吗？删除后无法恢复。",
    "created": "备份已创建",
    "restored": "备份已恢复",
    "deleted": "备份已删除",
    "openFolder": "打开所在文件夹",
    "headers": {
      "instanceName": "实例名称",
      "version": "版本",
      "createdAt": "创建时间",
      "note": "备注",
      "actions": "操作"
    }
  },
  "logs": {
    "title": "日志",
    "empty": "暂无日志",
    "sources": {
      "all": "全部来源",
      "system": "系统"
    },
    "levels": {
      "debug": "调试",
      "info": "信息",
      "warn": "警告",
      "error": "错误"
    },
    "sourceLabel": "来源",
    "sourceFilter": "来源筛选",
    "levelFilter": "级别筛选",
    "search": "搜索",
    "searchAria": "搜索日志",
    "autoScroll": "自动滚动",
    "fontSize": "字号",
    "decreaseFontSize": "减小字号",
    "increaseFontSize": "增大字号",
    "currentFontSize": "当前字号",
    "copyLogs": "复制日志",
    "clear": "清空",
    "clearLogs": "清空日志",
    "copied": "已复制",
    "copyFailed": "复制失败",
    "loadFailed": "日志订阅失败",
    "jumpToLatest": "回到最新",
    "jumpToLatestAria": "回到最新日志"
  },
  "instanceData": {
    "title": "数据管理",
    "saved": "已保存",
    "v4Unsupported": "实例数据仅支持 AstrBot v4 及以上版本。",
    "readonlyTooltip": "实例未停止，无法保存更改。",
    "editConfigFile": "编辑配置文件",
    "searchPlaceholder": "搜索",
    "actionsColumn": "操作",
    "all": "全部",
    "none": "无",
    "unsavedTitle": "未保存的更改",
    "unsavedMessage": "有未保存的更改，关闭后将丢弃。",
    "sections": {
      "personas": "人格",
      "cronJobs": "定时任务",
      "mcpServers": "MCP 服务器",
      "configFiles": "配置文件",
      "knowledgeBases": "知识库"
    },
    "personas": {
      "empty": "暂无人格",
      "addTitle": "新增人格",
      "editTitle": "编辑人格",
      "idRequired": "ID 不能为空。",
      "idExists": "ID 已存在。",
      "columns": {
        "personaId": "ID",
        "folder": "文件夹",
        "systemPrompt": "系统提示词",
        "tools": "工具",
        "skills": "技能"
      },
      "fields": {
        "personaId": "ID",
        "systemPrompt": "系统提示词",
        "beginDialogs": "开场对话",
        "beginDialogsPlaceholder": "每行一条",
        "tools": "工具",
        "skills": "技能",
        "customErrorMessage": "自定义错误消息",
        "listPlaceholder": "逗号或换行分隔，留空为全部",
        "folder": "所属文件夹",
        "rootFolder": "根目录",
        "sortOrder": "排序"
      }
    },
    "cronJobs": {
      "empty": "暂无定时任务",
      "addTitle": "新增定时任务",
      "editTitle": "编辑定时任务",
      "nameRequired": "名称不能为空。",
      "cronRequired": "请填写 Cron 表达式。",
      "payloadInvalid": "Payload 必须是 JSON 对象。",
      "columns": {
        "name": "名称",
        "jobType": "类型",
        "cronExpression": "Cron 表达式",
        "enabled": "启用",
        "nextRun": "下次运行"
      },
      "jobTypes": {
        "basic": "基础",
        "activeAgent": "主动智能体"
      },
      "fields": {
        "name": "名称",
        "description": "描述",
        "jobType": "类型",
        "cronExpression": "Cron 表达式",
        "timezone": "时区",
        "payload": "Payload",
        "enabled": "启用",
        "persistent": "持久化",
        "runOnce": "仅运行一次",
        "runAt": "运行时间",
        "prompt": "提示词"
      }
    },
    "mcpServers": {
      "empty": "暂无 MCP 服务器",
      "parseError": "mcp_server.json 不是有效的 MCP 配置。",
      "fileMissing": "mcp_server.json 尚未生成，实例首次运行后自动创建。",
      "addServer": "添加服务器",
      "removeServer": "删除服务器",
      "removeMessage": "确定删除服务器“{name}”吗？",
      "newServerName": "服务器名称",
      "duplicateName": "名称已存在。",
      "fields": {
        "name": "名称",
        "command": "命令",
        "args": "参数",
        "url": "URL",
        "active": "启用",
        "env": "环境变量"
      }
    },
    "knowledgeBases": {
      "empty": "暂无知识库",
      "emptyHint": "知识库与文档请在实例 WebUI 中添加。",
      "deleteTitle": "删除知识库",
      "deleteMessage": "确定删除知识库“{name}”吗？该操作立即生效。",
      "deleted": "已删除",
      "columns": {
        "name": "名称",
        "description": "描述",
        "docs": "文档数",
        "chunks": "分块数",
        "size": "大小"
      }
    },
    "configFiles": {
      "empty": "暂无配置文件",
      "jsonInvalid": "JSON 格式无效。"
    },
    "copy": {
      "title": "复制到实例",
      "targets": "目标实例",
      "overwrite": "覆盖同名项",
      "noTargets": "没有已停止的其他实例。",
      "copied": "已复制"
    }
  },
  "extensions": {
    "tabs": {
      "plugins": "插件",
      "skills": "技能"
    },
    "running": "运行中"
  },
  "plugins": {
    "instance": "目标实例",
    "tabs": {
      "market": "插件市场",
      "installed": "已安装"
    },
    "search": "搜索插件",
    "category": "分类",
    "allCategories": "全部分类",
    "official": "官方",
    "installed": "已安装",
    "downloads": "下载 {count}",
    "stars": "Star {count}",
    "requiresVersion": "要求的 AstrBot 版本",
    "repo": "仓库",
    "openRepo": "打开插件仓库",
    "install": "安装",
    "loadMore": "加载更多",
    "update": "检查更新",
    "uninstall": "卸载",
    "toggleAria": "启用或停用 {name}",
    "instanceRunning": "实例正在运行，安装、卸载与启停需先停止实例。",
    "noInstance": "请先选择实例。",
    "market": {
      "empty": "没有匹配的插件。"
    },
    "installedList": {
      "empty": "该实例还没有安装插件。"
    },
    "installConfirm": {
      "title": "安装插件",
      "pickInstance": "选择安装 {name} 的目标实例："
    },
    "uninstallConfirm": {
      "title": "卸载插件",
      "message": "卸载 {name}？",
      "deleteConfig": "同时删除插件配置文件",
      "deleteData": "同时删除插件持久化数据"
    },
    "toasts": {
      "installed": "已安装 {name}。",
      "updated": "已将 {name} 更新到 {version}。",
      "uninstalled": "已卸载 {name}。",
      "upToDate": "已是最新版本 {version}。",
      "marketMissing": "未在插件市场找到该插件的记录。"
    }
  },
  "skills": {
    "instance": "目标实例",
    "upload": "上传技能",
    "localTitle": "本地技能",
    "instanceRunning": "实例正在运行，上传、编辑与启停需先停止实例。",
    "noInstance": "请先选择实例。",
    "empty": "该实例还没有本地技能。",
    "edit": "编辑",
    "toggleAria": "启用或停用 {name}",
    "table": {
      "name": "名称",
      "description": "描述",
      "enabled": "启停"
    },
    "pluginSkills": {
      "title": "插件技能",
      "plugin": "所属插件",
      "view": "查看",
      "empty": "已安装的插件没有提供技能。"
    },
    "uploadDialog": {
      "title": "上传技能",
      "pickFile": "选择 ZIP 文件",
      "noFile": "未选择文件"
    },
    "deleteConfirm": {
      "title": "删除技能",
      "message": "删除技能 {name}？"
    },
    "editor": {
      "label": "SKILL.md 内容"
    },
    "toasts": {
      "installed": "已安装技能：{names}。",
      "saved": "已保存 {name}。",
      "deleted": "已删除 {name}。"
    }
  }
};

export type MessageSchema = typeof zhCN;

export default zhCN;
