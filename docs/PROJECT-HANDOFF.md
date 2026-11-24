# 项目交接状态

## 开发与发布

- 在 Git 仓库根目录运行 pnpm 和 Tauri。前端在 `src/`，桌面端在 `src-tauri/`，云服务在 `server/`。
- Git 远程 `origin` 和客户端更新源为 `WangLi0101/game_player`。应用版本以配置文件为准。首次发版前，在当前仓库配置现有的签名 Secrets，按 [更新发布说明](UPDATING.md) 检查。
- Windows 是桌面端目标平台。连接或操作掌机前，按 [掌机操作手册](HANDHELD-OPERATIONS.md) 确认目标设备及本次授权。

## 验证与待办

- 仓库目录整理后，锁文件安装、前端构建、226 项前端测试、Chrome 回归和本地发版脚本通过。构建脚本验证了仅移动目录且版本一致时跳过发布。
- Windows 仍需验收签名更新、透明托盘的焦点与多屏缩放、播放器及真实媒体目录，以及双设备存档同步与恢复。
- 既有影视实机验证覆盖目录扫描、PotPlayer 与 mpv 的续播和分集切换。游戏 v3 备份的范围与恢复流程见 [备份格式](BACKUP-FORMAT.md)，影视数据与回归入口见 [影视接口](desktop/MEDIA-CONTRACT.md)。
- 云服务的部署方式见 [Compose 配置](server/deploy/README.md)，维护前先核对线上状态。真实资料库和存档诊断遵循 [AGENTS.md](../AGENTS.md) 的保护约定。

早期构建、部署、实机验收和数据修复记录可通过 `git show a6896dd:PROJECT-HANDOFF.md` 查阅。
