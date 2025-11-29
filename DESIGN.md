# Layered VHDX System Manager — 设计文档

## 目标与范围
- 在单机 Windows 10/11（管理员）环境中，提供基于 VHDX 差分链的分层系统管理。
- 提供树状可视化、节点创建、引导记录生成、设置下次启动并立即重启、差分链删除、挂载/修复等能力。
- 仅依赖系统原生命令：`diskpart`、`dism`、`bcdboot`、`bcdedit`（无 Hyper-V cmdlet）。
- 所有 VHDX 置于用户指定的单一根目录，工具自身便携；BCD 绑定当前机器（便携受此限制）。

## 目录结构与命名
- 根目录由用户选择，可在任何本地/可移动路径。
- 固定子目录：
  - `base/`：基础盘。
  - `diff/`：差分盘。
  - `meta/`：`state.db`（SQLite）、日志、临时脚本、挂载点目录、国际化资源。
  - `meta/tmp/`：临时 diskpart 脚本等。
  - `meta/locales/`：i18n 资源（默认简体中文，可扩展）。
- VHDX 命名：`<seq>-<slug>.vhdx`，`seq` 来自 DB 自增（4 位以上，例 `0001-base.vhdx`）；`slug` 为人类可读标签。真实关系由 DB 维护，不依赖命名。

## 数据模型（SQLite）
- `nodes(id TEXT PRIMARY KEY, parent_id TEXT, name TEXT, path TEXT, bcd_guid TEXT, desc TEXT, created_at DATETIME, status TEXT, boot_files_ready BOOLEAN)`
  - `status`：normal / missing_file / missing_parent / missing_bcd / mounted / error 等。
  - `boot_files_ready`：标记是否已写入引导文件。
- `ops(id TEXT PRIMARY KEY, node_id TEXT, ts DATETIME, action TEXT, result TEXT, detail TEXT)`：操作历史。
- `settings(root_path TEXT, locale TEXT, seq_counter INTEGER, last_boot_guid TEXT, ...)`。

## 核心流程
### 启动/扫描
1) 读取 DB，构建内存树。
2) 校验文件存在性；用 `diskpart detail vdisk` 解析父链，验证与 DB 一致。
3) `bcdedit /enum all` 检查 `bcd_guid` 是否存在；缺失则标记待修复。
4) 汇总状态用于 UI 展示。

### 初始盘创建（无母盘时）
1) 用户选择根目录（若未选）和 WIM/ESD 文件；列出镜像 index 让用户选择。
2) 询问基础盘大小（默认场景可固定，例如 60–100GB）。
3) 生成文件名（`base/`），构造 diskpart 脚本：
   - `create vdisk file=<...> maximum=<size> type=expandable`
   - `attach vdisk`
   - `convert gpt`
   - `create partition efi size=100` -> `format fs=fat32 quick`
   - `create partition msr size=16`
   - `create partition primary` -> `format fs=ntfs quick`
4) 为 EFI 与系统分区分配临时目录挂载点（`meta/mnt-efi`, `meta/mnt-sys-<id>`）；若 bcdboot 不接受目录或卷 GUID，回退临时盘符。
5) `dism /Apply-Image /ImageFile:<wim> /Index:<n> /ApplyDir:<sys mount>`。
6) `bcdboot <sys mount>\\Windows /s <efi mount> /f UEFI`。
7) `bcdedit /enum all` 按 `device vhd=[...]` 匹配当前盘路径，解析 GUID -> `bcd_guid`。
8) 卸载目录/盘符、`detach vdisk`，写入 DB 与操作历史。

### 差分节点创建
1) 选择父节点 -> 生成文件名（`diff/`）与 `id`。
2) diskpart：`create vdisk file=<child> parent=<parent>`；`attach vdisk`。
3) 挂载系统分区到临时目录；EFI 分区复用父盘 EFI 或单独 EFI？（默认复用父链已有 EFI，使用相同 EFI 分区路径）。
4) `bcdboot <sys mount>\\Windows /s <efi mount> /f UEFI` 写入引导；解析新的 BCD GUID。
5) 记录节点（parent_id、path、bcd_guid、status=normal），写操作历史；`detach vdisk`。

### 设置下次启动并立即重启
1) `bcdedit /bootsequence {guid}`。
2) 写操作历史；`shutdown /r /t 0`。

### 级联删除（多级差分链）
1) DFS 收集子树（含自身），按层序删除；若任一节点失败，中断整体删除并标记失败节点。
2) 对每个节点：若附加则 `detach` -> `bcdedit /delete {guid}`（忽略缺失提示但记录）-> 删除 VHDX 文件 -> DB 删除记录，写操作历史。

### 修复/校验
- 缺失文件/父链：标记不可用，提供“移除记录”。
- 缺失 BCD：挂载系统分区，重跑 `bcdboot`，重新解析 GUID。
- 挂载/查看：`attach vdisk` + `list volume` 获取分区 GUID/盘符；优先目录挂载，操作结束后卸载。

## BCD 与挂载策略
- 优先使用卷 GUID 或目录挂载传递给 `bcdboot`：`bcdboot <sys>\\Windows /s \\?\\Volume{GUID}\\ /f UEFI` 或目录挂载路径；若失败回退临时盘符，操作完成即移除。
- 挂载点目录集中在 `meta/mnt-*`，确保操作后清理。
- BCD GUID 解析统一通过 `bcdedit /enum all` 输出匹配 `device vhd=[...]`（绝对路径）。

## 前端（Tauri UI）
- 左侧树：名称 + 状态图标（normal/missing/bcd-missing/mounted/error）。
- 右侧详情：路径、父链、BCD GUID、创建时间、描述、状态、操作历史（最近 N 条）。
- 主要操作：创建初始盘（选镜像+index）、创建子盘、挂载/卸载、设为下次启动并重启、修复、删除（提示将连带子级、失败则中断）、刷新。
- 国际化：默认简体中文，可切换其他语言；文案存 `meta/locales/`（如 zh-CN.json, en-US.json），前端用 i18n 库（如 i18next）。

## 后端（Rust/Tauri）
- Command 封装：`run_diskpart(script_path)`, `run_dism(args)`, `run_bcdboot(sys, efi)`, `run_bcdedit(args)`，统一返回 stdout/stderr/exit code。
- Diskpart 脚本生成器：根据场景生成脚本写入 `meta/tmp/<op>.txt`，执行后删除。
- 输出解析：`detail vdisk` 提取 Parent 路径；`list volume` 解析卷 GUID/盘符；`bcdedit /enum all` 解析 GUID。
- DB 事务：创建/删除节点时同时写 `nodes` 和 `ops`，更新 `seq_counter`。
- 权限检查：启动时检测管理员（若缺失则阻止危险操作并提示）。
- 日志：`meta/ops.log` 记录命令与结果；前端可展示最近操作。

## 运行与安全
- 所有系统调用捕获 stdout/stderr/exit code，错误气泡提示并写历史。
- 删除、重启等危险操作需 UI 二次确认。
- 写配置前做轻量备份（`state.db.bak`）。

## 验证与测试要点
- `bcdboot` 对卷 GUID/目录挂载的兼容性（若失败验证回退盘符方案）。
- 多级差分链创建/删除的完整性，删除失败时中断行为。
- 外置盘/移动路径下的路径处理（避免硬编码盘符）。
- 重启前确认 `bcdedit /bootsequence` 成功。
- 初始盘安装流程：wim/esd index 选择、dism 解包成功率、分区布局正确性。

## 非功能性
- 便携化：除 BCD 条目外，工具自身无全局写入；根目录可移动但 BCD 需重建。
- 性能：操作受限于磁盘与系统命令，UI 不做阻塞；长操作展示进度。
- 国际化：新增语言只需添加 locale 文件与菜单项。

