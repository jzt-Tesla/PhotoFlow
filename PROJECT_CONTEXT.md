# PhotoFlow — PROJECT_CONTEXT.md

> 最后更新: 2026-06-11 (V2.5)

---

## 项目目标

将 PhotoFlow 从基础图片浏览器升级为**功能完整的本地相册应用**。

核心约束：
- Windows 本地运行，完全离线
- 不引入任何 AI / 人脸识别 / 云同步
- Tauri 2 + Vue 3 + Pinia + SQLite 技术栈
- 10000 张照片规模下保持流畅

---

## 技术栈

| 层级 | 技术 | 版本 |
|------|------|------|
| 桌面框架 | Tauri 2 | 2.11.2 |
| 前端 | Vue 3 + TypeScript + Vite + Pinia | Vue 3.4, Vite 5.4, Pinia 2.1 |
| 后端 | Rust | edition 2021 |
| 数据库 | SQLite (rusqlite, WAL 模式) | rusqlite 0.31 |
| 图片处理 | image + turbojpeg | image 0.25, turbojpeg 1 |
| EXIF 提取 | kamadak-exif | 0.5 |
| 异步运行时 | tokio | 1 (rt feature) |
| 构建 | Vite + Tauri CLI + NSIS | — |

---

## 当前架构（V2.5）

```
┌─────────────────────────────────────────────────────────────────┐
│  前端 (Vue 3 + TypeScript + Pinia)                               │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Stores (单一数据源)                                        │   │
│  │  photoStore: photos[] + viewerIndex + loadPhotos/loadMore │   │
│  │  appStore: view + tags + dirs + status + navigateTo()     │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────┐   │
│  │ Sidebar  │ │ Photo    │ │ Photo    │ │ Settings         │   │
│  │ (导航+   │ │ Gallery  │ │ Viewer   │ │ View             │   │
│  │  目录树+ │ │ (纯展示) │ │ (全屏)   │ │                  │   │
│  │  标签)   │ │          │ │          │ │                  │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────────┘   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                        │
│  │ SearchBar│ │ TagMgr   │ │ ThumbCard│                        │
│  └──────────┘ └──────────┘ └──────────┘                        │
├────────────────── Tauri IPC (invoke/listen) ─────────────────────┤
│  后端 (Rust)                                                     │
│  ┌───────────┐ ┌────────────┐ ┌──────────┐ ┌────────────────┐  │
│  │ commands  │ │ scanner    │ │ db       │ │ thumbnail      │  │
│  │ (26 IPC)  │ │ (增量扫描) │ │ (SQLite) │ │ (turbojpeg DCT)│  │
│  └───────────┘ └────────────┘ └──────────┘ └────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ lib.rs — URI scheme handler + 安全验证 + 应用启动          │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 关键架构决策

1. **自定义 URI 协议**：注册 `photoflow://` scheme，前端通过 `http://photoflow.localhost/` 访问图片
2. **流式扫描**：scanner 每处理一张照片立即通过 `photo-stream` event 推送到前端，DB 批量写入（100 条/事务）
3. **turbojpeg DCT 缩放**：JPEG 缩略图在解码阶段直接缩放，避免全尺寸像素解码，提速 3×
4. **虚拟滚动**：手动 absolute positioning 实现，无第三方依赖
5. **增量扫描**：通过 `file_size` + `modified_time` 比对，未变化文件跳过
6. **Pinia 单一数据源**：photoStore 是照片数据唯一写入口，组件通过 storeToRefs 读取
7. **显式导航**：appStore.navigateTo() 直接调用 photoStore.loadPhotos()，无 watch 隐式触发
8. **Tauri 参数兼容**：filter 通过 JSON 字符串传递（filterJson），绕过 Tauri v2 对 Option 参数的 falsy 过滤

### 数据库 Schema（5 表）

| 表 | 字段数 | 说明 |
|----|--------|------|
| `photo` | 15 | 路径、尺寸、EXIF（拍摄时间/相机/镜头/GPS）、收藏标记 |
| `tag` | 3 + count | 标签名 + 颜色（list_tags 返回照片数量） |
| `photo_tag` | 2 | 多对多关联（CASCADE 删除） |
| `scan_directory` | 3 | 扫描目录路径 + 添加时间 |
| `settings` | 2 | 键值对配置存储 |

### 26 个 IPC 命令

| 类别 | 命令 |
|------|------|
| 扫描 | `select_and_scan_directory`, `rescan_all_directories`, `rescan_directory_by_id` |
| 照片加载 | `load_photos`, `get_photo_count`, `load_photos_filtered`, `photo_count_filtered` |
| 收藏 | `toggle_favorite` |
| 标签 | `create_tag`, `delete_tag`, `rename_tag`, `update_tag_color`, `list_tags`, `add_photo_tag`, `remove_photo_tag`, `get_photo_tags` |
| 时间轴 | `load_timeline_groups` |
| 多目录 | `add_scan_directory`, `remove_scan_directory`, `list_scan_directories`, `pick_and_add_directory` |
| 设置 | `get_settings`, `update_settings`, `clear_thumbnail_cache`, `get_app_info` |
| 维护 | `repair_thumbnails` |

---

## 已完成功能

### V1（基础版）
- [x] 单目录扫描 + SQLite 索引
- [x] 缩略图生成（200px Lanczos3）
- [x] 虚拟滚动画廊
- [x] 全屏查看器（←/→/Esc）
- [x] 自定义 URI 协议安全方案
- [x] EXIF 拍摄时间提取
- [x] 分页加载（60 张/页）

### V2（完整相册）
- [x] 增量索引（file_size + modified_time 比对）
- [x] EXIF 元数据扩展（相机/镜头/GPS）
- [x] turbojpeg DCT 缩放（3× 提速）
- [x] 流式照片加载（每张照片即时推送）
- [x] 秒开相册（DB 优先加载，~570ms 首屏）
- [x] 收藏系统（toggle + 筛选 + F 快捷键）
- [x] 标签系统（CRUD + 多对多关联 + 颜色 + count）
- [x] 搜索（文件名 + 标签，300ms 防抖）
- [x] 多目录管理（添加/删除/按目录筛选）
- [x] 设置页面（缩略图配置/目录管理/缓存清理）
- [x] PhotoViewer EXIF 信息面板（I 快捷键）

### V2.5（架构重构）
- [x] Pinia 统一状态管理（photoStore + appStore）
- [x] App.vue 职责拆分（581→322 行，script 245→58 行）
- [x] PhotoGallery 纯展示化（移除 local-fetch 模式）
- [x] TimelineView 精简（189→15 行，目录树移入 Sidebar）
- [x] Sidebar 目录树常驻显示 + 「所有照片」右侧「＋」添加按钮
- [x] Tauri v2 参数兼容（filterJson JSON 字符串传递）
- [x] 收藏过滤修复（Option<bool> → JSON 字符串反序列化）
- [x] 缩略图缺失自动修复（repair_thumbnails）
- [x] 目录切换不空白（旧照片保留到新数据到位）
- [x] 快捷键安全（忽略 INPUT/TEXTAREA 中的按键）
- [x] 安全审计修复（7 项：C1/H1-H4/M3/L1）

---

## 未完成功能

- [ ] 照片删除功能
- [ ] 照片旋转/编辑
- [ ] 批量操作（批量收藏/批量标签）
- [ ] 缩略图质量设置生效（size 已生效，quality 仍硬编码 80）
- [ ] 拖拽导入照片
- [ ] 照片地图视图（基于 GPS）
- [ ] 深色/浅色主题切换
- [ ] 视频文件支持
- [ ] RAW 格式支持
- [ ] 照片导出功能
- [ ] 多语言支持

---

## 已知问题

### 代码层面
| 编号 | 严重性 | 位置 | 说明 | 状态 |
|------|--------|------|------|------|
| D1 | Low | `Cargo.toml` | `tauri-plugin-fs` 已声明但未使用 | ✅ 已修复 |
| D2 | Low | `DirectoryManager.vue` | 组件存在但未被引用，属于死代码 | ✅ 已修复 |
| D3 | Low | `db.rs` | Tag 结构体缺少 count 字段 | ✅ 已修复 |
| D4 | Low | `App.vue` | `handleLoadMoreFiltered` 永远返回 false | ✅ 已修复 |
| D5 | Medium | `scanner.rs` | 缩略图尺寸硬编码为 200 | ✅ 已修复 |
| F1 | High | `api.ts`/`commands.rs` | 收藏过滤不生效（Tauri v2 Option 参数过滤） | ✅ 已修复 |
| F2 | Medium | `db.rs` | `add_scan_directory` 重复插入返回错误 ID | ✅ 已修复 |
| F3 | Medium | `db.rs` | 缩略图删除未验证路径 | ✅ 已修复 |
| F4 | Medium | `scanner.rs` | `flush_db_batch` 静默丢弃错误 | ✅ 已修复 |
| F5 | Low | `thumbnail.rs` | `resize_bilinear` u32 算术溢出 | ✅ 已修复 |

### 架构层面
- 首次扫描大量新照片时缩略图生成仍是瓶颈（turbojpeg 11ms/张）
- `batch_check_existing` 将全表加载到内存，超大库（>50000 张）可能有内存压力
- Tauri v2 对 `Option<T>` 参数做 falsy 过滤，已通过 JSON 字符串绕过

---

## 下一步开发计划

### 短期
1. **照片删除** — 右键菜单删除照片（DB + 文件 + 缩略图）
2. **批量操作** — 多选照片后批量收藏/标签
3. **缩略图质量生效** — 从 AppSettings 读取 quality 传给 turbojpeg

### 中期
4. **照片旋转** — 无损 JPEG 旋转（turbojpeg transform）
5. **地图视图** — 基于 GPS 坐标的照片地图
6. **拖拽导入** — 支持拖拽文件/文件夹到窗口导入

### 长期
7. **视频支持** — FFmpeg 缩略图 + 播放
8. **主题系统** — 深色/浅色/自定义主题
9. **多语言** — i18n 支持

---

## 重要技术决策记录

### 1. 自定义 URI 协议
**决策**：使用 `photoflow://` 自定义协议 + WebView2 `http://photoflow.localhost/` workaround
**原因**：需要从文件系统读取用户照片，Tauri asset 协议只能访问打包资源

### 2. turbojpeg vs image crate
**决策**：JPEG 用 turbojpeg（DCT 域缩放），PNG/WebP 回退 image crate
**原因**：实测提速 3×（11ms vs 32ms/张）

### 3. 流式扫描
**决策**：每张照片处理完立即通过 Tauri event 推送，DB 批量写入（100 条/事务）
**原因**：用户需要即时看到照片，不能等全部扫描完成

### 4. Pinia 单一数据源
**决策**：photoStore 为照片数据唯一写入口，appStore 管理应用状态
**原因**：V2 中照片数据存在双重所有权（App.vue + TimelineView），导致 favorite 不同步
**影响**：所有组件通过 storeToRefs 读取，不直接调用 API

### 5. Tauri v2 参数兼容
**决策**：PhotoFilter 通过 JSON 字符串（filterJson）传递给 Rust
**原因**：Tauri v2 对 `Option<T>` 参数做 falsy 值过滤，`Option<bool>` 始终收到 `None`
**影响**：Rust 端用 `serde_json::from_str` 反序列化，`PhotoFilter` 加 `#[serde(rename_all = "camelCase")]`

### 6. WAL 模式
**决策**：`PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL`
**原因**：读写并发，写入不阻塞读取

---

## 文件结构

```
PhotoFlow/
├── src/                              # Vue 前端
│   ├── stores/                       # Pinia 状态管理
│   │   ├── photoStore.ts             # 照片数据 + 查看器（~130 行）
│   │   └── appStore.ts              # 应用状态 + 导航（~140 行）
│   ├── composables/
│   │   └── useTauriEvents.ts         # Tauri 事件 + 微批次缓冲（~75 行）
│   ├── components/
│   │   ├── Sidebar.vue               # 左侧导航 + 目录树 + 标签
│   │   ├── TimelineView.vue          # 照片网格包装（~15 行）
│   │   ├── PhotoGallery.vue          # 虚拟滚动照片网格（纯展示）
│   │   ├── PhotoViewer.vue           # 全屏查看器 + EXIF 面板
│   │   ├── ThumbnailCard.vue         # 缩略图卡片
│   │   ├── SearchBar.vue             # 搜索栏（v-model + 300ms 防抖）
│   │   ├── TagManager.vue            # 标签管理对话框
│   │   └── SettingsView.vue          # 设置页面
│   ├── api.ts                        # IPC 函数 + URL 构建
│   ├── types.ts                      # TypeScript 类型定义
│   ├── App.vue                       # 根组件（~320 行，布局 + composable）
│   └── main.ts                       # Vue + Pinia 入口
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs                    # URI handler + 安全验证 + 启动
│   │   ├── main.rs                   # 程序入口
│   │   ├── commands.rs               # 26 个 IPC 命令
│   │   ├── db.rs                     # SQLite 层（~920 行）
│   │   ├── scanner.rs                # 增量扫描引擎
│   │   └── thumbnail.rs             # 缩略图生成（turbojpeg DCT）
│   ├── Cargo.toml                    # Rust 依赖
│   └── tauri.conf.json              # Tauri 配置
├── package.json                      # 前端依赖（含 pinia）
├── vite.config.ts                    # Vite 配置
├── README.md                         # 项目文档
└── PROJECT_CONTEXT.md                # 本文件
```

---

## 构建命令

```bash
npm install                    # 安装前端依赖
npm run tauri dev              # 开发模式（前端热重载 + Rust 后端）
npm run tauri build            # 生产构建（生成 NSIS 安装包）
cargo check --manifest-path src-tauri/Cargo.toml  # 仅检查 Rust 编译
```

---

## 数据存储位置

| 数据 | 路径 |
|------|------|
| 数据库 | `%APPDATA%/com.photoflow.app/photoflow.db` |
| 缩略图 | `%APPDATA%/com.photoflow.app/thumbs/{sha256}.jpg` |
| WAL 日志 | `%APPDATA%/com.photoflow.app/photoflow.db-wal` |
