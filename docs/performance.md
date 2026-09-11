# 性能优化说明

gpui 的界面函数**每帧都会被重新调用**，所以凡是在 `render()` 里做的重活（解析、格式化、
逐字符建元素、深拷贝状态）都会以每秒几十次的频率重复执行。这份文档记录已经做过的优化，
以及维护时必须守住的几条不变量。

## 一、已经消除的每帧开销

| 位置 | 原实现（每帧） | 现实现 |
| --- | --- | --- |
| 响应体渲染 `render_preview_body` | `serde_json::from_str` + `to_string_pretty` + 逐字符分词 + **每个字符一个 div 并 `to_string()`** | 结果缓存在 `MainView::response_highlight`，每帧只做 `SharedString` 的引用计数 clone |
| 翻译查表 `MainView::t()` | 每次 `HashMap<String, String>` 查询后 clone 一个 `String` | 字典值是 `Arc<str>`，查询后只做计数 +1 |
| 主题 `Theme` | 各面板每帧 `app_state.lock()` + `Theme::from_str(&theme_name)` | 统一用 `MainView::cached_theme` |
| params / headers / form-data 表格 | 每帧 `this.params.clone()`、`entries.to_vec()` | 直接借用 `this.params` / 切片 |
| `history` / `saved_requests` / `folders` / `collection_items` / `environments` | 每帧深拷贝整个 `Vec`（含其中所有 `String`） | 字段是 `Arc<Vec<T>>`，每帧只 clone 一次 `Arc` |
| `render()` 里的 6 个局部快照 | `method` / `params` / `headers` / `body_state` / `auth_state` / `settings` 拷了但**从未使用** | 直接删除 |

量化（900 KB 级响应体，`cargo test --offline per_frame_report -- --ignored --nocapture --test-threads=1`）：

```
响应体 89550 字节 -> 11004 行 / 34006 个片段
每帧分配次数:   旧 187582  ->  新 0
每帧分配字节数: 旧 2356148 ->  新 0
一次性建缓存:   29071 次分配（只在响应到达 / 换主题时付一次）
每帧 119 次翻译查表:      旧 119 次分配 -> 新 0
每帧 400 条历史/收藏快照: 旧 1601 次分配 / 56070 字节 -> 新 0
```

## 二、响应体高亮：缓存契约

实现在 `src/ui/response_highlight.rs`。

* `build()` 负责「解析 → 美化 → 分词 → 按行切片」，产出 `HighlightedBody`：
  * JSON：`lines: Vec<Vec<Run>>`，`Run { text: SharedString, color: Rgba }`；
  * 非 JSON：`plain: Option<Arc<str>>`，直接持有响应体的 `Arc`，渲染时零拷贝。
* 分词是**单遍、按字节区间**做的，只有最后切行时才为每个片段分配一次 `SharedString`。
  相邻同色片段会合并，片段数从「每字符一个」降到「每行几个」。
* 缓存判定（`Cache::is_valid_for`）只看两件事：
  1. `Arc::ptr_eq(&cache.body, &resp.body)` —— 响应对象整体替换，指针相同即内容与
     Content-Type 都相同；
  2. 主题是否相等（主题变了颜色就变了）。
* `MainView::ensure_response_highlight()` 在 `render()` 开头调用：这里是唯一同时满足
  「能拿到 `&mut self`」和「每帧都会执行」两个条件的地方。缓存缺失时
  `render_preview_body` 会现算一次（`build` 是纯函数），所以渲染结果永远正确，
  最坏情况只是慢一帧。

**维护注意**：改动响应体渲染时，不要退回「每帧 parse/逐字符建元素」的写法；
新增的颜色规则请同步更新 `response_highlight.rs` 的 `token_colors_follow_theme` 测试。

## 三、顺带修掉的三个显示 bug

重写渲染逻辑后，下面三处旧行为被显式钉进了测试：

1. `true` / `false` / `null` 在旧分词器里被拆成单个字母（走空白分支），
   所以 `json_boolean` / `json_null` 两个主题色从来没生效过 —— 现在生效了。
2. 旧规则「上一个 token 是括号或逗号就是 key」遇到美化后的缩进会失效
   （缩进是空白 token，会把标记清掉），导致 key 一律用普通字符串色；
   现在用「容器栈 + 是否在等 key」的状态机，数组里的字符串不会被误判成 key。
3. `src/ui/body.rs` 里 227 行旧高亮实现（`highlight_json` / `JsonHighlightToken`）
   已删除，避免两套实现并存。

## 四、仍然存在的开销（后续可做）

* 响应体每帧仍会构建约 3.4 万个 `div`（旧实现约 20 万个）。下一步可以：
  * 用 `gpui::StyledText` + `TextRun` 把**一行做成一个元素**，元素数再降约 3 倍；
  * 或者只渲染滚动区域内的可见行（虚拟滚动），这是收益最大的一步。
* 侧边栏历史/集合列表同样是全量渲染，可用同一思路做虚拟滚动。
* `MainView` 仍在每帧 clone `request_tabs`（标签数量少，暂未处理）。

## 五、第二轮：历史列表每帧快照（所有权/克隆）

### 问题

`MainView::render` 每帧都要为侧边栏历史列表做快照，而历史行的监听器是 `'static` 闭包，
旧实现为了把数据交给它，逐行复制整个对象：

```rust
// ❌ 每行每帧：整个 entry（含响应体）+ 响应头 + 响应体，各复制一份
let entry_clone = entry.clone();
let entry_response_body = entry.response_body.clone();
let entry_response_headers = entry.response_headers.clone();
```

`HistoryEntry` 的 `body` / `response_headers` / `response_body` 都是裸 `String`
（HTTP 响应体动辄几百 KB ~ 几 MB），**30 条历史就是每帧几十 MB 的纯 memcpy**，
而这些数据只在「点击该行」时才真正需要。

### 改法：共享所有权，而不是复制数据

`history` 本来就是 `Arc<Vec<HistoryEntry>>`，所以按「Arc + 下标」把整份列表交给监听器，
每行只做一次引用计数：

```rust
.children(history.iter().enumerate().map(|(history_idx, entry)| {
    // 每行只做一次 Arc 引用计数（原子自增），不复制 payload
    let history_row = self.history.clone();
    ...
    .on_mouse_down(MouseButton::Left, cx.listener(move |this, ..| {
        // 点击时才从 Arc 里取；要拥有的字段此时才 clone（按需付费）
        let Some(entry) = history_row.get(history_idx) else { return; };
        this.load_saved_request(&entry.id, &entry.method, &entry.url, "", ...);
    }))
```

要点：

* **大对象放堆上 + `Arc` 共享**：`HttpResponse.body` 上一轮已是 `Arc<str>`、
  `raw_body` 是 `Arc<[u8]>`，这次把同思路用到历史行。
* **生命周期换所有权**：gpui 元素与监听器必须是 `'static`，无法借用局部数据；
  正确解法是 `Arc`（廉价克隆 + `'static`），而不是 `clone()` 整份数据。
* **按需付费**：只在真正用到的分支（点击）里 clone，渲染路径上零复制。
* 用 `enumerate()` 的**下标**而不是存引用，避免把 `HistoryEntry` 本身也 `Arc` 化，
  改动面最小（`Arc<Vec<T>>` 已经是现成的）。

### 实测（`src/ui/perf_snapshot.rs` 的回归测试）

30 条历史、每条 1MB 响应体，测一帧快照的分配（`alloc_probe::measure`）：

| | 分配次数 | 复制字节 |
|---|---|---|
| 旧写法（逐行 clone） | 150 | **60.0 MB** |
| 新写法（Arc + 下标） | **0** | **0** |

即 60fps 下每秒省掉约 3.6 GB 的内存复制。测试会断言两种写法读取结果一致，
且新写法分配次数 ≤ 2 —— 以后谁再往渲染路径里加整对象 clone，测试会直接失败。

> 测试放在独立文件而不是 `main_view.rs`：后者内嵌的 gpui 元素类型嵌套极深，
> 在该文件里展开 `#[test]` 宏会把 rustc 的宏展开递归撑爆（提高 `recursion_limit`
> 甚至触发 SIGSEGV）。

### 已做（第二轮后续）：`response` 与 `request_tabs` 改 Arc

两处都是**渲染每帧快照**的成本，改成共享所有权后每帧只剩引用计数：

| 快照对象 | 旧写法 | 新写法 | 字段类型 |
|---|---|---|---|
| 响应（25 个响应头 + 1MB 体），60 帧 | 3064 次分配 / 1.17 MB | **0 / 0** | `Option<Arc<HttpResponse>>` |
| 10 个标签页，60 帧 | 4921 次分配 / 3.56 MB | **0 / 0** | `Arc<Vec<RequestTab>>` |

* `HttpResponse` 的 `headers`（`HashMap`）和 `cookies`（`Vec`）是裸的，clone 一次就是几十次堆分配；
  `body`/`raw_body` 本来已是 `Arc`，所以把整个 `HttpResponse` 放进 `Arc` 即可。
* 赋值侧配合改：得到响应时先 `let response = Arc::new(response);` 再 `this.response = Some(response.clone())`
  （后面还要读 `response` 的 body/headers，所以"先包再用"）。
* `request_tabs` 的写入路径用 `Arc::make_mut(&mut self.request_tabs)`：
  渲染期间没有写操作，引用计数为 1，实际不会触发拷贝。
* 注意 `Arc::make_mut(&mut self.request_tabs).push(RequestTab { name: self.t(...), ... })`
  **编译不过**：push 的实参借用 `self`，与 `&mut self.request_tabs` 冲突 —— 先构造好局部变量再 push。
* 测试见 `src/ui/perf_snapshot.rs`，断言"新写法 0 次分配"，属回归保护。

### 仍然可以继续做的

1. 每个标签页仍有 `tab.method.clone()` 之类的短字符串分配（数量级小，不是大对象）。
3. 每个标签页仍有 `tab.method.clone()` / `tab.name.clone()` 等短字符串分配
   （数量级小，非大对象）。
4. 响应体仍每帧构建约 3.4 万个 `div`；`StyledText` + `TextRun` 或虚拟滚动仍是最大的
   结构性收益（见第三节）。
