# UI用户界面布局

## 1. 标题栏（Header）
- **左侧**：菜单图标（☰）、工作空间名称（下拉切换）、同步状态图标
- **中间**：搜索框（全局搜索请求、集合、环境等）
- **右侧**：学习中心、设置（⚙️）、通知（🔔）、账户头像

## 2. 侧边栏（Sidebar）
三个主要选项卡：
- **Collections（集合）**：树状结构展示 API 请求分组，支持文件夹和子请求
- **APIs**：管理 OpenAPI/Swagger 定义的 API 规范
- **Environments（环境）**：快速切换环境变量（如 dev、prod）

底部折叠区域：
- History（历史记录）
- Trash（回收站）

## 3. 主工作区（Main Work Area）

### 3.1 请求标签栏（Request Tabs）
- 已打开的请求标签页（支持拖动、关闭、固定）
- 右侧 `+` 新建请求按钮
- Runner（运行器）入口

### 3.2 请求构造器（Request Builder）
- HTTP 方法下拉（GET、POST、PUT、DELETE 等）
- URL 输入框（自动补全、参数高亮）
- 参数配置标签页：
  - Params（查询参数）
  - Authorization（认证）
  - Headers（请求头）
  - Body（请求体：form-data、raw、JSON 等）
  - Pre-request Script（前置脚本）
  - Tests（测试脚本）
  - Settings（请求设置）
- 操作按钮：
  - **Send**（蓝色主按钮）
  - **Save** / **Save As**

### 3.3 响应查看器（Response Viewer）
- **Body**：格式化显示 JSON、XML、HTML、文本，支持美化、预览、原始视图
- **Cookies**：显示响应中的 Cookies
- **Headers**：响应头键值对
- **Test Results**：显示测试脚本断言通过/失败数量

## 4. 底部状态栏（Status Bar）
- 当前环境名称（如 "No Environment"）
- 授权类型（如 "Bearer Token"）
- 网络状态（在线/离线）
- **Console**（控制台）按钮
- 请求/响应耗时与大小（发送后显示）

## 5. 辅助面板（Auxiliary Panels）
- **Console（控制台）**：请求/响应日志、脚本输出、错误堆栈
- **Collection Runner**：批量运行集合中的请求
- **Mock Server 管理界面**
- **Monitor 管理界面**

## 6. 右键上下文菜单
在侧边栏、请求标签、响应 Body 区域右键可执行：
- 复制、重命名、删除
- 生成代码
- 添加示例
- 折叠/展开全部

## 布局特点总结
- **可折叠侧边栏** + **多标签页** + **垂直分割请求/响应**
- 深色/浅色主题可选
- 界面现代、紧凑，强调请求构造与结果对比效率