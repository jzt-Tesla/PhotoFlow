# PhotoFlow V2.5

基于 Tauri 2 + Vue 3 + TypeScript + Pinia + SQLite 的 Windows 本地相册应用。完全离线运行，不引入任何 AI 模型。

## V2 功能

### 🔍 增量索引
- 记录文件大小和最后修改时间
- 二次启动仅扫描变化文件，10000 张照片 < 5 秒
- 新文件自动入库、修改文件重新生成缩略图、删除文件自动移除

### 📸 EXIF 元数据
- 自动提取：拍摄时间、相机型号、镜头型号、GPS 坐标
- 优先使用拍摄时间排序，无拍摄时间时使用文件创建时间

### ⭐ 收藏系统
- 一键收藏/取消收藏（数据库过滤，非前端标记）
- 按收藏筛选照片
- 快捷键：`F`（全屏查看器中，需非输入框焦点）

### 🏷️ 标签系统
- 创建/编辑/删除标签，自定义颜色
- 一张照片可绑定多个标签
- 按标签筛选照片，侧边栏显示每个标签的照片数量

### 🔎 搜索
- 文件名搜索 + 标签搜索
- 实时响应（300ms 防抖）

### 📁 多目录管理
- 侧边栏「所有照片」右侧「＋」按钮添加目录
- 左侧目录树常驻显示，点击目录筛选照片
- 设置页可移除目录

### ⚙️ 设置页面
- 缩略图尺寸/质量调整
- 移除目录
- 清理孤立缩略图缓存
- 查看应用统计信息

### 🖼️ 全屏查看器
- ←/→ 翻页，Esc 关闭
- F 收藏，I 显示 EXIF 信息面板
- 查看原图（photoflow:// 协议）

## V2.5 架构重构

### 统一状态管理（Pinia）
- `photoStore`：照片数据唯一写入口，含 viewerIndex
- `appStore`：应用状态、导航、标签、目录、扫描
- 组件不再直接调用 API，全部通过 store action

### 数据流
```
Tauri Backend → useTauriEvents composable → photoStore → 组件（reactive 订阅）
                                              ↑
                              appStore.navigateTo() 显式调用
```

### 安全审计修复
- `add_scan_directory` 重复插入返回正确 ID
- 缩略图删除前验证路径合法性
- DB 批量写入错误不再静默丢弃
- `resize_bilinear` 算术溢出修复
- 缩略图缺失时自动修复（`repair_thumbnails`）

## 功能特性

- 🖼️ 支持格式：JPG、JPEG、PNG、WebP
- ⚡ 虚拟滚动画廊，支持万级照片无卡顿
- 🔒 安全的自定义 URI 协议，路径遍历防护
- 🔄 缩略图缺失自动修复

## 技术栈

| 层级 | 技术 |
|------|------|
| 前端 | Vue 3 + TypeScript + Vite + Pinia |
| 后端 | Rust (Tauri 2) |
| 数据库 | SQLite (rusqlite, WAL 模式) |
| 图片处理 | image + turbojpeg |
| EXIF | kamadak-exif |
| 构建 | Vite + Tauri CLI |

## 快速开始

```bash
# 环境要求：Node.js 18+，Rust (stable)，Windows 10+

git clone https://github.com/your-username/PhotoFlow.git
cd PhotoFlow
npm install

# 开发模式
npm run tauri dev

# 构建安装包
npm run tauri build
```

## 项目结构

```
PhotoFlow/
├── src/                          # Vue 前端
│   ├── stores/                   # Pinia 状态管理
│   │   ├── photoStore.ts         # 照片数据 + 查看器状态
│   │   └── appStore.ts           # 应用状态 + 导航 + 扫描
│   ├── composables/
│   │   └── useTauriEvents.ts     # Tauri 事件监听 + 微批次缓冲
│   ├── components/
│   │   ├── Sidebar.vue           # 左侧导航 + 目录树 + 标签
│   │   ├── TimelineView.vue      # 照片网格包装
│   │   ├── PhotoGallery.vue      # 虚拟滚动照片网格（纯展示）
│   │   ├── PhotoViewer.vue       # 全屏查看器
│   │   ├── ThumbnailCard.vue     # 缩略图卡片
│   │   ├── SearchBar.vue         # 搜索栏（v-model）
│   │   ├── SettingsView.vue      # 设置页面
│   │   └── TagManager.vue        # 标签管理对话框
│   ├── api.ts                    # Tauri IPC + URI 构建
│   ├── types.ts                  # TypeScript 类型
│   ├── App.vue                   # 根组件（布局 + composable 初始化）
│   └── main.ts                   # Vue + Pinia 入口
├── src-tauri/                    # Rust 后端
│   └── src/
│       ├── lib.rs                # URI handler + 安全验证 + 应用启动
│       ├── main.rs               # 程序入口
│       ├── db.rs                 # SQLite（schema 迁移 + 方法）
│       ├── scanner.rs            # 增量扫描 + EXIF 元数据
│       ├── thumbnail.rs          # 缩略图生成（turbojpeg DCT）
│       └── commands.rs           # IPC 命令
└── package.json
```

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| ← / → | 上/下一张照片 |
| Esc | 关闭查看器 |
| F | 收藏/取消收藏 |
| I | 显示/隐藏 EXIF 信息面板 |

## 数据存储位置

- 数据库：`%APPDATA%/com.photoflow.app/photoflow.db`
- 缩略图：`%APPDATA%/com.photoflow.app/thumbs/`

## 许可证

MIT
