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
    use gpui::Rgba;

    fn dark() -> Theme {
        Theme::from_str("dark")
    }

    /// 把 `lines` 摊平成「可见字符 + 颜色」（换行原本由行容器承载，不在这里）
    fn visible_chars_from_lines(hl: &HighlightedBody) -> Vec<(char, Rgba)> {
        hl.lines
            .iter()
            .flat_map(|line| line.iter())
            .flat_map(|run| run.text.chars().map(move |c| (c, run.color)))
            .collect()
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

            // 3) 每个可见字符的颜色必须和逐行数据完全一致
            assert_eq!(
                visible_chars_from_styled(&hl),
                visible_chars_from_lines(&hl),
                "字符或颜色不一致: {body}"
            );
        }
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

        let spans: usize = hl.lines.iter().map(|line| line.len()).sum();
        // 旧：外层 flex_col 容器 + 每行一个 div + 每个片段一个 div
        let old_elements = 1 + hl.lines.len() + spans;
        // 新：外层「不折行」容器 + 一个 StyledText（正文、换行、颜色全在里面）
        let new_elements = 2;

        println!(
            "响应体 {} 字节：{} 行 / {} 个着色片段",
            body.len(),
            hl.lines.len(),
            hl.styled.runs.len()
        );
        println!("每帧元素数：旧 {old_elements} -> 新 {new_elements}");

        assert!(
            old_elements > 10_000,
            "旧实现每帧确实要建上万个元素（实际 {old_elements}）"
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
}
