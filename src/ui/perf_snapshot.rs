//! 性能回归测试：每帧快照的分配量
//!
//! 放在独立文件里是因为 `main_view.rs` 内嵌的 gpui 元素类型嵌套极深，
//! 在该文件里展开 `#[test]` 宏会把 rustc 的宏展开递归撑爆（甚至 SIGSEGV）。
#[cfg(test)]
mod tests {
    use crate::ui::response_highlight::alloc_probe;
    use chrono::Utc;
    use std::sync::Arc;

    /// 造 n 条历史记录，每条带一份 size 字节的响应体
    fn make_history(n: usize, size: usize) -> Arc<Vec<crate::app::database::HistoryEntry>> {
        let body = "x".repeat(size);
        let items = (0..n)
            .map(|i| crate::app::database::HistoryEntry {
                id: format!("id-{i}"),
                method: "GET".to_string(),
                url: format!("https://example.com/{i}"),
                headers: None,
                body: None,
                response_status: Some(200),
                response_headers: None,
                response_body: Some(body.clone()),
                response_time_ms: Some(1),
                response_size: Some(size as i64),
                created_at: Utc::now(),
            })
            .collect();
        Arc::new(items)
    }

    /// 旧写法：每行 clone 整个 entry + 响应体（渲染时才需要，却每帧复制）
    fn snapshot_old(history: &[crate::app::database::HistoryEntry]) -> (usize, usize) {
        let mut urls = 0usize;
        let mut bytes = 0usize;
        for entry in history {
            let entry_clone = entry.clone();
            let response_body = entry.response_body.clone();
            let response_headers = entry.response_headers.clone();
            urls += entry_clone.url.len();
            bytes += response_body.map(|b| b.len()).unwrap_or(0);
            bytes += response_headers.map(|h| h.len()).unwrap_or(0);
        }
        (urls, bytes)
    }

    /// 新写法：共享 Arc + 下标，行快照只做引用计数
    fn snapshot_new(history: &Arc<Vec<crate::app::database::HistoryEntry>>) -> (usize, usize) {
        let mut urls = 0usize;
        let mut bytes = 0usize;
        for (idx, _) in history.iter().enumerate() {
            let row = Arc::clone(history);
            let entry = &row[idx];
            urls += entry.url.len();
            bytes += entry.response_body.as_ref().map(|b| b.len()).unwrap_or(0);
            bytes += entry.response_headers.as_ref().map(|h| h.len()).unwrap_or(0);
        }
        (urls, bytes)
    }

    /// 每帧历史列表快照的分配量对比：30 条记录、每条 1MB 响应体
    #[test]
    fn history_row_snapshot_avoids_copying_bodies() {
        let history = make_history(30, 1024 * 1024);

        let (old_allocs, old_bytes, old_out) = alloc_probe::measure(|| snapshot_old(&history));
        let (new_allocs, new_bytes, new_out) = alloc_probe::measure(|| snapshot_new(&history));

        println!(
            "旧写法: {} 次分配 / {:.1} MB；新写法: {} 次分配 / {} 字节",
            old_allocs,
            old_bytes as f64 / 1024.0 / 1024.0,
            new_allocs,
            new_bytes
        );

        assert_eq!(old_out, new_out, "两种写法的读取结果必须一致");
        assert!(
            new_bytes * 1000 < old_bytes,
            "新写法复制的字节数应比旧写法少 3 个数量级（{} vs {}）",
            new_bytes,
            old_bytes
        );
        assert!(
            new_allocs * 10 <= old_allocs,
            "新写法的分配次数应少一个数量级以上（{} vs {}）",
            new_allocs,
            old_allocs
        );
    }

    // ---------- 第二轮：response / request_tabs 改 Arc 后的每帧快照成本 ----------

    use crate::http::HttpResponse;
    use crate::ui::main_view::{RequestTab, TabState};
    use std::collections::HashMap;

    fn headers(n: usize) -> HashMap<String, String> {
        (0..n)
            .map(|i| (format!("x-header-{i}"), format!("value-{i}")))
            .collect()
    }

    fn response() -> HttpResponse {
        let body = "x".repeat(1024 * 1024);
        HttpResponse {
            status: 200,
            headers: headers(25),
            body: Arc::from(body),
            raw_body: None,
            time_ms: 12,
            size_bytes: 1024 * 1024,
            cookies: Vec::new(),
        }
    }

    fn request_tabs(n: usize) -> Vec<RequestTab> {
        (0..n)
            .map(|i| RequestTab {
                id: i,
                method: "GET".to_string(),
                url: format!("https://api.example.com/v1/resource/{i}"),
                name: format!("请求 {i}"),
                tab_state: TabState::default(),
            })
            .collect()
    }

    /// 响应快照：裸 HttpResponse 每帧会克隆 headers/cookies（几十次分配），
    /// 换成 Arc<HttpResponse> 后每帧只有引用计数。
    #[test]
    fn response_snapshot_is_refcounted_not_copied() {
        let plain = response();
        let shared = Arc::new(response());

        let (old_allocs, old_bytes, _) = alloc_probe::measure(|| {
            for _ in 0..60 {
                let snapshot = Some(plain.clone());
                std::hint::black_box(&snapshot);
            }
        });
        let (new_allocs, new_bytes, _) = alloc_probe::measure(|| {
            for _ in 0..60 {
                let snapshot = Some(Arc::clone(&shared));
                std::hint::black_box(&snapshot);
            }
        });

        println!(
            "响应快照 60 帧：旧 {} 次分配 / {} 字节；新 {} 次分配 / {} 字节",
            old_allocs, old_bytes, new_allocs, new_bytes
        );
        // 用数量级对比而不是「严格为 0」：计数器是全局的，并行跑测试时
        // 其它测试的分配会被计入，绝对值为 0 的断言会随机失败
        assert!(
            new_allocs * 10 <= old_allocs,
            "Arc 快照的分配次数应少一个数量级以上（{} vs {}）",
            new_allocs,
            old_allocs
        );
        assert!(new_bytes * 10 <= old_bytes, "Arc 快照的分配字节数应少一个数量级以上");
        assert!(old_allocs >= 60, "旧写法每帧都在克隆 headers/cookies");
    }

    /// 标签页列表快照：Vec<RequestTab> 每帧克隆所有标签的字符串，
    /// 换成 Arc<Vec<RequestTab>> 后每帧只有引用计数。
    #[test]
    fn request_tabs_snapshot_is_refcounted_not_copied() {
        let plain = request_tabs(10);
        let shared = Arc::new(request_tabs(10));

        let (old_allocs, old_bytes, _) = alloc_probe::measure(|| {
            for _ in 0..60 {
                let snapshot = plain.clone();
                std::hint::black_box(&snapshot);
            }
        });
        let (new_allocs, new_bytes, _) = alloc_probe::measure(|| {
            for _ in 0..60 {
                let snapshot = Arc::clone(&shared);
                std::hint::black_box(&snapshot);
            }
        });

        println!(
            "标签列表快照 60 帧：旧 {} 次分配 / {} 字节；新 {} 次分配 / {} 字节",
            old_allocs, old_bytes, new_allocs, new_bytes
        );
        assert!(
            new_allocs * 10 <= old_allocs,
            "Arc 快照的分配次数应少一个数量级以上（{} vs {}）",
            new_allocs,
            old_allocs
        );
        assert!(old_allocs >= 60, "旧写法每帧都在克隆所有标签");
    }
}
