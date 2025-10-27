# Cat History

一个轻量级的剪切板历史管理工具，采用简洁风格 UI 设计。

## 功能特性

-  **自动监听剪切板**：实时监控系统剪切板变化
-  **历史记录管理**：存储最近 100 条剪切板记录（可调）
-  **全文搜索**：支持快速模糊搜索和关键词高亮
-  **收藏功能**：置顶常用内容
-  **标签管理**：为记录添加自定义标签
-  **快速操作**：一键复制回剪切板

## 功能展示

<p align="center">功能展示</p>

| 主页面 | 查询 | 收藏 |
| :----: | :--: | :--: |
| ![主页面截图](img/image.png) | ![查询截图](img/image3.png) | ![收藏截图](img/image4.png) |

## 技术栈

- **后端**：Rust + Tauri
- **前端**：React + TypeScript
- **数据库**：SQLite with FTS5
- **样式**：CSS3 (Apple Design)

## 开发

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建
npm run tauri build
```

## 架构

```
Cat History
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs        # 主入口
│   │   ├── lib.rs         # Tauri 命令
│   │   ├── clipboard.rs   # 剪切板监听
│   │   ├── database.rs    # 数据库操作
│   │   └── config.rs      # 配置管理
│   └── Cargo.toml
└── src/                   # React 前端
    ├── App.tsx            # 主组件
    ├── main.tsx           # 入口
    └── styles.css         # 样式

```

## 数据库设计

- `clipboard_history`: 历史记录表
- `tags`: 标签表
- `item_tags`: 项目-标签关联表
- `clipboard_fts`: 全文搜索虚拟表

## 许可证

MIT
