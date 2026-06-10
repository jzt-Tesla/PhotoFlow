# PhotoFlow

基于 Tauri 2 + Vue 3 + TypeScript + SQLite 的 Windows 桌面照片管理应用。

## 功能特性

- 📂 选择并递归扫描本地照片目录
- 🖼️ 支持格式：JPG、JPEG、PNG、WebP
- 🔍 自动提取 EXIF 拍摄时间，按时间倒序排列
- 🗂️ SQLite 数据库索引，快速检索
- 🖼️ 自动生成 200px 缩略图，独立存储
- ⚡ 虚拟滚动画廊，支持万级照片无卡顿
- 🔎 全屏查看器，支持键盘左右翻页（←/→/Esc）
- ♻️ 增量扫描 + 孤儿记录自动清理
- 🔒 安全的自定义 URI 协议，路径遍历防护

## 技术栈

| 层级 | 技术 |
|------|------|
| 前端 | Vue 3 + TypeScript + Vite |
| 后端 | Rust (Tauri 2) |
| 数据库 | SQLite (rusqlite) |
| 图片处理 | image + kamadak-exif |
| 构建工具 | Vite + Tauri CLI |

## 快速开始

### 环境要求

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) (stable)
- Windows 10+ (WebView2 Runtime，Win11 自带)

### 安装与运行

```bash
# 克隆项目
git clone https://github.com/your-username/PhotoFlow.git
cd PhotoFlow

# 安装前端依赖
npm install

# 开发模式运行
npm run tauri dev

# 构建 Windows 安装包
npm run tauri build
```

构建产物：
- 可执行文件：`src-tauri/target/release/photoflow.exe`
- 安装包：`src-tauri/target/release/bundle/nsis/PhotoFlow_1.0.0_x64-setup.exe`

## 项目结构

```
PhotoFlow/
├── src/                          # Vue 前端
│   ├── components/
│   │   ├── PhotoGallery.vue      # 虚拟滚动画廊
│   │   ├── PhotoViewer.vue       # 全屏照片查看器
│   │   └── ThumbnailCard.vue     # 缩略图卡片组件
│   ├── api.ts                    # Tauri IPC 桥接 + URI 构建
│   ├── types.ts                  # TypeScript 类型定义
│   ├── App.vue                   # 根组件（扫描/画廊/查看器状态）
│   └── main.ts                   # 入口文件
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── lib.rs                # 应用入口、URI scheme handler、安全验证
│   │   ├── main.rs               # 程序入口
│   │   ├── db.rs                 # SQLite 数据库操作（线程安全）
│   │   ├── scanner.rs            # 目录扫描、EXIF 提取、批量索引
│   │   ├── thumbnail.rs          # 缩略图生成（Lanczos3 算法）
│   │   └── commands.rs           # Tauri IPC 命令定义
│   ├── capabilities/
│   │   └── default.json          # 应用权限声明
│   ├── tauri.conf.json           # Tauri 配置
│   └── Cargo.toml                # Rust 依赖
└── package.json                  # 前端依赖
```

## 架构说明

### 数据流

```
用户选择目录
    ↓
scanner::scan_directory()
    ├─ WalkDir 递归遍历
    ├─ image::image_dimensions() 获取尺寸
    ├─ exif::Reader 读取拍摄时间
    ├─ thumbnail::generate_thumbnail() 生成缩略图
    └─ db::insert_photos_batch() 批量入库
    ↓
前端 loadPhotos() 分页加载
    ↓
PhotoGallery 虚拟滚动渲染
    ├─ ThumbnailCard → http://photoflow.localhost/thumb/{hash}
    └─ PhotoViewer  → http://photoflow.localhost/photo/{base64}
    ↓
URI Scheme Handler (lib.rs)
    ├─ /thumb/{hash}  → 读取 AppData/thumbs/{hash}.jpg
    └─ /photo/{base64} → 校验路径后读取原图
```

### 自定义协议

应用注册了 `photoflow://` 自定义 URI 协议用于向 WebView 传递图片数据：

- **缩略图**：`http://photoflow.localhost/thumb/{sha256_hash}`
  - hash 由文件路径 SHA-256 生成，确保唯一性
  - 直接读取 AppData 下的缩略图文件
- **原图**：`http://photoflow.localhost/photo/{base64_encoded_path}`
  - 路径经过 canonicalize + 目录白名单校验
  - Magic bytes 验证，仅返回真实图片

### 安全机制

| 机制 | 说明 |
|------|------|
| 路径遍历防护 | `/photo/` 端点校验 `canonicalize()` 后路径必须在允许目录下 |
| 目录白名单 | `ALLOWED_ROOTS` 动态管理，仅允许用户选择的扫描目录 |
| Hash 验证 | `/thumb/` 端点仅接受十六进制 hash，防止路径注入 |
| Magic bytes | 文件头验证，拒绝非图片文件 |
| CSP 策略 | 限制图片来源为 `self` + `photoflow.localhost` |
| 权限最小化 | 移除 fs 插件的通配符权限，前端不直接访问文件系统 |

## 配置说明

### tauri.conf.json

| 配置项 | 值 | 说明 |
|--------|-----|------|
| `identifier` | `com.photoflow.app` | 应用标识符 |
| `security.csp` | 见文件 | 内容安全策略 |
| `bundle.targets` | `nsis` | 构建 Windows 安装包 |
| `app.windows` | 1200×800 | 默认窗口尺寸 |

### 数据存储位置

- 数据库：`%APPDATA%/com.photoflow.app/photoflow.db`
- 缩略图：`%APPDATA%/com.photoflow.app/thumbs/`
- 扫描目录：存储在数据库 `settings` 表的 `scan_dir` 键中

## 开发指南

### 常用命令

```bash
npm run dev          # 启动 Vite 开发服务器（仅前端）
npm run build        # TypeScript 类型检查 + Vite 构建
npm run tauri dev    # 开发模式（前端 + Rust 后端联动）
npm run tauri build  # 生产构建（生成安装包）
```

### 添加新的 IPC 命令

1. 在 `src-tauri/src/commands.rs` 中定义 `#[tauri::command]` 函数
2. 在 `src-tauri/src/lib.rs` 的 `invoke_handler` 中注册
3. 在 `src/api.ts` 中封装前端调用函数

### 数据库迁移

当前使用 `CREATE TABLE IF NOT EXISTS` 建表。如需新增字段，在 `db.rs` 的 `open()` 方法中添加 `ALTER TABLE` 语句。

## 许可证

MIT
