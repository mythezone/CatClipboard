use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// 剪切板历史记录项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: i64,
    pub content_type: String, // "text", "image", "file"
    pub content: String,      // 文本内容或base64编码的图片
    pub preview: String,      // 预览文本
    pub is_favorite: bool,
    pub tags: Vec<String>,
    pub created_at: String,
}

/// 数据库管理器
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

fn build_like_pattern(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut escaped = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '%' => escaped.push_str("\\%"),
            '_' => escaped.push_str("\\_"),
            _ => escaped.push(ch),
        }
    }

    if escaped.is_empty() {
        None
    } else {
        Some(format!("%{}%", escaped))
    }
}

impl Database {
    /// 初始化数据库
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        conn.execute("PRAGMA foreign_keys = ON", [])?;
        
        // 创建历史记录表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type TEXT NOT NULL,
                content TEXT NOT NULL,
                preview TEXT NOT NULL,
                is_favorite INTEGER DEFAULT 0,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        // 创建标签表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL
            )",
            [],
        )?;

        // 创建项目-标签关联表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS item_tags (
                item_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (item_id, tag_id),
                FOREIGN KEY (item_id) REFERENCES clipboard_history(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // 创建全文搜索虚拟表
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS clipboard_fts USING fts5(
                content,
                preview,
                content='clipboard_history',
                content_rowid='id'
            )",
            [],
        )?;

        // 创建触发器以保持 FTS 同步
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS clipboard_ai AFTER INSERT ON clipboard_history BEGIN
                INSERT INTO clipboard_fts(rowid, content, preview) 
                VALUES (new.id, new.content, new.preview);
            END",
            [],
        )?;

        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS clipboard_ad AFTER DELETE ON clipboard_history BEGIN
                DELETE FROM clipboard_fts WHERE rowid = old.id;
            END",
            [],
        )?;

        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS clipboard_au AFTER UPDATE ON clipboard_history BEGIN
                UPDATE clipboard_fts SET content = new.content, preview = new.preview 
                WHERE rowid = new.id;
            END",
            [],
        )?;

        Ok(Database {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 清空所有数据
    pub fn reset_all(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;

        tx.execute("DELETE FROM item_tags", [])?;
        tx.execute("DELETE FROM tags", [])?;
        tx.execute("DELETE FROM clipboard_history", [])?;
        tx.execute("DELETE FROM clipboard_fts", [])?;

        tx.commit()?;
        Ok(())
    }

    /// 添加剪切板记录
    pub fn add_item(&self, content_type: &str, content: &str, preview: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now: DateTime<Utc> = Utc::now();
        
        conn.execute(
            "INSERT INTO clipboard_history (content_type, content, preview, created_at) 
             VALUES (?1, ?2, ?3, ?4)",
            params![content_type, content, preview, now.to_rfc3339()],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// 获取所有历史记录（带分页）
    pub fn get_items(&self, limit: i64, offset: i64) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content_type, content, preview, is_favorite, created_at 
             FROM clipboard_history 
             ORDER BY created_at DESC 
             LIMIT ?1 OFFSET ?2",
        )?;

        let items = stmt
            .query_map(params![limit, offset], |row| {
                let item_id: i64 = row.get(0)?;
                Ok(ClipboardItem {
                    id: item_id,
                    content_type: row.get(1)?,
                    content: row.get(2)?,
                    preview: row.get(3)?,
                    is_favorite: row.get::<_, i64>(4)? != 0,
                    tags: Vec::new(), // 稍后填充
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // 为每个项目获取标签
        let mut items_with_tags = Vec::new();
        for mut item in items {
            item.tags = self.get_item_tags_internal(&conn, item.id)?;
            items_with_tags.push(item);
        }

        Ok(items_with_tags)
    }

    /// 搜索历史记录
    pub fn search_items(&self, query: &str, limit: i64) -> Result<Vec<ClipboardItem>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return self.get_items(limit, 0);
        }

        let conn = self.conn.lock().unwrap();
        let like_pattern = match build_like_pattern(trimmed) {
            Some(pattern) => pattern,
            None => return Ok(Vec::new()),
        };
        let like_param = like_pattern.to_lowercase();

        let mut stmt = conn.prepare(
            "SELECT DISTINCT h.id, h.content_type, h.content, h.preview, h.is_favorite, h.created_at
             FROM clipboard_history h
             LEFT JOIN item_tags it ON h.id = it.item_id
             LEFT JOIN tags t ON it.tag_id = t.id
             WHERE LOWER(h.content) LIKE ?1 ESCAPE '\\'
                OR LOWER(h.preview) LIKE ?1 ESCAPE '\\'
                OR LOWER(IFNULL(t.name, '')) LIKE ?1 ESCAPE '\\'
             ORDER BY h.is_favorite DESC, h.created_at DESC
             LIMIT ?2",
        )?;

        let items = stmt
            .query_map(params![like_param, limit], |row| {
                let item_id: i64 = row.get(0)?;
                Ok(ClipboardItem {
                    id: item_id,
                    content_type: row.get(1)?,
                    content: row.get(2)?,
                    preview: row.get(3)?,
                    is_favorite: row.get::<_, i64>(4)? != 0,
                    tags: Vec::new(),
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut items_with_tags = Vec::with_capacity(items.len());
        for mut item in items {
            item.tags = self.get_item_tags_internal(&conn, item.id)?;
            items_with_tags.push(item);
        }

        Ok(items_with_tags)
    }

    /// 切换收藏状态
    pub fn toggle_favorite(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let is_favorite: i64 = conn
            .query_row(
                "SELECT is_favorite FROM clipboard_history WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0);

        let new_state = if is_favorite == 0 { 1 } else { 0 };
        
        conn.execute(
            "UPDATE clipboard_history SET is_favorite = ?1 WHERE id = ?2",
            params![new_state, id],
        )?;

        Ok(new_state != 0)
    }

    /// 删除记录
    pub fn delete_item(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clipboard_history WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// 清空所有非收藏的历史记录
    pub fn clear_non_favorites(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clipboard_history WHERE is_favorite = 0", [])?;
        Ok(())
    }

    /// 维护历史记录数量上限
    pub fn maintain_limit(&self, max_items: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        if max_items <= 0 {
            conn.execute("DELETE FROM clipboard_history", [])?;
            return Ok(());
        }

        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clipboard_history",
            [],
            |row| row.get(0),
        )?;

        if total <= max_items {
            return Ok(());
        }

        let to_remove = total - max_items;

        let removed_non_favorites = conn.execute(
            "DELETE FROM clipboard_history WHERE id IN (
                 SELECT id FROM clipboard_history
                 WHERE is_favorite = 0
                 ORDER BY created_at ASC, id ASC
                 LIMIT ?1
             )",
            params![to_remove],
        )? as i64;

        let remaining = to_remove.saturating_sub(removed_non_favorites);

        if remaining > 0 {
            conn.execute(
                "DELETE FROM clipboard_history WHERE id IN (
                     SELECT id FROM clipboard_history
                     ORDER BY created_at ASC, id ASC
                     LIMIT ?1
                 )",
                params![remaining],
            )?;
        }
        Ok(())
    }

    /// 添加标签
    pub fn add_tag(&self, name: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", params![name])?;
        
        let tag_id: i64 = conn.query_row(
            "SELECT id FROM tags WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )?;
        
        Ok(tag_id)
    }

    /// 为项目添加标签
    pub fn add_item_tag(&self, item_id: i64, tag_name: &str) -> Result<()> {
        let tag_id = self.add_tag(tag_name)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO item_tags (item_id, tag_id) VALUES (?1, ?2)",
            params![item_id, tag_id],
        )?;
        Ok(())
    }

    /// 移除项目标签
    pub fn remove_item_tag(&self, item_id: i64, tag_name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM item_tags 
             WHERE item_id = ?1 
             AND tag_id = (SELECT id FROM tags WHERE name = ?2)",
            params![item_id, tag_name],
        )?;
        Ok(())
    }

    /// 获取项目的所有标签（内部方法，用于已有连接）
    fn get_item_tags_internal(&self, conn: &Connection, item_id: i64) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(
            "SELECT t.name FROM tags t
             JOIN item_tags it ON t.id = it.tag_id
             WHERE it.item_id = ?1",
        )?;

        let tags = stmt
            .query_map(params![item_id], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(tags)
    }

    /// 获取所有标签
    pub fn get_all_tags(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT name FROM tags ORDER BY name")?;
        
        let tags = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(tags)
    }

    /// 按标签获取项目
    pub fn get_items_by_tag(&self, tag_name: &str, limit: i64) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT h.id, h.content_type, h.content, h.preview, h.is_favorite, h.created_at
             FROM clipboard_history h
             JOIN item_tags it ON h.id = it.item_id
             JOIN tags t ON it.tag_id = t.id
             WHERE t.name = ?1
             ORDER BY h.created_at DESC
             LIMIT ?2",
        )?;

        let items = stmt
            .query_map(params![tag_name, limit], |row| {
                let item_id: i64 = row.get(0)?;
                Ok(ClipboardItem {
                    id: item_id,
                    content_type: row.get(1)?,
                    content: row.get(2)?,
                    preview: row.get(3)?,
                    is_favorite: row.get::<_, i64>(4)? != 0,
                    tags: Vec::new(),
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut items_with_tags = Vec::new();
        for mut item in items {
            item.tags = self.get_item_tags_internal(&conn, item.id)?;
            items_with_tags.push(item);
        }

        Ok(items_with_tags)
    }
}
