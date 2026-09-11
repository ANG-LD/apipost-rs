//! 响应体高亮：把「解析 + 美化 + 分词」的结果算一次缓存起来，渲染时只做廉价搬运。
//!
//! 背景：gpui 每帧都会重新调用渲染函数，而原来的流程把
//! `serde_json::from_str` + `to_string_pretty` + 逐字符分词 + 逐字符建 div
//! 全放在渲染里，一段几百 KB 的 JSON 每帧要重新解析一遍、并产生几十万次分配。
//!
//! 现在：
//! * `build()` 只在响应到达或主题变化时调用一次；
//! * 结果是按行组织的「着色片段」，文本用 `SharedString`（本质是 Arc），
//!   渲染时 clone 一次只是引用计数 +1，不再复制字符串；
//! * 相邻同色片段会合并，片段数从「每个字符一个」降到「每行几个」。

use crate::ui::themes::Theme;
use gpui::{Rgba, SharedString};
use std::ops::Range;
use std::sync::Arc;

/// 行内的一个着色片段。
///
/// `text` 用 `SharedString`：渲染时 `clone()` 是 O(1) 的引用计数操作，
/// 不会像 `String` 那样每帧复制一遍正文。
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub text: SharedString,
    pub color: Rgba,
}

/// 一行由若干着色片段拼成
pub type Line = Vec<Run>;

/// 整块响应体的渲染计划：一段拼好的正文 + 一张 run 长度表。
///
/// 渲染每帧只需要按它构造 `Vec<TextRun>`：正文用 `SharedString` 共享（引用计数），
/// `runs[i] = (字节长度, 颜色)` 与正文按顺序一一对应，长度之和恰好等于正文长度。
/// 行与行之间的换行由正文自己的 `'\n'` 承载，因此整块响应体只需要**一个**元素，
/// 不再像旧实现那样「每行一个 div + 每个片段一个 div」。
#[derive(Clone, Debug, Default)]
pub struct StyledBody {
    pub text: SharedString,
    pub runs: Vec<(u32, Rgba)>,
}

/// 一段响应体的高亮结果
#[derive(Clone, Debug, Default)]
pub struct HighlightedBody {
    /// 逐行片段；JSON 走这里（渲染不再用它建元素，保留给测试与按行统计）
    pub lines: Vec<Line>,
    /// 非 JSON 原文：直接持有响应体的 `Arc<str>`，渲染时零拷贝
    pub plain: Option<Arc<str>>,
    /// 整块渲染计划；JSON 走这里，`plain` 为 `Some` 时是空的
    pub styled: StyledBody,
}

impl HighlightedBody {
    /// 行数（给测试和后续按可视区域渲染用）
    pub fn line_count(&self) -> usize {
        if self.plain.is_some() {
            1
        } else {
            self.lines.len()
        }
    }
}

/// 响应体渲染缓存。
///
/// 判定是否要重算只看两件事：响应体还是不是同一个 `Arc`（指针比较，
/// 响应对象整体替换，所以指针相同就意味着内容、Content-Type 都没变），
/// 以及主题有没有变（主题变了颜色就变了）。
pub struct Cache {
    body: Arc<str>,
    theme: Theme,
    /// 高亮结果放在堆上共享，需要时 clone 这个 Arc 就行
    pub highlighted: Arc<HighlightedBody>,
}

impl Cache {
    pub fn new(body: Arc<str>, theme: Theme, highlighted: HighlightedBody) -> Self {
        Self {
            body,
            theme,
            highlighted: Arc::new(highlighted),
        }
    }

    pub fn is_valid_for(&self, body: &Arc<str>, theme: &Theme) -> bool {
        Arc::ptr_eq(&self.body, body) && &self.theme == theme
    }
}

/// 分词用的中间结构：只记录字节区间和类型，不分配字符串
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// 字符串字面量（还要再判断是 key 还是 value）
    Str,
    Number,
    Boolean,
    Null,
    Bracket,
    Comma,
    Colon,
    /// 空白和无法识别的字符
    Plain,
}

/// 判断响应体是否按 JSON 高亮（与之前 render 里的判断保持一致）
pub fn is_json_like(content_type: Option<&str>, body: &str) -> bool {
    let ct = content_type.unwrap_or("");
    ct.contains("json")
        || body.trim_start().starts_with('{')
        || body.trim_start().starts_with('[')
}

/// 构造高亮结果。同样的输入 + 同样的主题，结果完全一样，可以放心缓存。
pub fn build(body: &str, content_type: Option<&str>, theme: &Theme) -> HighlightedBody {
    if !is_json_like(content_type, body) {
        // 非 JSON：不需要美化也不分词，直接把 Arc 交给渲染层
        return HighlightedBody {
            lines: Vec::new(),
            plain: Some(Arc::from(body)),
            styled: StyledBody::default(),
        };
    }

    // 只有在确实需要美化时才 parse；解析失败就按原文高亮
    let formatted = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| body.to_string()),
        Err(_) => body.to_string(),
    };

    let colored = colorize(&formatted, theme);
    let lines = split_lines(&formatted, &colored);
    HighlightedBody {
        styled: compress(&lines, theme),
        lines,
        plain: None,
    }
}

/// 把「逐行片段」压成「整块正文 + run 长度表」。
///
/// 换行符并入它前面那个 run：`TextRun` 只是一段字节长度，正文里的 `'\n'` 必须落在
/// 某个 run 里，而换行本身不可见，挂在哪个颜色上都一样。这样 run 长度之和
/// 恰好等于正文长度（`StyledText::with_runs` 会严格校验这件事，对不上会 panic）。
fn compress(lines: &[Line], theme: &Theme) -> StyledBody {
    // 先按「正文 + 行间换行」算准容量，避免反复扩容
    let total: usize = lines
        .iter()
        .flat_map(|line| line.iter())
        .map(|run| run.text.len())
        .sum::<usize>()
        + lines.len().saturating_sub(1);
    let mut text = String::with_capacity(total);
    let mut runs: Vec<(u32, Rgba)> = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            text.push('\n');
            match runs.last_mut() {
                // 换行并进上一个 run
                Some(last) => last.0 += 1,
                // 空行开头：正文以换行起头，得先有个 run 承载它
                None => runs.push((1, theme.json_bracket)),
            }
        }
        for run in line {
            // 单个片段超过 4GB 才会溢出；真到了那个量级宁可在这里直接报错，
            // 也不要静默截断长度表（那会让 run 长度之和和正文对不上）
            let len = u32::try_from(run.text.len()).expect("单个着色片段超过 4GB");
            text.push_str(&run.text);
            // 相邻同色直接并进上一个 run，run 数与行内片段数一致
            match runs.last_mut() {
                Some(last) if last.1 == run.color => last.0 += len,
                _ => runs.push((len, run.color)),
            }
        }
    }

    StyledBody {
        text: SharedString::from(text),
        runs,
    }
}

/// 单遍扫描分词：记录区间，不复制文本。
///
/// 和旧实现相比去掉了两个热点：
/// * `json_str.chars().skip(pos).take(n).collect()` —— 每个 `n/t/f` 都要从头
///   重新扫一遍字符串（整体接近 O(n²)）；
/// * `c.to_string()` —— 每个括号、逗号、冒号、空白都单独分配一次。
fn tokenize(json: &str) -> Vec<(Kind, Range<usize>)> {
    let bytes = json.as_bytes();
    let mut tokens: Vec<(Kind, Range<usize>)> = Vec::new();
    let mut i = 0usize;

    // 相邻同类型直接延长上一个区间，避免一个空格一个 token
    let push = |tokens: &mut Vec<(Kind, Range<usize>)>, kind: Kind, start: usize, end: usize| {
        if let Some(last) = tokens.last_mut() {
            // 注意：字符串/数字等语义 token 不合并，避免影响 key 判定
            if last.0 == kind && matches!(kind, Kind::Plain) && last.1.end == start {
                last.1.end = end;
                return;
            }
        }
        tokens.push((kind, start..end));
    };

    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'"' => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    match bytes[i] {
                        b'\\' => i += 2, // 跳过被转义的字符
                        b'"' => {
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
                let end = i.min(bytes.len());
                tokens.push((Kind::Str, start..end));
            }
            b'{' | b'}' | b'[' | b']' => {
                tokens.push((Kind::Bracket, i..i + 1));
                i += 1;
            }
            b':' => {
                tokens.push((Kind::Colon, i..i + 1));
                i += 1;
            }
            b',' => {
                tokens.push((Kind::Comma, i..i + 1));
                i += 1;
            }
            b'n' if json[i..].starts_with("null") => {
                tokens.push((Kind::Null, i..i + 4));
                i += 4;
            }
            b't' if json[i..].starts_with("true") => {
                tokens.push((Kind::Boolean, i..i + 4));
                i += 4;
            }
            b'f' if json[i..].starts_with("false") => {
                tokens.push((Kind::Boolean, i..i + 5));
                i += 5;
            }
            b'-' | b'0'..=b'9' => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    match bytes[i] {
                        b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-' => i += 1,
                        _ => break,
                    }
                }
                tokens.push((Kind::Number, start..i));
            }
            b' ' | b'\t' | b'\n' | b'\r' => {
                let start = i;
                while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                    i += 1;
                }
                push(&mut tokens, Kind::Plain, start, i);
            }
            _ => {
                // UTF-8 多字节字符按整字符前进，保证区间永远落在字符边界上
                let ch = json[i..].chars().next().unwrap_or(' ');
                let start = i;
                i += ch.len_utf8();
                push(&mut tokens, Kind::Plain, start, i);
            }
        }
    }

    tokens
}

/// 给分词结果上色。
///
/// key 判定用「容器栈 + 是否在等 key」的状态机，而不是旧的
/// 「上一个 token 是括号或逗号」这种启发式 —— 旧规则遇到美化后的缩进
/// 就会失效（缩进是空白 token，会把标记清掉），导致 key 一律被当成普通字符串，
/// `json_key` 颜色从来没生效过。`true` / `false` / `null` 同理。
fn colorize(json: &str, theme: &Theme) -> Vec<(Rgba, Range<usize>)> {
    let mut colored: Vec<(Rgba, Range<usize>)> = Vec::new();
    // 容器栈：true 表示当前在对象里（等 key），false 表示在数组里（等值）
    let mut containers: Vec<bool> = Vec::new();
    let mut expecting_key = false;

    for (kind, range) in tokenize(json) {
        let color = match kind {
            Kind::Str => {
                let color = if expecting_key {
                    theme.json_key
                } else {
                    theme.json_string
                };
                expecting_key = false;
                color
            }
            Kind::Number => {
                expecting_key = false;
                theme.json_number
            }
            Kind::Boolean => {
                expecting_key = false;
                theme.json_boolean
            }
            Kind::Null => {
                expecting_key = false;
                theme.json_null
            }
            Kind::Bracket => {
                match json.as_bytes().get(range.start).copied() {
                    Some(b'{') => {
                        containers.push(true);
                        expecting_key = true;
                    }
                    Some(b'[') => {
                        containers.push(false);
                        expecting_key = false;
                    }
                    _ => {
                        containers.pop();
                        expecting_key = false;
                    }
                }
                theme.json_bracket
            }
            Kind::Comma => {
                // 对象里逗号后面是 key，数组里逗号后面是值
                expecting_key = *containers.last().unwrap_or(&false);
                theme.json_bracket
            }
            // 冒号和空白不影响「是否在等 key」，这一条正是旧实现漏掉的
            Kind::Colon | Kind::Plain => theme.json_bracket,
        };

        // 相邻同色合并：行内片段数从「每字符一个」降到「每行几个」
        if let Some(last) = colored.last_mut() {
            if last.0 == color && last.1.end == range.start {
                last.1.end = range.end;
                continue;
            }
        }
        colored.push((color, range));
    }

    colored
}

/// 按换行切分，生成逐行片段。
/// 每个片段只在这里分配一次 `SharedString`，之后每帧复用。
fn split_lines(json: &str, colored: &[(Rgba, Range<usize>)]) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    let mut current: Line = Vec::new();

    for (color, range) in colored {
        let text = &json[range.clone()];
        let mut rest = text;
        loop {
            match rest.find('\n') {
                Some(pos) => {
                    if pos > 0 {
                        current.push(Run {
                            text: SharedString::from(&rest[..pos]),
                            color: *color,
                        });
                    }
                    lines.push(std::mem::take(&mut current));
                    rest = &rest[pos + 1..];
                }
                None => {
                    if !rest.is_empty() {
                        current.push(Run {
                            text: SharedString::from(rest),
                            color: *color,
                        });
                    }
                    break;
                }
            }
        }
    }
    lines.push(current);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> Theme {
        Theme::from_str("dark")
    }

    fn sample() -> &'static str {
        r#"{"name":"apipost","version":2,"nested":{"ok":true,"list":[1,2.5,-3e2],"none":null},"中文":"值"}"#
    }

    /// 摊平成「每行文本 + 颜色」，方便断言。行间换行符的颜色按空白处理（bracket 色）。
    fn flatten(hl: &HighlightedBody, theme: &Theme) -> Vec<(String, Vec<(char, Rgba)>)> {
        let mut lines: Vec<(String, Vec<(char, Rgba)>)> = Vec::new();
        for (index, line) in hl.lines.iter().enumerate() {
            let mut text = String::new();
            let mut chars: Vec<(char, Rgba)> = Vec::new();
            if index > 0 {
                text.push('\n');
                chars.push(('\n', theme.json_bracket));
            }
            for run in line {
                text.push_str(&run.text);
                for ch in run.text.chars() {
                    chars.push((ch, run.color));
                }
            }
            lines.push((text, chars));
        }
        lines
    }

    fn colors_of(hl: &HighlightedBody, theme: &Theme) -> Vec<(char, Rgba)> {
        flatten(hl, theme)
            .into_iter()
            .flat_map(|(_, chars)| chars)
            .collect()
    }

    fn joined_text(hl: &HighlightedBody, theme: &Theme) -> String {
        flatten(hl, theme)
            .into_iter()
            .map(|(text, _)| text)
            .collect()
    }

    /// 切分不能丢字、错序：拼回来必须正好是美化后的 JSON。
    #[test]
    fn text_round_trips_to_pretty_json() {
        let theme = theme();
        for body in [
            sample(),
            r#"[1,2,3]"#,
            r#"{"a":"b"}"#,
            r#"{"空":[],"num":1.5e-3,"esc":"a\"b"}"#,
            r#"{"deep":{"deeper":{"x":[true,false,null]}}}"#,
            r#"[{"a":1},{"b":[2,3]}]"#,
        ] {
            let expected = serde_json::to_string_pretty(
                &serde_json::from_str::<serde_json::Value>(body).expect("样例必须是合法 JSON"),
            )
            .expect("序列化不应失败");
            let hl = build(body, None, &theme);
            assert_eq!(joined_text(&hl, &theme), expected, "文本不一致: {body}");
        }
    }

    /// 每类 token 的颜色必须是主题里对应的字段。
    ///
    /// 旧实现在这两点上是有 bug 的，顺手修掉（渲染逻辑重写后必须显式钉住）：
    /// 1. `true` / `false` / `null` 被拆成单个字母，落到空白色，从没用到
    ///    `json_boolean` / `json_null`；
    /// 2. 美化后的 JSON 里 key 前面有缩进，旧实现把 key 判成了普通字符串。
    #[test]
    fn token_colors_follow_theme() {
        let theme = theme();
        let hl = build(sample(), None, &theme);
        let chars = colors_of(&hl, &theme);

        let color_at = |needle: &str| -> Rgba {
            let offset = joined_text(&hl, &theme)
                .find(needle)
                .unwrap_or_else(|| panic!("样例里找不到 {needle}"));
            chars[offset].1
        };

        assert_eq!(color_at("\"name\""), theme.json_key, "key 应该用 key 色");
        assert_eq!(color_at("\"apipost\""), theme.json_string, "值应该用 string 色");
        assert_eq!(color_at("2"), theme.json_number, "数字应该用 number 色");
        assert_eq!(color_at("true"), theme.json_boolean, "布尔应该用 boolean 色");
        assert_eq!(color_at("null"), theme.json_null, "null 应该用 null 色");
        assert_eq!(color_at(":"), theme.json_bracket, "标点应该用 bracket 色");
    }

    /// 行数必须和换行一致，否则渲染会错行
    #[test]
    fn split_lines_matches_newline_count() {
        let theme = theme();
        for body in [sample(), r#"[1,2,3]"#, r#"{"a":{"b":1}}"#] {
            let hl = build(body, None, &theme);
            let pretty = serde_json::to_string_pretty(
                &serde_json::from_str::<serde_json::Value>(body).expect("样例必须是合法 JSON"),
            )
            .expect("序列化不应失败");
            assert_eq!(hl.line_count(), pretty.matches('\n').count() + 1, "{body}");
        }
    }

    /// 非 JSON 走 plain 分支：直接持有 Arc，不复制正文
    #[test]
    fn plain_body_shares_arc() {
        let hl = build("hello world", None, &theme());
        let plain = hl.plain.expect("非 JSON 应该有 plain");
        assert_eq!(&*plain, "hello world");
        assert!(hl.lines.is_empty());
    }

    /// 中文、emoji 等多字节字符不能被按字节切断
    #[test]
    fn handles_multibyte() {
        let hl = build(r#"{"名字":"张三","emoji":"🚀"}"#, None, &theme());
        let text = joined_text(&hl, &theme());
        assert!(text.contains("张三"));
        assert!(text.contains("🚀"));
    }

    /// 片段数应该是「每行几个」，而不是「每个字符一个」
    #[test]
    fn runs_are_merged_per_line() {
        let theme = theme();
        let body = serde_json::to_string(&serde_json::json!({
            "items": (0..50).map(|i| serde_json::json!({"id": i, "ok": true})).collect::<Vec<_>>()
        }))
        .unwrap();
        let hl = build(&body, None, &theme);
        let runs: usize = hl.lines.iter().map(|line| line.len()).sum();
        let chars: usize = hl
            .lines
            .iter()
            .flat_map(|line| line.iter())
            .map(|run| run.text.chars().count())
            .sum();
        // 旧实现是「每个字符一个 div + 一次 to_string()」，
        // 现在是「每行几个片段、且不再分配字符串」。
        assert!(runs * 3 < chars, "合并效果不够：{runs} 个片段 / {chars} 个字符");
    }

    /// 量化「每帧」的分配开销：旧实现 vs 新实现。
    ///
    /// 旧渲染函数每帧都要：解析整段 JSON -> 美化 -> 逐字符分词 -> 每个字符建一个
    /// div 并 `to_string()`。这里用同一份响应体对比两侧的分配次数。
    #[test]
    #[ignore = "量化报告，手动跑：--ignored --nocapture --test-threads=1"]
    fn per_frame_report() {
        use super::alloc_probe::measure;

        let theme = theme();
        // 造一份约 200KB 的响应体（1000 个对象）
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
        println!("\n响应体大小: {} 字节", body.len());

        // ---- 旧路径：每帧重新做一遍 ----
        let (old_allocs, old_bytes, _) = measure(|| {
            let pretty = serde_json::to_string_pretty(
                &serde_json::from_str::<serde_json::Value>(&body).unwrap(),
            )
            .unwrap();
            // 旧实现：逐字符分词 + 逐字符 to_string 建元素
            let mut elements = 0usize;
            for ch in pretty.chars() {
                let _owned: String = ch.to_string();
                elements += 1;
            }
            elements
        });

        // ---- 新路径：缓存建立一次 ----
        let (build_allocs, build_bytes, hl) = measure(|| build(&body, None, &theme));
        let runs: usize = hl.lines.iter().map(|line| line.len()).sum();
        println!("高亮结果: {} 行, {} 个片段（旧实现是每个字符一个元素）", hl.lines.len(), runs);

        // ---- 新路径：每帧渲染（clone 片段是引用计数操作） ----
        let (frame_allocs, frame_bytes, _) = measure(|| {
            let mut elements = 0usize;
            for line in &hl.lines {
                for run in line {
                    let text: SharedString = run.text.clone();
                    let _color = run.color;
                    std::hint::black_box(text);
                    elements += 1;
                }
            }
            elements
        });

        println!("\n每帧分配次数:   旧 {old_allocs:>8}  ->  新 {frame_allocs:>8}");
        println!("每帧分配字节数: 旧 {old_bytes:>8}  ->  新 {frame_bytes:>8}");
        println!("一次性建缓存:   {build_allocs} 次分配 / {build_bytes} 字节（只在响应到达或换主题时付一次）");
        println!(
            "每帧分配次数下降: {:.1} 倍\n",
            old_allocs as f64 / frame_allocs.max(1) as f64
        );

        assert!(frame_allocs < old_allocs / 10, "每帧分配应该至少降一个数量级");
    }

    /// 量化另外两处「每帧」开销：翻译查表、大集合的状态快照。
    #[test]
    #[ignore = "量化报告，手动跑：--ignored --nocapture --test-threads=1"]
    fn per_frame_report_state() {
        use super::alloc_probe::measure;
        use std::collections::HashMap;
        use std::sync::Arc;

        // ---- 翻译：旧实现每次 clone 一个 String，新实现是 Arc 计数 ----
        let map: HashMap<String, Arc<str>> = (0..600)
            .map(|i| (format!("k{i}"), Arc::<str>::from("一段翻译文案")))
            .collect();
        let old_map: HashMap<String, String> = map
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect();
        let keys: Vec<String> = (0..119).map(|i| format!("k{}", i % 600)).collect();

        let (old_allocs, _, _) = measure(|| {
            for key in &keys {
                let text: String = old_map.get(key).cloned().unwrap_or_default();
                std::hint::black_box(text);
            }
        });
        let (new_allocs, _, _) = measure(|| {
            for key in &keys {
                let text: Arc<str> = Arc::clone(map.get(key).unwrap());
                std::hint::black_box(text);
            }
        });
        println!("\n每帧 119 次翻译查表分配次数: 旧 {old_allocs} -> 新 {new_allocs}");

        // ---- 状态快照：旧实现每帧深拷贝整个 Vec，新实现是 Arc 计数 ----
        #[derive(Clone)]
        #[allow(dead_code)]
        struct Entry {
            id: String,
            url: String,
            method: String,
            response_body: Option<String>,
        }
        let entries: Vec<Entry> = (0..400)
            .map(|i| Entry {
                id: format!("id-{i}"),
                url: format!("https://example.com/api/{i}"),
                method: "GET".into(),
                response_body: Some(format!("{{\"i\":{i}}}")),
            })
            .collect();
        let shared = Arc::new(entries.clone());

        let (old_allocs, old_bytes, _) = measure(|| {
            let snapshot = entries.clone();
            std::hint::black_box(snapshot.len());
        });
        let (new_allocs, new_bytes, _) = measure(|| {
            let snapshot = Arc::clone(&shared);
            std::hint::black_box(snapshot.len());
        });
        println!(
            "每帧 400 条历史/收藏快照: 旧 {old_allocs} 次分配 / {old_bytes} 字节 -> 新 {new_allocs} 次 / {new_bytes} 字节"
        );

        assert_eq!(new_allocs, 0);
        assert_eq!(new_bytes, 0);
        assert!(old_allocs > 1000);
    }

    /// 缓存判定：同一份响应体 + 同一主题命中缓存，换主题或换响应体就必须失效
    #[test]
    fn cache_invalidation() {
        let theme = theme();
        let body: Arc<str> = Arc::from(sample());
        let cache = Cache::new(Arc::clone(&body), theme.clone(), build(&body, None, &theme));

        assert!(cache.is_valid_for(&body, &theme), "同样的输入应该命中缓存");
        assert!(
            !cache.is_valid_for(&Arc::from(sample()), &theme),
            "新的响应体（新 Arc）必须失效"
        );
        assert!(
            !cache.is_valid_for(&body, &Theme::from_str("light")),
            "换主题必须失效"
        );
    }
}

/// 只在测试里挂的计数分配器：用来量化「每帧到底分配了多少次」。
/// 用 `cargo test --offline per_frame_report -- --ignored --nocapture --test-threads=1` 运行。
#[cfg(test)]
pub(crate) mod alloc_probe {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    pub static ALLOCS: AtomicUsize = AtomicUsize::new(0);
    pub static BYTES: AtomicUsize = AtomicUsize::new(0);
    pub static ON: AtomicBool = AtomicBool::new(false);

    pub struct Counting;

    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            if ON.load(Ordering::Relaxed) {
                ALLOCS.fetch_add(1, Ordering::Relaxed);
                BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            }
            System.alloc(layout)
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout)
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            if ON.load(Ordering::Relaxed) {
                ALLOCS.fetch_add(1, Ordering::Relaxed);
                BYTES.fetch_add(new_size, Ordering::Relaxed);
            }
            System.realloc(ptr, layout, new_size)
        }
    }

    #[global_allocator]
    pub static GLOBAL: Counting = Counting;

    /// 统计一段代码的 (分配次数, 分配字节数)
    ///
    /// 全局计数器是进程级的，多个测试并行测量会互相污染，所以这里加锁串行化。
    /// 注意：其它线程（非测量中的测试）的分配仍会计入，因此断言请用
    /// 「数量级对比」而不是「严格等于 0」。
    pub fn measure<R>(f: impl FnOnce() -> R) -> (usize, usize, R) {
        static MEASURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = MEASURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        ALLOCS.store(0, Ordering::SeqCst);
        BYTES.store(0, Ordering::SeqCst);
        ON.store(true, Ordering::SeqCst);
        let out = f();
        ON.store(false, Ordering::SeqCst);
        (
            ALLOCS.load(Ordering::SeqCst),
            BYTES.load(Ordering::SeqCst),
            out,
        )
    }
}
