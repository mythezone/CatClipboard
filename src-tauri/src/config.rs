use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MIN_HISTORY_LIMIT: i64 = 1;
const MAX_HISTORY_LIMIT: i64 = 5_000;

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 历史记录最大数量
    pub max_history_items: i64,
    /// 是否开机自启
    pub auto_start: bool,
    /// 主题模式: "light", "dark", "auto"
    pub theme: String,
    /// 全局快捷键
    pub hotkey: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_history_items: 100,
            auto_start: false,
            theme: "auto".to_string(),
            hotkey: "CommandOrControl+Shift+V".to_string(),
        }
    }
}

impl Config {
    /// 加载配置
    pub fn load(config_path: PathBuf) -> Result<Self> {
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let mut config: Config = serde_json::from_str(&content)?;
            config.sanitize();
            Ok(config)
        } else {
            let config = Config::default();
            config.save(config_path)?;
            Ok(config)
        }
    }

    /// 保存配置
    pub fn save(&self, config_path: PathBuf) -> Result<()> {
        let mut sanitized = self.clone();
        sanitized.sanitize();
        let content = serde_json::to_string_pretty(&sanitized)?;
        
        // 确保父目录存在
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(config_path, content)?;
        Ok(())
    }

    /// 规范化配置中的字段，确保取值安全
    pub fn sanitize(&mut self) {
        if self.max_history_items < MIN_HISTORY_LIMIT {
            self.max_history_items = MIN_HISTORY_LIMIT;
        } else if self.max_history_items > MAX_HISTORY_LIMIT {
            self.max_history_items = MAX_HISTORY_LIMIT;
        }

        if !matches!(self.theme.as_str(), "light" | "dark" | "auto") {
            self.theme = "auto".to_string();
        }

        if self.hotkey.trim().is_empty() {
            self.hotkey = Config::default().hotkey;
        }
    }

    /// 返回一个经过 sanitize 处理的配置副本
    pub fn sanitized(mut self) -> Self {
        self.sanitize();
        self
    }
}
