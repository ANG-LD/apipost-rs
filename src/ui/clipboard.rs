//! 剪贴板工具
//!
//! 注意：X11 剪贴板数据存储在设置者进程中，Clipboard 实例必须在数据被粘贴前保持存活。
//! 因此使用全局静态实例。

use std::sync::Mutex;
use std::sync::OnceLock;

static CLIPBOARD: OnceLock<Mutex<Option<arboard::Clipboard>>> = OnceLock::new();

fn get_clipboard() -> &'static Mutex<Option<arboard::Clipboard>> {
    CLIPBOARD.get_or_init(|| Mutex::new(arboard::Clipboard::new().ok()))
}

pub fn copy_to_clipboard(text: &str) -> bool {
    match get_clipboard().lock() {
        Ok(mut guard) => match guard.as_mut() {
            Some(clipboard) => match clipboard.set_text(text.to_string()) {
                Ok(()) => {
                    log::info!("已复制 {} 字节到剪贴板", text.len());
                    true
                }
                Err(e) => {
                    log::error!("剪贴板写入失败: {}", e);
                    false
                }
            },
            None => {
                log::error!("剪贴板不可用");
                false
            }
        },
        Err(e) => {
            log::error!("剪贴板锁获取失败: {}", e);
            false
        }
    }
}
