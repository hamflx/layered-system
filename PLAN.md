# 3 个里程碑计划

## 里程碑 1：项目骨架与基础设施
- 初始化 Tauri 工程（Rust 后端 + 前端框架），配置工作目录可选根路径输入，约定根目录子结构 `base/`、`diff/`、`meta/`、`meta/tmp/`、`meta/locales/`。
- 引入 SQLite（例如 rusqlite）并实现 DB 初始化/迁移：创建 `nodes`、`ops`、`settings` 表，`settings.seq_counter` 自增生成 `<seq>-<slug>.vhdx` 文件名。
- 后端基础模块：统一系统命令执行器（捕获 stdout/stderr/exit code），临时文件管理器（在 `meta/tmp/` 写入/清理 diskpart 脚本），路径工具（根目录绝对化、slug 生成、挂载点路径生成）。
- 权限检查：启动时检测管理员权限，缺失时前端警告并禁用高危操作。
- 日志与操作历史：在 `meta/ops.log` 追加命令与结果；`ops` 表写入操作事件。
- i18n 框架搭建：前端集成 i18n（如 i18next），加载 `meta/locales/` 语言包；预置 zh-CN/en-US 骨架。

## 里程碑 2：核心后端能力与基础 UI 流
- Diskpart/DISM/BCD 封装：
  - 生成并执行 diskpart 脚本：创建基础盘（GPT+EFI/MSR/主分区）、创建差分盘、attach/detach、list volume、detail vdisk；解析输出获取父链、卷 GUID/盘符。
  - DISM：列出 WIM/ESD index，Apply-Image 到指定挂载目录。
  - BCD：`bcdboot <sys>\Windows /s <efi> /f UEFI`，`bcdedit /enum all` 解析 device vhd 匹配 GUID，`/bootsequence`，`/delete`。
- 核心业务流程实现：
  - 启动扫描：读 DB -> 校验文件存在、父链一致性、BCD 存在性，标记状态。
  - 初始盘创建：选择 WIM/ESD+index，diskpart 创建/分区/格式化，目录挂载 EFI/系统分区（失败回退临时盘符），dism 应用，bcdboot 写引导，解析 GUID，记录节点与 ops，清理挂载并 detach。
  - 差分节点创建：diskpart create vdisk parent=... -> attach -> 挂载系统分区（复用父 EFI），bcdboot 写引导 -> 解析 GUID -> 入库 -> detach。
  - 设置下次启动并重启：bcdedit /bootsequence {guid} 成功后 shutdown /r /t 0，记录 ops。
  - 级联删除：DFS 收集子树，逐节点 detach（若需要）-> bcdedit /delete -> 删除文件 -> DB 删除，任一失败中断整棵删除。
  - 修复缺失 BCD：挂载系统分区，重跑 bcdboot，解析新 GUID，更新节点。
  - 挂载/卸载：attach vdisk，列卷获取系统/EFI 分区，创建目录挂载点；卸载清理挂载点与 detach。
- 前端基础界面：
  - 树形视图展示节点与状态；详情面板显示路径、父链、BCD GUID、时间、描述、状态、最近操作。
  - 操作入口：创建初始盘、创建子盘、挂载/卸载、设为下次启动并重启、修复、删除（提示连带子级）、刷新。
  - 表单/对话：WIM/ESD 选择、index 下拉、名称/描述输入、确认重启/删除。

## 里程碑 3：健壮性、体验与文档
- 兼容性与回退：验证 bcdboot 对卷 GUID/目录挂载的支持，封装自动回退到临时盘符并清理；外置盘/移动路径测试。
- 错误处理与提示：前端统一错误展示（含 stdout/stderr 摘要），危险操作二次确认，长耗时操作进度/状态更新。
- 状态与修复完善：更细的状态标识（missing_file/missing_parent/missing_bcd/mounted/error），修复向导（缺失 BCD/记录删除），操作历史过滤。
- 国际化完善：补全 UI 文案中英双语，语言切换设置写入 `settings.locale`。
- 文档与示例：更新 README/使用指南，说明目录结构、权限需求、常见问题；添加示例流程截图/描述。
- 稳定性验证：多级差分创建/删除回归，重启流程验证，安装流程全链路试跑；备份 `state.db` 前再写。
