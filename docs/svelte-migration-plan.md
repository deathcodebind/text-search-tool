# Svelte 迁移方案

## 1. 迁移背景

当前前端实现是一个基于原生 DOM 操作的静态页面应用，页面状态和业务逻辑高度耦合，错误恢复和组件复用较难维护。迁移到 Svelte 的目标是：

- 把 UI 拆成清晰的组件和页面
- 用可组合的状态管理替代全局 DOM 查询逻辑
- 用框架化路由代替手工切页
- 用可复用的持久化/缓存机制替代散落的 localStorage 代码

## 2. 当前前端现状

当前 `ui/` 目录结构是：

- `index.html`：单个页面模板
- `app.js`：所有 UI 逻辑、路由切换、请求、事件绑定全部集中在一个文件
- `styles.css`：整体样式
- `jxemall-district-options.js`：地区选择数据

当前业务页面可以拆成四个核心视图：

- 登录页（`login`）
- 拉取页（`pull`）
- 关键词预览页（`keyword`）
- 详情页（`detail`）

## 3. 迁移目标

- 组件化：把表单、按钮、卡片、列表、分页、状态提示拆成可复用组件
- 路由化：页面间导航用路由而不是 DOM class 切换
- 结构化数据流：用 `store` 管理 session、任务状态、记录列表、关键词配置
- 缓存与持久化：登录记忆、关键词规则、查询结果、拉取列表都要有明确缓存策略

## 4. 关键决策

### 4.1 组件库：`Skeleton`（推荐）

推荐使用 `Skeleton`：

- 原生 Svelte 组件库，适配 `Vite + Svelte` 生态
- 内置表单、按钮、卡片、表格、弹窗、分页、主题切换
- 适合桌面应用风格，组件样式可快速定制
- 对 Tauri 的静态打包友好，不依赖复杂运行时

备选项（如果需要更轻量或更原生）：

- `Shoelace`：框架无关的 Web Components，Svelte 中也能直接使用
- `Flowbite Svelte`：如果愿意使用 Tailwind 生态
- `Svelte Material UI`：如果偏向 Material 风格

### 4.2 路由逻辑：`svelte-spa-router` + hash/`history` 路由

当前页面结构不需要服务器端路由，推荐用轻量客户端路由：

- `svelte-spa-router`：成熟、简单、支持参数路由
- 用路径表示页面：
  - `/login`
  - `/pull`
  - `/keyword`
  - `/detail/:sourceId`
- 如果希望更简单，也可用一个 `page` store + `#hash` 路由，但建议 `svelte-spa-router` 可减少自定义切页逻辑错误

路由要满足：

- 页面直接打开地址可进入对应视图
- 详情页可通过 `/detail/:sourceId` 直接访问
- 有“返回”功能，可从详情页返回上一个页面
- 可保留 `history`，避免上一页丢失状态

### 4.3 缓存逻辑：`@tanstack/svelte-query` + 持久化 `localStorage`

缓存方案分两类：

1. 后端请求数据缓存
   - 推荐使用 `@tanstack/svelte-query`（Svelte 版 React Query）
   - 优点：自动缓存、自动重试、状态统一、手动失效
   - 适合 `pull_records`, `pull_progress`, `preview_keyword_groups`, `pull_record_detail`
   - 结合 Tauri invoke 直接做 data fetching

2. 本地持久化状态
   - 登录记忆（`loginBaseUrl`, `username`）
   - 关键词配置（`rootMinimumShouldMatch`, `groups`）
   - 页面设置（`pageSize`, 默认筛选、主题、是否启用关键词过滤）
   - 使用自定义 Svelte store 包装 `localStorage`：
     - `createPersistedStore(key, defaultValue)`
     - `store.subscribe` 时自动写入
   - 也可选 `svelte-persisted-store` 或 `localforage`，但本项目场景简单，`localStorage` 足够

3. 会话与临时状态
   - 当前任务号、当前拉取页、选中地区、登录状态等，用 `writable()` 存储
   - 对于记录列表，可以缓存最近一次查询结果，并在 `pull_start` 后手动刷新

## 5. 迁移建议步骤

### 5.1 初始化 Svelte 项目

- 在 `ui/` 目录初始化 `package.json`
- 安装依赖：
  - `svelte`, `@sveltejs/vite-plugin-svelte`, `vite`
  - `@skeletonlabs/skeleton`（或其它组件库）
  - `svelte-spa-router`
  - `@tanstack/svelte-query`

### 5.2 设计页面与组件目录

建议目录结构：

- `ui/src/App.svelte`
- `ui/src/main.ts`
- `ui/src/routes/Login.svelte`
- `ui/src/routes/Pull.svelte`
- `ui/src/routes/Keyword.svelte`
- `ui/src/routes/Detail.svelte`
- `ui/src/components/RegionPicker.svelte`
- `ui/src/components/RecordTable.svelte`
- `ui/src/components/KeywordGroupEditor.svelte`
- `ui/src/stores/session.ts`
- `ui/src/stores/pull.ts`
- `ui/src/stores/keyword.ts`

### 5.3 迁移逻辑优先级

1. 先搭建 `Login -> Pull -> Keyword -> Detail` 路由和页面布局
2. 再把登录/拉取/详情请求封装成 `api.ts`
3. 用 `useQuery` / `useMutation` 管理异步数据
4. 迁移 `localStorage` 持久化逻辑到 `persisted store`
5. 最后补基础 UI 组件和样式

## 6. 结论

核心选择：

- 组件库：`Skeleton`（推荐）
- 路由逻辑：`svelte-spa-router`（简洁可靠）
- 缓存逻辑：`@tanstack/svelte-query` + `localStorage` 持久化

这种组合在 Tauri 桌面应用中既能保持简单，又能避免目前原生 DOM 版本的状态与路由错乱问题。
