use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;
use tauri::Emitter;

#[cfg(windows)]
use std::ffi::c_void;

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{HANDLE, HWND},
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber,
            IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
        },
        Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
    },
    UI::Shell::{DragQueryFileW, HDROP},
};

#[cfg(windows)]
const CF_UNICODETEXT: u32 = 13;
#[cfg(windows)]
const CF_HDROP: u32 = 15;

/// 剪切板事件负载，发送给前端和后端监听器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardSnapshot {
    pub content_type: String, // "text" | "file" | "image"
    pub content: String,      // 原始内容（文本或 JSON 字符串等）
    pub preview: String,      // 展示用预览文本
}

impl ClipboardSnapshot {
    fn signature(&self) -> String {
        format!("{}:{}", self.content_type, self.content)
    }
}

/// 剪切板监听器
pub struct ClipboardMonitor {
    last_signature: Arc<Mutex<String>>,
    #[cfg(windows)]
    last_sequence: Arc<AtomicU32>,
}

impl ClipboardMonitor {
    pub fn new() -> Self {
        Self {
            last_signature: Arc::new(Mutex::new(String::new())),
            #[cfg(windows)]
            last_sequence: Arc::new(AtomicU32::new(0)),
        }
    }

    /// 启动剪切板监听
    #[cfg(windows)]
    pub fn start<R: tauri::Runtime>(&self, app_handle: tauri::AppHandle<R>) {
        let signature_guard = Arc::clone(&self.last_signature);
        let sequence_guard = Arc::clone(&self.last_sequence);

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(320));

                let current_sequence = unsafe { GetClipboardSequenceNumber() };

                // 0 表示失败或不支持，直接跳过
                if current_sequence == 0 {
                    continue;
                }

                let previous_sequence = sequence_guard.load(Ordering::Relaxed);
                if current_sequence == previous_sequence {
                    continue;
                }

                sequence_guard.store(current_sequence, Ordering::Relaxed);

                match Self::capture_clipboard_snapshot() {
                    Ok(Some(snapshot)) => {
                        let mut last = signature_guard
                            .lock()
                            .expect("poisoned clipboard signature");
                        if *last == snapshot.signature() {
                            continue;
                        }

                        *last = snapshot.signature();

                        if let Err(err) = app_handle.emit("clipboard-changed", snapshot) {
                            eprintln!("Failed to emit clipboard event: {err:?}");
                        }
                    }
                    Ok(None) => {
                        // 没有有效内容，忽略
                    }
                    Err(err) => {
                        eprintln!("Clipboard capture error: {err:?}");
                    }
                }
            }
        });
    }

    #[cfg(windows)]
    fn capture_clipboard_snapshot() -> Result<Option<ClipboardSnapshot>> {
        unsafe {
            let _guard = ClipboardGuard::acquire()?;

            if IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 {
                if let Some(text) = Self::read_unicode_text()? {
                    let normalized = normalize_newlines(&text);
                    if normalized.trim().is_empty() {
                        return Ok(None);
                    }

                    let preview = build_text_preview(&normalized);
                    return Ok(Some(ClipboardSnapshot {
                        content_type: "text".to_string(),
                        content: normalized,
                        preview,
                    }));
                }
            }

            if IsClipboardFormatAvailable(CF_HDROP) != 0 {
                if let Some(files) = Self::read_file_list()? {
                    if !files.is_empty() {
                        let preview = build_file_preview(&files);
                        let content = serde_json::to_string(&files)?;
                        return Ok(Some(ClipboardSnapshot {
                            content_type: "file".to_string(),
                            content,
                            preview,
                        }));
                    }
                }
            }

            Ok(None)
        }
    }

    #[cfg(windows)]
    unsafe fn read_unicode_text() -> Result<Option<String>> {
        let handle: HANDLE = GetClipboardData(CF_UNICODETEXT);
        if handle.is_null() {
            return Ok(None);
        }

        let data = GlobalLock(handle);
        if data.is_null() {
            return Ok(None);
        }

        let text = read_wide_string(data as *const u16).unwrap_or_default();

        GlobalUnlock(handle);

        if text.is_empty() {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }

    #[cfg(windows)]
    unsafe fn read_file_list() -> Result<Option<Vec<String>>> {
        let handle: HANDLE = GetClipboardData(CF_HDROP);
        if handle.is_null() {
            return Ok(None);
        }

        let data = GlobalLock(handle);
        if data.is_null() {
            return Ok(None);
        }

        let hdrop = data as HDROP;
        let count = DragQueryFileW(hdrop, u32::MAX, std::ptr::null_mut(), 0);
        let mut files = Vec::new();

        for index in 0..count {
            let length = DragQueryFileW(hdrop, index, std::ptr::null_mut(), 0);
            if length == 0 {
                continue;
            }

            let mut buffer = vec![0u16; (length + 1) as usize];
            let copied = DragQueryFileW(hdrop, index, buffer.as_mut_ptr(), length + 1);

            if copied > 0 {
                let path = String::from_utf16_lossy(&buffer[..copied as usize]);
                files.push(path);
            }
        }

        GlobalUnlock(handle);

        Ok(Some(files))
    }

    /// 设置剪切板文本
    #[cfg(windows)]
    pub fn set_clipboard_text(text: &str) -> Result<()> {
        unsafe {
            let _guard = ClipboardGuard::acquire()?;
            if EmptyClipboard() == 0 {
                return Err(anyhow!("Failed to empty clipboard"));
            }

            let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let len_bytes = wide.len() * 2;
            let handle = GlobalAlloc(GMEM_MOVEABLE, len_bytes);
            if handle.is_null() {
                return Err(anyhow!("Failed to allocate clipboard memory"));
            }

            let data = GlobalLock(handle);

            if data.is_null() {
                return Err(anyhow!("Failed to lock global memory for clipboard"));
            }

            std::ptr::copy_nonoverlapping(wide.as_ptr(), data as *mut u16, wide.len());
            GlobalUnlock(handle);

            if SetClipboardData(CF_UNICODETEXT, handle).is_null() {
                return Err(anyhow!("Failed to set clipboard data"));
            }
            Ok(())
        }
    }

    /// 获取剪切板图片（base64 编码）
    #[cfg(windows)]
    #[allow(dead_code)]
    pub fn get_clipboard_image() -> Result<Option<String>> {
        // TODO: 实现图片获取逻辑（转换 DIB 到 PNG）
        Ok(None)
    }
}

#[cfg(not(windows))]
impl ClipboardMonitor {
    pub fn start<R: tauri::Runtime>(&self, _app_handle: tauri::AppHandle<R>) {
        eprintln!("Clipboard monitoring is only supported on Windows");
    }

    pub fn set_clipboard_text(_text: &str) -> Result<()> {
        anyhow::bail!("Clipboard is only supported on Windows")
    }

    #[allow(dead_code)]
    pub fn get_clipboard_image() -> Result<Option<String>> {
        Ok(None)
    }
}

#[cfg(windows)]
struct ClipboardGuard;

#[cfg(windows)]
impl ClipboardGuard {
    unsafe fn acquire() -> Result<Self> {
        for _ in 0..5 {
            if OpenClipboard(std::ptr::null_mut::<c_void>() as HWND) != 0 {
                return Ok(Self);
            }
            thread::sleep(Duration::from_millis(30));
        }

        Err(anyhow!("Unable to open clipboard"))
    }
}

#[cfg(windows)]
impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

fn build_text_preview(text: &str) -> String {
    const MAX_PREVIEW_LEN: usize = 120;
    let single_line = text.trim().lines().take(6).collect::<Vec<_>>().join("\n");
    if single_line.len() <= MAX_PREVIEW_LEN {
        single_line
    } else {
        // 安全地在字符边界处截取
        let mut end_index = MAX_PREVIEW_LEN;
        while end_index > 0 && !single_line.is_char_boundary(end_index) {
            end_index -= 1;
        }
        format!("{}…", &single_line[..end_index])
    }
}

fn build_file_preview(files: &[String]) -> String {
    let mut segments: Vec<String> = files
        .iter()
        .take(3)
        .map(|path| {
            Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| path.clone())
        })
        .collect();

    if files.len() > 3 {
        segments.push(format!("… 等 {} 个文件", files.len()));
    }

    segments.join("\n")
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

#[cfg(windows)]
unsafe fn read_wide_string(ptr: *const u16) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    let mut len = 0usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }

    let slice = std::slice::from_raw_parts(ptr, len);
    Some(String::from_utf16_lossy(slice))
}
