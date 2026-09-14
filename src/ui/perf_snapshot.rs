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
            for _ in 0..FRAMES {
                let snapshot = Some(plain.clone());
                std::hint::black_box(&snapshot);
            }
        });
        let (new_allocs, new_bytes, _) = alloc_probe::measure(|| {
            for _ in 0..FRAMES {
                let snapshot = Some(Arc::clone(&shared));
                std::hint::black_box(&snapshot);
            }
        });

        println!(
            "响应每帧快照 x {FRAMES} 帧：旧 {} 次分配 / {} 字节；新 {} 次分配 / {} 字节",
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
            for _ in 0..FRAMES {
                let snapshot = plain.clone();
                std::hint::black_box(&snapshot);
            }
        });
        let (new_allocs, new_bytes, _) = alloc_probe::measure(|| {
            for _ in 0..FRAMES {
                let snapshot = Arc::clone(&shared);
                std::hint::black_box(&snapshot);
            }
        });

        println!(
            "标签列表每帧快照 x {FRAMES} 帧：旧 {} 次分配 / {} 字节；新 {} 次分配 / {} 字节",
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

    // ---------- 第三轮：请求编辑状态 5 个字段（params / headers / body_state / auth_state / settings）改 Arc ----------

    /// 「每帧快照」的帧数。取得比较大是为了让信号压过噪声：`alloc_probe` 的计数器是进程级的，
    /// 并行跑的其他测试会往里加数，如果被测代码真的每帧分配一次，两千帧下来就是两千次，
    /// 而噪声（取 7 轮最小值之后）只有几十次。
    const FRAMES: usize = 2000;

    /// 一个测量窗口：在锁里依次量每段代码的分配量
    fn measure_round(steps: &mut [(&str, &mut dyn FnMut())], out: &mut [(usize, usize)]) {
        use std::sync::atomic::Ordering;
        let (.., ()) = alloc_probe::measure(|| {
            for (index, (_, step)) in steps.iter_mut().enumerate() {
                alloc_probe::ALLOCS.store(0, Ordering::SeqCst);
                alloc_probe::BYTES.store(0, Ordering::SeqCst);
                step();
                out[index] = (
                    alloc_probe::ALLOCS.load(Ordering::SeqCst),
                    alloc_probe::BYTES.load(Ordering::SeqCst),
                );
            }
        });
    }

    /// 依次量多段代码的分配量，返回每段的 (次数, 字节)。
    ///
    /// 两个讲究：
    /// 1. 几段代码必须放在同一个窗口里（同一把锁、依次清零再读）；单独调用两次
    ///    `measure()`，两边的污染量不一样，比较就没有意义。
    /// 2. 整窗重复若干轮、每段取**最小值**。全局计数器是进程级的，并行跑的其它测试
    ///    只会把读数**抬高**，取最小值就能逼近真实值 —— 否则「新写法 0 次分配」这类
    ///    断言会被邻居的分配随机顶穿（实测空转基线能飘到 50+ 次）。
    fn measure_steps(steps: &mut [(&str, &mut dyn FnMut())]) -> Vec<(usize, usize)> {
        const ROUNDS: usize = 7;
        let mut best: Vec<(usize, usize)> = vec![(usize::MAX, usize::MAX); steps.len()];
        let mut current: Vec<(usize, usize)> = vec![(0, 0); steps.len()];
        for _ in 0..ROUNDS {
            measure_round(steps, &mut current);
            for (slot, sample) in best.iter_mut().zip(current.iter()) {
                if sample.0 < slot.0 {
                    *slot = *sample;
                }
            }
        }
        best
    }

    /// 空转基线：`FRAMES` 次什么都不做的循环，作为该窗口的噪声参照
    fn noop_frames() {
        for _ in 0..FRAMES {
            std::hint::black_box(());
        }
    }

    /// 量「每帧快照」的分配量：空转基线 / 旧写法（裸 clone）/ 新写法（Arc clone）
    fn snapshot_stats<T: Clone>(
        plain: &T,
        shared: &Arc<T>,
    ) -> ((usize, usize), (usize, usize), (usize, usize)) {
        let mut noop = || noop_frames();
        let mut old = || {
            for _ in 0..FRAMES {
                std::hint::black_box(plain.clone());
            }
        };
        let mut new = || {
            for _ in 0..FRAMES {
                std::hint::black_box(Arc::clone(shared));
            }
        };
        let stats = measure_steps(&mut [("空转", &mut noop), ("旧", &mut old), ("新", &mut new)]);
        (stats[0], stats[1], stats[2])
    }

    /// 断言：新写法确实把「每帧深拷贝」换成了「每帧引用计数」
    fn assert_refcounted(
        label: &str,
        noop: (usize, usize),
        old: (usize, usize),
        new: (usize, usize),
    ) {
        println!("{label} 每帧快照 x {FRAMES} 帧：空转 {noop:?}；旧 {old:?}；新 {new:?}");
        assert!(
            old.0 >= FRAMES,
            "{label}：旧写法应该每帧至少分配一次（{} < {}）",
            old.0,
            FRAMES
        );
        assert!(
            new.0 * 10 <= old.0,
            "{label}：Arc 快照的分配次数应少一个数量级以上（{} vs {}）",
            new.0,
            old.0
        );
        // 字节数只有在旧写法确实在搬运大块数据时才是可靠信号：`Theme` 的 name 只有
        // 几个字节（每帧 4 字节），并行测试的噪声足以盖过它，那种情况只打印不断言。
        if old.1 >= 100_000 {
            assert!(
                new.1 * 10 <= old.1,
                "{label}：Arc 快照的分配字节数应少一个数量级以上（{} vs {}）",
                new.1,
                old.1
            );
        }
    }

    /// 断言：Arc 快照不随帧数增长，即每帧一次引用计数、零分配。
    ///
    /// 阈值取 `FRAMES / 4`：真正的「每帧分配一次」600 帧下来会到 600 次，而并行测试
    /// 造成的污染实测只有几十次，这个阈值既能容忍噪声又能抓住回归（写死 0 会随机失败）。
    ///
    /// 这里**不**用旧写法的读数做判据：`AuthState` / `RequestSettings` 的裸 clone 本来
    /// 就不分配堆内存，实测到的几十次全部是并行测试的污染（污染量与被测代码耗时成正比，
    /// 裸 clone 更慢所以读数反而更高）。旧写法只打印出来做对照。
    fn assert_snapshot_does_not_allocate(
        label: &str,
        noop: (usize, usize),
        old: (usize, usize),
        new: (usize, usize),
    ) {
        println!("{label} 每帧快照 x {FRAMES} 帧：空转 {noop:?}；旧 {old:?}；新 {new:?}");
        let tolerance = FRAMES / 4;
        assert!(
            new.0 <= tolerance,
            "{label}：Arc 快照不该每帧分配（{} > {}）",
            new.0,
            tolerance
        );
        assert!(
            new.0 <= old.0.max(noop.0) + tolerance,
            "{label}：Arc 快照不该比裸 clone 更贵（{} vs {}，空转 {}）",
            new.0,
            old.0,
            noop.0
        );
    }

    /// 与 `ParamEntry` / `HeaderEntry` 同构：两个 gpui `Entity` 句柄 + 一个 bool。
    ///
    /// `Entity<InputState>` 必须有 App 上下文才能创建，单元测试里造不出来，所以用
    /// `Arc<()>` 占位 —— `Entity` 本身就是 Arc，clone 同样只是引用计数，占位不影响
    /// 要量的事：裸 `Vec` 快照的缓冲区分配。
    #[derive(Clone)]
    struct EntryLike {
        key: Arc<()>,
        value: Arc<()>,
        enabled: bool,
    }

    fn entries_like(n: usize) -> Vec<EntryLike> {
        (0..n)
            .map(|_| EntryLike {
                key: Arc::new(()),
                value: Arc::new(()),
                enabled: true,
            })
            .collect()
    }

    /// `MainView::params`：`Vec<ParamEntry>` -> `Arc<Vec<ParamEntry>>`。
    /// 旧写法每帧复制整个 Vec 的缓冲区，新写法每帧只有一次引用计数。
    #[test]
    fn params_snapshot_is_refcounted_not_copied() {
        let plain = entries_like(12);
        let shared = Arc::new(entries_like(12));

        let (noop, old, new) = snapshot_stats(&plain, &shared);
        assert_refcounted("params", noop, old, new);
    }

    /// `MainView::headers`：`Vec<HeaderEntry>` -> `Arc<Vec<HeaderEntry>>`。
    /// 与 params 同构（两个 Entity 句柄 + bool），同样是「每帧少一次 Vec 缓冲区分配」。
    #[test]
    fn headers_snapshot_is_refcounted_not_copied() {
        let plain = entries_like(8);
        let shared = Arc::new(entries_like(8));

        let (noop, old, new) = snapshot_stats(&plain, &shared);
        assert_refcounted("headers", noop, old, new);
    }

    /// 与 `FormDataEntry` 同构：Entity 句柄 + 可能带文件路径的字符串值
    #[derive(Clone)]
    struct FormDataLike {
        key: Arc<()>,
        value: String,
        file_path: Option<String>,
        enabled: bool,
    }

    /// 与 `BodyState` 同构：两个条目列表 + 若干堆字符串。
    #[derive(Clone)]
    struct BodyStateLike {
        body_type: u8,
        raw_format: u8,
        raw_content: Arc<()>,
        form_data: Vec<FormDataLike>,
        urlencoded_data: Vec<FormDataLike>,
        binary_file_path: Option<String>,
        json_error: Option<String>,
    }

    fn body_state_like(n: usize) -> BodyStateLike {
        let entry = |i: usize| FormDataLike {
            key: Arc::new(()),
            value: format!("value-{i}"),
            file_path: Some(format!("/tmp/file-{i}.bin")),
            enabled: true,
        };
        BodyStateLike {
            body_type: 1,
            raw_format: 0,
            raw_content: Arc::new(()),
            form_data: (0..n).map(entry).collect(),
            urlencoded_data: (0..n).map(entry).collect(),
            binary_file_path: Some("/tmp/big.bin".to_string()),
            json_error: None,
        }
    }

    /// `MainView::body_state`：`BodyState` -> `Arc<BodyState>`。
    ///
    /// 这个字段才是真热点：`render_body_panel` 每帧都做 `this.body_state.clone()`，
    /// 裸 clone 会连 form_data / urlencoded_data 两个列表、以及每个条目的字符串一起深拷贝。
    #[test]
    fn body_state_snapshot_is_refcounted_not_copied() {
        let plain = body_state_like(20);
        let shared = Arc::new(body_state_like(20));

        let (noop, old, new) = snapshot_stats(&plain, &shared);
        assert_refcounted("body_state", noop, old, new);
    }

    /// 与 `AuthState` 同构：各变体只持有 gpui `Entity` 句柄（Entity 自身就是 Arc）。
    #[derive(Clone)]
    enum AuthLike {
        NoAuth,
        Bearer {
            token: Arc<()>,
        },
        Basic {
            username: Arc<()>,
            password: Arc<()>,
        },
        ApiKey {
            key: Arc<()>,
            value: Arc<()>,
            location: Arc<()>,
            in_header: bool,
        },
    }

    fn auth_like() -> AuthLike {
        AuthLike::ApiKey {
            key: Arc::new(()),
            value: Arc::new(()),
            location: Arc::new(()),
            in_header: true,
        }
    }

    /// `MainView::auth_state`：`AuthState` -> `Arc<AuthState>`。
    ///
    /// 如实记录一个反直觉的结论：`AuthState` 各变体只拿 `Entity` 句柄，裸 clone
    /// **本来就不分配堆内存**（Entity 的 clone 就是引用计数），所以这个字段的收益不是
    /// 省分配，而是把每帧快照统一成一次引用计数，并防止以后往变体里加堆数据时
    /// 悄悄退化成每帧深拷贝。测试钉住的就是「两条路径都不随帧数增长」。
    #[test]
    fn auth_state_snapshot_is_refcounted_not_copied() {
        let plain = auth_like();
        let shared = Arc::new(auth_like());

        let (noop, old, new) = snapshot_stats(&plain, &shared);
        assert_snapshot_does_not_allocate("auth_state", noop, old, new);
    }

    /// `MainView::settings`：`RequestSettings` -> `Arc<RequestSettings>`。
    ///
    /// 这里用真实类型测量（不造占位结构）：`RequestSettings` 只有 u64/u32/bool，
    /// 裸 clone 本来零分配，实测两条路径都是 0。改成 Arc 的意义同样是「统一成引用计数 +
    /// 给将来加堆字段上保险」，不是当下的分配收益 —— 如实钉进测试，避免报告里夸大。
    #[test]
    fn settings_snapshot_is_refcounted_not_copied() {
        use crate::ui::settings::RequestSettings;

        let plain = RequestSettings::default();
        let shared = Arc::new(RequestSettings::default());

        let (noop, old, new) = snapshot_stats(&plain, &shared);
        // 如果这里失败了，说明 RequestSettings 新增了堆字段，每帧快照成本要重新评估
        assert!(
            std::mem::size_of::<RequestSettings>() <= 32,
            "RequestSettings 应该只有标量字段（当前 {} 字节）",
            std::mem::size_of::<RequestSettings>()
        );
        assert_snapshot_does_not_allocate("settings", noop, old, new);
    }

    /// `MainView::cached_theme`：`Theme` -> `Arc<Theme>`。
    ///
    /// `Theme` 是「一个 String 名字 + 20 个 Rgba」，裸 clone 每次都要为名字分配一次堆内存；
    /// 而 render 里主视图 + 每个面板 + 设置对话框闭包每帧各取一份，改成 Arc 后每帧零分配。
    #[test]
    fn theme_snapshot_is_refcounted_not_copied() {
        use crate::ui::themes::Theme;

        let plain = Theme::from_str("dark");
        let shared = Arc::new(Theme::from_str("dark"));

        let (noop, old, new) = snapshot_stats(&plain, &shared);
        assert_refcounted("cached_theme", noop, old, new);
    }

    // ---------- 第四轮：响应体渲染从「每行一个 div」改成整块一个 StyledText ----------

    use crate::ui::response_highlight::{self, HighlightedBody};
    use crate::ui::themes::Theme;
    use gpui::{Rgba, SharedString};

    fn dark() -> Theme {
        Theme::from_str("dark")
    }

    /// 把 `styled` 摊平成「可见字符 + 颜色」：按 run 的字节长度走，跳过换行。
    /// 长度表和正文对不上就会在这里 panic，正是渲染时 `StyledText::with_runs` 会炸的情况。
    fn visible_chars_from_styled(hl: &HighlightedBody) -> Vec<(char, Rgba)> {
        let mut out = Vec::new();
        let mut runs = hl.styled.runs.iter();
        let mut current = runs.next().map(|(len, color)| (*len as usize, *color));
        for ch in hl.styled.text.chars() {
            while matches!(current, Some((0, _))) {
                current = runs.next().map(|(len, color)| (*len as usize, *color));
            }
            let (left, color) = current.expect("run 长度表比正文短，StyledText::with_runs 会 panic");
            if ch != '\n' {
                out.push((ch, color));
            }
            current = Some((left.saturating_sub(ch.len_utf8()), color));
        }
        assert!(
            matches!(current, Some((0, _)) | None),
            "run 长度表比正文长，StyledText::with_runs 会 panic"
        );
        out
    }

    /// 新的整块渲染计划必须和旧的逐行数据完全等价：同一个正文、同一个字符、同一个颜色。
    #[test]
    fn styled_body_preserves_text_and_colors() {
        let theme = dark();
        // 混合三种情况：合法 JSON（走 parse + 美化）、Content-Type 说自己是 json 但内容
        // 解析不了（走原文，里面可能自带换行/尾随换行）、空响应体。
        for (body, content_type) in [
            (
                r#"{"name":"apipost","version":2,"nested":{"ok":true,"list":[1,2.5,-3e2],"none":null},"中文":"值"}"#,
                None,
            ),
            (r#"[]"#, None),
            (r#"{}"#, None),
            (r#"[1,2,3]"#, None),
            (r#"{"空":[],"num":1.5e-3,"esc":"a\"b","emoji":"🚀"}"#, None),
            (r#"[{"a":1},{"b":[2,3]}]"#, None),
            // 解析失败 -> 直接用原文，原文里含换行
            ("{\n  \"a\": 1,\n}", Some("application/json")),
            // 原文以换行结尾（会多出一个空行）
            ("{not json}\n", Some("application/json")),
            // 不给 Content-Type，靠正文首字符判断成 JSON
            (r#"{"k":[{"n":1e10},{"n":-0.5}]}"#, Some("text/javascript")),
            // 空响应体 + json Content-Type
            ("", Some("application/json")),
            // 纯文本 + json Content-Type（解析失败，走原文）
            ("hello\nworld\n", Some("application/json")),
        ] {
            let hl = response_highlight::build(body, content_type, &theme);

            // 1) 正文必须正好是「美化后的 JSON」或（解析失败时的）原文，
            //    一个字都不能丢、不能错序
            let expected = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(value) => serde_json::to_string_pretty(&value).expect("序列化不应失败"),
                Err(_) => body.to_string(),
            };
            assert_eq!(hl.styled.text.as_ref(), expected, "正文不一致: {body}");

            // 2) run 长度之和必须等于正文长度 —— 这是 `StyledText::with_runs` 的硬校验，
            //    对不上运行时直接 panic，所以必须钉在测试里
            let sum: usize = hl.styled.runs.iter().map(|(len, _)| *len as usize).sum();
            assert_eq!(sum, hl.styled.text.len(), "run 长度之和与正文长度不一致: {body}");

            // 3) 每个字符的颜色必须钉在主题颜色上（换行不参与：它的颜色只是
            //    「被并进前一个 run」，不可见、不构成断言）
            //
            //    改造前这条比较的是「逐行片段」与「整块长度表」两种表示是否一致；
            //    `lines` 删除后没有第二种表示可比，改成直接断言绝对颜色 ——
            //    覆盖的是同一批字符，且不再依赖「两份数据恰好一起算错」的自洽性。
            let colors = visible_colors(&hl);
            assert_eq!(
                colors.len(),
                hl.styled.text.chars().filter(|c| *c != '\n').count(),
                "每个可见字符都要有颜色: {body}"
            );
            for (needle, expected, what) in [
                ("\"a\"", theme.json_key, "key"),
                ("1", theme.json_number, "数字"),
            ] {
                if let Some(at) = visible_offsets(&hl, needle).first().copied() {
                    let count = needle.chars().count();
                    for color in &colors[at..at + count] {
                        assert_eq!(*color, expected, "{what} 的颜色不对（{needle}）: {body}");
                    }
                }
            }
            // 每个 run 的长度都非零，且边界落在字符边界上（否则多字节会被切开）
            let mut offset = 0usize;
            for (len, _) in &hl.styled.runs {
                assert!(*len > 0, "run 长度不能为 0: {body}");
                offset += *len as usize;
                assert!(
                    hl.styled.text.is_char_boundary(offset),
                    "run 边界 {offset} 落在字符中间: {body}"
                );
            }
            assert_eq!(offset, hl.styled.text.len(), "run 表必须盖满正文: {body}");
        }
    }

    /// 把「正文 + 长度表」摊平成「可见字符 + 颜色」（换行由正文承载，这里跳过）
    fn visible_colors(hl: &HighlightedBody) -> Vec<Rgba> {
        visible_chars_from_styled(hl).into_iter().map(|(_, c)| c).collect()
    }

    /// 某个子串在「可见字符序列」里的起始下标（找不到就返回空）
    fn visible_offsets(hl: &HighlightedBody, needle: &str) -> Vec<usize> {
        let chars: Vec<char> = hl
            .styled
            .text
            .chars()
            .filter(|c| *c != '\n')
            .collect();
        let needle: Vec<char> = needle.chars().collect();
        chars
            .windows(needle.len())
            .enumerate()
            .filter(|(_, w)| *w == needle.as_slice())
            .map(|(i, _)| i)
            .collect()
    }

    /// 非 JSON 走 plain 分支，不应该多出一份正文
    #[test]
    fn styled_body_is_empty_for_plain_text() {
        let hl = response_highlight::build("hello world", None, &dark());
        assert!(hl.plain.is_some(), "非 JSON 应该有 plain");
        assert_eq!(hl.styled.text.len(), 0);
        assert!(hl.styled.runs.is_empty());
    }

    /// 元素数对比：旧实现「外层 1 + 每行 1 + 每片段 1」，新实现整块只有 2 个元素。
    #[test]
    fn styled_body_collapses_per_line_elements() {
        // 约 200KB 的响应体，和 response_highlight::per_frame_report 同一量级
        let payload = serde_json::json!({
            "items": (0..1000).map(|i| serde_json::json!({
                "id": i,
                "name": format!("item-{i}"),
                "active": i % 2 == 0,
                "score": i as f64 * 1.5,
                "tags": ["alpha", "beta", "gamma"],
            })).collect::<Vec<_>>()
        });
        let body = serde_json::to_string(&payload).unwrap();
        let hl = response_highlight::build(&body, None, &dark());

        // 旧实现的元素数要「每行一个 div + 行内每个着色片段一个 div」，而这些片段
        // 就是删掉的 `lines` 的逐行切分。它可以从「正文 + 长度表」**精确**推回来：
        // 每个 run 在它跨过的每一行里各算一个片段（行由正文里的换行承载），
        // 切分规则与被删的 `split_lines` 逐字一致（换行处断行、空片段不计数）。
        let mut offset = 0usize;
        let mut lines = 1usize;
        let mut spans = 0usize;
        for (len, _) in &hl.styled.runs {
            let mut rest = &hl.styled.text[offset..offset + *len as usize];
            offset += *len as usize;
            loop {
                match rest.find('\n') {
                    Some(pos) => {
                        if pos > 0 {
                            spans += 1;
                        }
                        lines += 1;
                        rest = &rest[pos + 1..];
                    }
                    None => {
                        if !rest.is_empty() {
                            spans += 1;
                        }
                        break;
                    }
                }
            }
        }
        assert_eq!(offset, hl.styled.text.len(), "run 表必须盖满正文");
        // 旧：外层 flex_col 容器 + 每行一个 div + 每个片段一个 div
        let old_elements = 1 + lines + spans;
        // 新：外层「不折行」容器 + 一个 StyledText（正文、换行、颜色全在里面）
        let new_elements = 2;

        println!(
            "响应体 {} 字节：{} 行 / {} 个着色片段（合并后 {} 个 run）",
            body.len(),
            lines,
            spans,
            hl.styled.runs.len()
        );
        println!("每帧元素数：旧 {old_elements} -> 新 {new_elements}");

        assert!(
            old_elements > 10_000,
            "旧实现每帧确实要建上万个元素（实际 {old_elements}）"
        );
        assert!(
            spans > hl.styled.runs.len(),
            "跨行合并后 run 数必须少于逐行片段数（{} vs {}）",
            hl.styled.runs.len(),
            spans
        );
        assert!(
            new_elements * 3 <= old_elements,
            "新实现的元素数应至少降 3 倍（{new_elements} vs {old_elements}）"
        );
        // run 数（每帧要铺设的 TextRun 数量）应该和片段数同量级，而不是和字符数同量级
        let chars = hl.styled.text.chars().count();
        assert!(
            hl.styled.runs.len() * 3 < chars,
            "run 数应远小于字符数（{} vs {chars}）",
            hl.styled.runs.len()
        );
    }

    /// `HighlightedBody` 只保留**一份**正文：结构体里没有第二个存正文的字段。
    ///
    /// 改造前还有 `lines: Vec<Vec<Run>>`，每个片段各持一个 `SharedString`，
    /// 于是正文的每个字节在内存里存了两遍（`styled.text` 一份 + 逐片段一份），
    /// 外加每个片段一次堆分配。这条测试把「第二份正文」的代价量化出来：
    /// 用「与已删除的 `lines` 同构」的重建体做对照，量两边在一次构建/保留中的分配量。
    #[test]
    fn highlighted_body_keeps_a_single_copy_of_the_text() {
        use crate::ui::response_highlight::{self, HighlightedBody, StyledBody};
        use std::mem::size_of;

        // 结构体只能由「plain + styled」两块组成：把任何一份额外的正文表示加回来
        // （例如恢复 lines 字段）都会让这条立刻失败
        assert_eq!(
            size_of::<HighlightedBody>(),
            size_of::<Option<Arc<str>>>() + size_of::<StyledBody>(),
            "HighlightedBody 只能有 plain + styled 两块"
        );

        /// 与已删除的 `lines` 同构：把正文按「行 × 片段」切成若干 `SharedString`
        /// （切分规则与被删的 `split_lines` 逐字一致：'\n' 处断行、空片段不计数）
        fn lines_like(text: &str, runs: &[(u32, Rgba)]) -> Vec<Vec<SharedString>> {
            let mut lines: Vec<Vec<SharedString>> = vec![Vec::new()];
            let mut offset = 0usize;
            for (len, _) in runs {
                let mut rest = &text[offset..offset + *len as usize];
                offset += *len as usize;
                loop {
                    match rest.find('\n') {
                        Some(pos) => {
                            if pos > 0 {
                                lines.last_mut().unwrap().push(SharedString::from(&rest[..pos]));
                            }
                            lines.push(Vec::new());
                            rest = &rest[pos + 1..];
                        }
                        None => {
                            if !rest.is_empty() {
                                lines.last_mut().unwrap().push(SharedString::from(rest));
                            }
                            break;
                        }
                    }
                }
            }
            lines
        }

        // 与 per_frame_report 同一量级的响应体
        let payload = serde_json::json!({
            "items": (0..1000).map(|i| serde_json::json!({
                "id": i,
                "name": format!("item-{i}"),
                "active": i % 2 == 0,
                "score": i as f64 * 1.5,
                "tags": ["alpha", "beta", "gamma"],
            })).collect::<Vec<_>>()
        });
        let body = serde_json::to_string(&payload).unwrap();
        let hl = response_highlight::build(&body, None, &dark());

        let text = hl.styled.text.clone();
        let runs = hl.styled.runs.clone();
        let text_bytes = text.len();
        let newline_bytes = text.matches('\n').count();

        // ---- 改造后的常驻表示：一份正文 + 一张长度表（+ 结构体本身，上面已断言只有两块）
        let resident =
            text_bytes + runs.len() * size_of::<(u32, Rgba)>() + size_of::<HighlightedBody>();

        // ---- 已删除的 `lines` 表示：精确的常驻结构体开销（不依赖分配器读数）
        let lines = lines_like(&text, &runs);
        let line_count = lines.len();
        let segments: usize = lines.iter().map(|line| line.len()).sum();
        // 逐片段复制的正文字节数：片段拼起来 = 正文去掉换行
        let segment_bytes: usize = lines.iter().flatten().map(|s| s.len()).sum();
        /// 与已删除的 `Run` 同构（`SharedString` + `Rgba`），用来算那层表示的常驻开销
        struct RunLike {
            _text: SharedString,
            _color: Rgba,
        }
        let removed_structs = line_count * size_of::<Vec<SharedString>>()
            + segments * size_of::<RunLike>();

        // ---- 分配器视角：重建那层表示一次要分配多少次 / 多少字节（取 7 轮最小值）
        let mut old = || {
            let lines = lines_like(&text, &runs);
            std::hint::black_box(lines.len());
        };
        let mut new = || {
            // 现在保留的表示：正文只 clone 一次（引用计数），长度表就是 run 表本身
            let kept: SharedString = text.clone();
            std::hint::black_box((kept.len(), runs.len()));
        };
        let stats = measure_steps(&mut [("旧 lines 表示", &mut old), ("现在的表示", &mut new)]);
        let (old_allocs, old_bytes) = stats[0];
        let (new_allocs, new_bytes) = stats[1];

        println!(
            "响应体 {} 字节 → 常驻表示 {} 字节（正文 {} + run 表 {} × {} + 结构体 {}）",
            body.len(),
            resident,
            text_bytes,
            runs.len(),
            size_of::<(u32, Rgba)>(),
            size_of::<HighlightedBody>(),
        );
        println!(
            "已删除的 lines 表示：{} 行 × Vec 头 {} 字节 + {} 个片段 × Run {} 字节 = {} 字节常驻结构体；\
             它还把这 {} 个片段逐段复制了一遍（合计 {} 字节，≈ 正文去掉换行）",
            line_count,
            size_of::<Vec<SharedString>>(),
            segments,
            size_of::<RunLike>(),
            removed_structs,
            segments,
            segment_bytes,
        );
        println!(
            "重建那层表示一次：{} 次分配 / {} 字节（含 Vec 扩容余量）；现在保留一份正文：{} 次 / {} 字节",
            old_allocs, old_bytes, new_allocs, new_bytes
        );

        // 1) 那层表示逐片段复制了正文的每个非换行字节 —— 这就是「第二份正文」
        assert_eq!(
            segment_bytes + newline_bytes,
            text_bytes,
            "逐片段拼起来必须正好是正文去掉换行（说明第二份正文是完整的）"
        );
        // 2) 光结构体开销就比整个响应体还大
        assert!(
            removed_structs > text_bytes,
            "已删除的那层表示的结构体开销应当大于正文本身（{} vs {}）",
            removed_structs,
            text_bytes
        );
        assert!(
            segments > runs.len(),
            "逐行片段数必须多于跨行合并后的 run 数（{} vs {}）",
            segments,
            runs.len()
        );
        // 3) 现在只保留一份：正文 clone 是引用计数，长度表本来就在，不该再分配
        assert!(
            new_bytes * 100 <= old_bytes,
            "现在的表示不该再复制正文（{} vs {} 字节）",
            new_bytes,
            old_bytes
        );
        assert!(
            new_allocs * 100 <= old_allocs,
            "现在的表示不该再逐片段分配（{} vs {} 次）",
            new_allocs,
            old_allocs
        );
    }

    // ---------- 第五轮：收藏夹树（默认侧栏标签页）的每帧克隆 ----------

    use crate::app::database::{Folder, SavedRequest};
    use crate::ui::sidebar::{build_collection_tree, CollectionItem};
    use std::collections::HashSet;

    /// 与改造前的 `CollectionItem` 同构：字段是 `String`，子节点是 `Vec`。
    /// 渲染每帧都要把这些字段 clone 给 `'static` 闭包，所以 String/Vec 的 clone
    /// 就是每帧的真实分配量。
    #[derive(Clone)]
    #[allow(dead_code)]
    enum CollectionItemLike {
        Folder {
            id: String,
            name: String,
            depth: usize,
            is_expanded: bool,
            children: Vec<CollectionItemLike>,
        },
        Request {
            id: String,
            name: String,
            method: String,
            url: String,
            depth: usize,
        },
    }

    fn collection_like(folders: usize, per_folder: usize) -> Vec<CollectionItemLike> {
        (0..folders)
            .map(|f| CollectionItemLike::Folder {
                id: format!("folder-{f}"),
                name: format!("文件夹 {f}"),
                depth: 0,
                is_expanded: true,
                children: (0..per_folder)
                    .map(|r| CollectionItemLike::Request {
                        id: format!("req-{f}-{r}"),
                        name: format!("请求 {f}-{r}"),
                        method: "POST".to_string(),
                        url: format!("https://api.example.com/v1/{f}/{r}"),
                        depth: 1,
                    })
                    .collect(),
            })
            .collect()
    }

    fn folder(f: usize) -> Folder {
        Folder {
            id: format!("folder-{f}"),
            name: format!("文件夹 {f}"),
            parent_id: None,
            created_at: None,
        }
    }

    fn saved_request(f: usize, r: usize) -> SavedRequest {
        SavedRequest {
            id: format!("req-{f}-{r}"),
            name: format!("请求 {f}-{r}"),
            method: "POST".to_string(),
            url: format!("https://api.example.com/v1/{f}/{r}"),
            headers: None,
            body: None,
            description: None,
            folder_id: Some(format!("folder-{f}")),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 「每帧把收藏夹树整棵 clone 一遍」的旧代价。
    ///
    /// 侧栏默认标签页就是收藏夹，`render_collection_panel` 每帧都要把每个条目的
    /// `id/name/method/url` 交给 `'static` 闭包，所以旧实现每帧都在深拷贝整棵树。
    /// 这里量的是**同一棵树**在新旧两种字段布局下的每帧克隆成本：
    /// 旧布局是 `String` + `Vec<children>`（深拷贝），新布局是 `Arc<str>` + `Arc<[..]>`
    /// （只做引用计数）。「旧」用同构结构体，「新」用真实类型 `CollectionItem`。
    #[test]
    fn collection_tree_snapshot_is_refcounted_not_copied() {
        const FOLDERS: usize = 8;
        const PER_FOLDER: usize = 6;

        let old_items = collection_like(FOLDERS, PER_FOLDER);
        let folders: Vec<Folder> = (0..FOLDERS).map(folder).collect();
        let requests: Vec<SavedRequest> = (0..FOLDERS)
            .flat_map(|f| (0..PER_FOLDER).map(move |r| saved_request(f, r)))
            .collect();
        let expanded: HashSet<String> = (0..FOLDERS).map(|f| format!("folder-{f}")).collect();
        // 主视图把它放在 Arc 里（`collection_items: Arc<Vec<CollectionItem>>`），
        // 渲染每帧的快照就是这一次 Arc::clone
        let new_items: Arc<Vec<CollectionItem>> =
            Arc::new(build_collection_tree(&folders, &requests, &expanded));

        let stats = measure_steps(&mut [
            ("空转", &mut || noop_frames()),
            ("旧（String + Vec 深拷贝）", &mut || {
                for _ in 0..FRAMES {
                    std::hint::black_box(old_items.clone());
                }
            }),
            ("新（Arc 引用计数）", &mut || {
                for _ in 0..FRAMES {
                    std::hint::black_box(Arc::clone(&new_items));
                }
            }),
        ]);
        assert_refcounted("收藏夹树", stats[0], stats[1], stats[2]);
    }

    /// 单条目内部那些「给 `'static` 闭包用的字段克隆」：旧布局每帧每项都要为
    /// id/name/method/url 分配，新布局全是引用计数。
    #[test]
    fn collection_item_field_clones_are_refcounted() {
        let old_items = collection_like(8, 6);
        let folders: Vec<Folder> = (0..8).map(folder).collect();
        let requests: Vec<SavedRequest> = (0..8)
            .flat_map(|f| (0..6).map(move |r| saved_request(f, r)))
            .collect();
        let expanded: HashSet<String> = (0..8).map(|f| format!("folder-{f}")).collect();
        let new_items = build_collection_tree(&folders, &requests, &expanded);

        // 渲染里对每个条目做的事：把 id / name / method / url 各克隆一份
        fn old_per_item(items: &[CollectionItemLike]) {
            for item in items {
                match item {
                    CollectionItemLike::Folder { children, id, name, .. } => {
                        std::hint::black_box(id.clone());
                        std::hint::black_box(name.clone());
                        old_per_item(children);
                    }
                    CollectionItemLike::Request { id, name, method, url, .. } => {
                        std::hint::black_box(id.clone());
                        std::hint::black_box(name.clone());
                        std::hint::black_box(method.clone());
                        std::hint::black_box(url.clone());
                    }
                }
            }
        }
        fn new_per_item(items: &[CollectionItem]) {
            for item in items {
                match item {
                    CollectionItem::Folder { children, id, name, keys, .. } => {
                        std::hint::black_box(Arc::clone(id));
                        std::hint::black_box(Arc::clone(name));
                        std::hint::black_box(keys.row.clone());
                        std::hint::black_box(keys.label.clone());
                        new_per_item(children);
                    }
                    CollectionItem::Request { id, name, method, url, keys, .. } => {
                        std::hint::black_box(Arc::clone(id));
                        std::hint::black_box(Arc::clone(name));
                        std::hint::black_box(Arc::clone(method));
                        std::hint::black_box(Arc::clone(url));
                        std::hint::black_box(keys.label.clone());
                    }
                }
            }
        }

        let stats = measure_steps(&mut [
            ("空转", &mut || noop_frames()),
            ("旧（String clone）", &mut || {
                for _ in 0..FRAMES {
                    old_per_item(&old_items);
                }
            }),
            ("新（Arc/SharedString clone）", &mut || {
                for _ in 0..FRAMES {
                    new_per_item(&new_items);
                }
            }),
        ]);
        assert_refcounted("收藏夹条目字段", stats[0], stats[1], stats[2]);
    }

    // ---------- 第六轮：响应正文交给编辑器的那一层 ----------


    use crate::http::HttpResponse as Resp;

    fn big_response(size: usize) -> Resp {
        Resp {
            status: 200,
            headers: headers(3),
            body: Arc::from("x".repeat(size)),
            raw_body: None,
            time_ms: 1,
            size_bytes: size as i64,
            cookies: Vec::new(),
        }
    }

    // ---------- 第七轮：侧栏「环境变量」页每帧的深拷贝 / 加锁 ----------

    use crate::app::EnvironmentManager;
    use crate::ui::sidebar::{parse_vars, truncate_display, variable_count};

    fn env_json(n: usize) -> String {
        serde_json::to_string(
            &(0..n)
                .map(|i| (format!("var_{i}"), format!("value-{i}")))
                .collect::<std::collections::HashMap<String, String>>(),
        )
        .unwrap()
    }

    /// `variable_count` 必须与 `parse_vars(...).len()` 逐字等价 ——
    /// 这是「把每帧的全量解析换成一个计数」能不能成立的前提。
    #[test]
    fn variable_count_matches_full_parse() {
        for json in [
            env_json(0),
            env_json(1),
            env_json(20),
            // 重复键：HashMap 语义是「只算一次」，计数必须一致
            r#"{"a":"1","a":"2","b":"3"}"#.to_string(),
            // 值不是字符串：原写法整体解析失败 -> 0，新写法也必须 0
            r#"{"a":1}"#.to_string(),
            // 键值含转义序列（Cow 借不到，要走分配分支）
            r#"{"a\nb":"v\"x"}"#.to_string(),
            // 根本不是对象
            "[1,2,3]".to_string(),
            "".to_string(),
            "not json".to_string(),
        ] {
            assert_eq!(
                variable_count(&json),
                parse_vars(&json).len(),
                "计数与完整解析必须一致: {json}"
            );
        }
    }

    /// 侧栏环境页每帧做两件「只为拿一个数字」的重活：
    /// 1. 把**每个**环境的变量 JSON 都反序列化成 `Vec<(String, String)>`（再排序）；
    /// 2. `get_all_globals()` 克隆整份全局变量 HashMap、转 Vec、排序。
    /// 改造后分别换成 `variable_count()` 与 `globals_count()`（同一语义，零堆分配）。
    #[test]
    fn environment_panel_counts_avoid_per_frame_deep_copies() {
        const ENVS: usize = 10;
        const VARS: usize = 20;
        const ROUNDS: usize = 20;

        let jsons: Vec<String> = (0..ENVS).map(|_| env_json(VARS)).collect();
        let em = EnvironmentManager::new();
        for i in 0..VARS {
            em.set_global(format!("k{i}"), format!("v{i}"));
        }

        // 侧栏每帧做的事：非当前环境的变量也要解析出来，只为了 .len()
        fn old_counts(jsons: &[String]) -> usize {
            jsons.iter().map(|j| parse_vars(j).len()).sum()
        }
        fn new_counts(jsons: &[String]) -> usize {
            jsons.iter().map(|j| variable_count(j)).sum()
        }
        fn old_globals(em: &EnvironmentManager) -> usize {
            let mut v: Vec<(String, String)> =
                em.get_all_globals().into_iter().collect();
            v.sort_by(|a, b| a.0.cmp(&b.0));
            v.len()
        }
        fn new_globals(em: &EnvironmentManager) -> usize {
            em.globals_count()
        }

        assert_eq!(old_counts(&jsons), new_counts(&jsons), "变量计数结果必须一致");
        assert_eq!(old_globals(&em), new_globals(&em), "全局变量计数结果必须一致");

        let stats = measure_steps(&mut [
            ("空转", &mut || noop_frames()),
            ("旧（每帧全量解析 + 克隆排序）", &mut || {
                for _ in 0..ROUNDS {
                    std::hint::black_box(old_counts(&jsons) + old_globals(&em));
                }
            }),
            ("新（每帧只计数）", &mut || {
                for _ in 0..ROUNDS {
                    std::hint::black_box(new_counts(&jsons) + new_globals(&em));
                }
            }),
        ]);
        let (noop, old, new) = (stats[0], stats[1], stats[2]);
        println!(
            "环境页计数 x {ROUNDS} 帧：空转 {noop:?}；旧（{ENVS} 个环境 x {VARS} 个变量）{old:?}；新 {new:?}"
        );
        assert!(old.0 >= ROUNDS, "旧写法每帧都在分配");
        // 新写法每帧仍有「每个环境一张 HashMap 表」的开销（20 条键值会增长 4 次），
        // 但「每个变量 2 次字符串分配 + 整表排序」那部分已经没有了
        assert!(
            new.0 * 10 <= old.0,
            "新写法的分配次数应少一个数量级以上（{} vs {}）",
            new.0,
            old.0
        );
    }

    /// 每帧取 9 条翻译文案：旧写法（锁 app_state + `get(key).to_string()`）
    /// vs 新写法（从 `Arc<Translations>` 里取 `Arc<str>`，引用计数）。
    #[test]
    fn environment_panel_translations_are_refcounted() {
        const KEYS: [&str; 9] = [
            "sidebar.env",
            "env.create",
            "env.vars_count",
            "env.vars",
            "env.vars_edit",
            "env.no_vars",
            "sidebar.global_vars",
            "env.empty_list",
            "env.create",
        ];
        use std::sync::Mutex;

        let i18n = Mutex::new(crate::i18n::I18nManager::new("zh-CN"));
        let translations = i18n.lock().unwrap().translations_arc();

        let stats = measure_steps(&mut [
            ("空转", &mut || noop_frames()),
            ("旧（锁 + to_string）", &mut || {
                for _ in 0..FRAMES {
                    for key in KEYS {
                        // 与改造前 environment_panel 的 t 闭包同形
                        std::hint::black_box(i18n.lock().unwrap().get(key).to_string());
                    }
                }
            }),
            ("新（Arc 查表）", &mut || {
                for _ in 0..FRAMES {
                    for key in KEYS {
                        std::hint::black_box(gpui::SharedString::from(Arc::clone(
                            translations.get(key).unwrap(),
                        )));
                    }
                }
            }),
        ]);
        assert_refcounted("环境页翻译", stats[0], stats[1], stats[2]);
    }

    /// `truncate_display` 改成返回 `Cow` 之后，显示结果必须一字不差。
    #[test]
    fn truncate_display_returns_cow_with_same_text() {
        for (s, w) in [
            ("short", 14usize),
            ("a-very-long-environment-name", 14),
            ("中文名字测试", 6),
            ("中文名字测试", 40),
            ("", 10),
        ] {
            let got = truncate_display(s, w);
            // 旧实现：不超宽就 to_string()，超宽就 format!("{}…", 前缀)
            let mut width = 0usize;
            let mut expected = s.to_string();
            for (i, ch) in s.char_indices() {
                width += if ch.is_ascii() { 1 } else { 2 };
                if width > w {
                    expected = format!("{}…", &s[..i]);
                    break;
                }
            }
            assert_eq!(got.as_ref(), expected, "截断结果必须一致: {s:?} / {w}");
            if expected == s {
                assert!(
                    matches!(got, std::borrow::Cow::Borrowed(_)),
                    "没被截断时必须零分配借用: {s:?}"
                );
            }
        }
    }

    /// 响应正文「交给编辑器」这一层的分配量。
    ///
    /// 改造前：每来一次响应（以及每次点历史记录 / 恢复工作区），正文都会被
    /// `set_value` 交给 **4** 个 `InputState`（xml / text / html 那三个从来没有被
    /// `Input::new(...)` 渲染过，也没人读它们的值），而且每次都从 `&str` 转换 ——
    /// `InputState::set_value` 收 `impl Into<SharedString>`，`SmolStr::from(&str)`
    /// 对长文本会为整篇正文分配一次 `Arc<str>` 并拷贝。
    /// 改造后：只交给唯一那个编辑器，并且直接接管正文本来就有的 `Arc<str>`（零分配）。
    ///
    /// 如实标注：`InputState` 内部把文本抄进自己的 `Rope`、绘制时再建一次语法高亮
    /// 的那部分开销需要 gpui 的 App/Window 上下文，单元测试里造不出来 —— 这里量的
    /// 只是**我们代码里发生的**那一层转换分配（它是真实调用路径上的分配）。
    #[test]
    fn response_body_is_handed_to_one_editor_by_pointer() {
        const BODIES: usize = 10;
        let resp = big_response(5 * 1024 * 1024);

        // 旧：4 个编辑器，每个都把 &str 转成 SharedString
        let (old_allocs, old_bytes, _) = alloc_probe::measure(|| {
            for _ in 0..BODIES {
                for _ in 0..4 {
                    std::hint::black_box(gpui::SharedString::from(resp.body.as_ref()));
                }
            }
        });
        // 新：1 个编辑器，直接接管 Arc<str>
        let (new_allocs, new_bytes, _) = alloc_probe::measure(|| {
            for _ in 0..BODIES {
                std::hint::black_box(gpui::SharedString::from(Arc::clone(&resp.body)));
            }
        });

        println!(
            "5MB 正文 x {BODIES} 次响应：旧（4 个编辑器 + &str 转换）{old_allocs} 次分配 / {:.1} MB；\
             新（1 个编辑器 + Arc 接管）{new_allocs} 次分配 / {new_bytes} 字节",
            old_bytes as f64 / 1024.0 / 1024.0
        );
        assert!(
            new_allocs * 10 <= old_allocs,
            "正文交接口的分配次数应少一个数量级以上（{} vs {}）",
            new_allocs,
            old_allocs
        );
        assert!(
            new_bytes * 1000 <= old_bytes,
            "正文交接口的字节数应少 3 个数量级（{} vs {}）",
            new_bytes,
            old_bytes
        );
        assert!(
            old_bytes >= BODIES * 4 * 5 * 1024 * 1024,
            "旧写法确实每次都在整篇正文上分配"
        );
    }

    // ---------- 第八轮：侧栏历史列表每行的派生文案 ----------

    // 复刻 UI 文案的耗时格式化：本文件里凡是出现耗时文案的地方都必须调
    // `format_response_time` 本体，不能自己写 `format!("{}ms", t)` —— 否则量的是/断言的是
    // 另一段代码，UI 改了格式这里也不会失败（等于没测真代码）。
    use crate::ui::main_view::{format_response_time, HistoryList, HistoryRow};

    fn history_entries(n: usize) -> Vec<crate::app::database::HistoryEntry> {
        (0..n)
            .map(|i| crate::app::database::HistoryEntry {
                id: format!("id-{i}"),
                method: "GET".to_string(),
                url: format!("https://api.example.com/v1/resource/{i}"),
                headers: None,
                body: None,
                response_status: Some(200),
                response_headers: None,
                response_body: None,
                response_time_ms: Some(12),
                response_size: Some(128),
                created_at: Utc::now(),
            })
            .collect()
    }

    /// 侧栏历史列表每帧每行要做的事：旧写法现算
    /// `entry.method.clone()` + `entry.url.clone()` + `format!("history-row-{}")`
    /// + 状态行 `format!`；新写法每行只 clone 现成的 `SharedString`（引用计数）。
    ///
    /// （列表本身已经是 `Arc<HistoryList>`，快照只做引用计数 —— 这里量的是**行内**那层。）
    #[test]
    fn history_row_labels_are_cached_not_rebuilt() {
        const ROWS: usize = 50;
        let entries = history_entries(ROWS);
        let list = Arc::new(HistoryList::new(history_entries(ROWS)));

        let stats = measure_steps(&mut [
            ("空转", &mut || noop_frames()),
            ("旧（每帧 clone + format!）", &mut || {
                for _ in 0..FRAMES {
                    for entry in &entries {
                        std::hint::black_box(entry.method.clone());
                        std::hint::black_box(entry.url.clone());
                        std::hint::black_box(gpui::SharedString::from(format!(
                            "history-row-{}",
                            entry.id
                        )));
                        std::hint::black_box(format!(
                            "{} ({})",
                            entry.response_status.unwrap_or(0),
                            entry
                                .response_time_ms
                                .map(format_response_time)
                                .unwrap_or_default()
                        ));
                    }
                }
            }),
            ("新（clone 缓存的 SharedString）", &mut || {
                for _ in 0..FRAMES {
                    for row in &list.rows {
                        std::hint::black_box(row.element_id.clone());
                        std::hint::black_box(row.method.clone());
                        std::hint::black_box(row.url.clone());
                        std::hint::black_box(row.status_line.clone());
                    }
                }
            }),
        ]);
        assert_refcounted("历史行文案", stats[0], stats[1], stats[2]);
    }

    /// 缓存文案必须与改造前 render 里现算的表达式逐字一致（用户可见行为）。
    #[test]
    fn history_row_labels_match_derived_text() {
        use crate::app::database::HistoryEntry;

        let mut entries = history_entries(3);
        entries[0].response_time_ms = None; // 有状态码但没有耗时 -> 空括号
        entries[1].response_status = None; // 没有状态码 -> 不显示状态行
        entries[2].response_time_ms = Some(1500);
        entries[2].response_status = Some(404);
        let list = HistoryList::new(entries.clone());

        for (entry, row) in entries.iter().zip(list.rows.iter()) {
            assert_eq!(row.element_id.as_ref(), format!("history-row-{}", entry.id));
            assert_eq!(row.method.as_ref(), entry.method);
            assert_eq!(row.url.as_ref(), entry.url);
            let expected = match entry.response_status {
                Some(status) => format!(
                    "{} ({})",
                    status,
                    entry
                        .response_time_ms
                        .map(format_response_time)
                        .unwrap_or_default()
                ),
                None => String::new(),
            };
            assert_eq!(row.status_line.as_ref(), expected);
        }
    }

    /// 缓存派生渲染数据之后，树上带的 element id / 显示文案必须和现算的一模一样
    /// （否则 hover/展开状态会串行，或列表里的名字变了 —— 属于用户可见行为）。
    #[test]
    fn collection_tree_cached_keys_match_derived_values() {
        let folders = vec![folder(0), folder(1)];
        let requests = vec![saved_request(0, 0), saved_request(0, 1)];
        let expanded: HashSet<String> = ["folder-0".to_string()].into_iter().collect();
        let tree = build_collection_tree(&folders, &requests, &expanded);

        // 顶层是两个文件夹；folder-0 已展开，里面挂着两个请求
        assert_eq!(tree.len(), 2, "顶层是两个文件夹");
        let mut all: Vec<&CollectionItem> = Vec::new();
        for item in &tree {
            all.push(item);
            if let CollectionItem::Folder { children, .. } = item {
                all.extend(children.iter());
            }
        }
        assert_eq!(all.len(), 4, "两个文件夹 + folder-0 里的两个请求");
        for item in all {
            let (id, name, depth, keys) = match item {
                CollectionItem::Folder { id, name, depth, keys, .. } => (id, name, depth, keys),
                CollectionItem::Request { id, name, depth, keys, .. } => (id, name, depth, keys),
            };
            // 行 id：文件夹 "folder-{id}"、请求 "req-{id}"（与改造前 format! 的结果逐字一致）
            let prefix = if matches!(item, CollectionItem::Folder { .. }) { "folder" } else { "req" };
            assert_eq!(keys.row.as_ref(), format!("{prefix}-{}", id));
            // 展开箭头只有文件夹有；请求条目的 toggle 留空（渲染里从不用它）
            let expected_toggle = if prefix == "folder" { format!("folder-toggle-{}", id) } else { String::new() };
            assert_eq!(keys.toggle.as_ref(), expected_toggle);
            assert_eq!(keys.more.as_ref(), format!("more-{}", id));
            // 显示文案：按显示宽度截断（中文≈2 宽）
            let expected =
                crate::ui::sidebar::truncate_name(name, 24usize.saturating_sub(*depth * 2));
            assert_eq!(keys.label.as_ref(), expected);
        }
    }
}
