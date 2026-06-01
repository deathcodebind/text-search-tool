# Svelte 迁移任务清单

## 1. 项目初始化

- [ ] 在 `ui/` 目录创建 `package.json`、`vite.config.ts`、`tsconfig.json`
- [ ] 安装 `svelte`, `@sveltejs/vite-plugin-svelte`, `vite`
- [ ] 安装组件库：`@skeletonlabs/skeleton`
- [ ] 安装路由与缓存依赖：`svelte-spa-router`, `@tanstack/svelte-query`
- [ ] 准备 `src/main.ts`、`src/App.svelte`, `src/routes`、`src/stores`

## 2. 页面与路由搭建

- [ ] 设计四个核心页面组件：`Login.svelte`, `Pull.svelte`, `Keyword.svelte`, `Detail.svelte`
- [ ] 配置 `svelte-spa-router` 路由路径：
  - `/login`
  - `/pull`
  - `/keyword`
  - `/detail/:sourceId`
- [ ] 支持直接打开路由和浏览器返回
- [ ] 在页面顶部或列表页加“返回”按钮

## 3. 组件库与 UI 规范

- [ ] 使用 `Skeleton` 组件构建表单、按钮、卡片、表格、分页、提示信息
- [ ] 确定主题样式与全局布局
- [ ] 统一按钮、输入、列表、提示条的交互行为

## 4. 状态与缓存逻辑

- [ ] 抽象 `login` 状态存储：`baseUrl`, `username`, `remember`
- [ ] 抽象 `pull` 任务状态：`jobId`, `currentPage`, `selectedDistricts`
- [ ] 抽象 `keyword` 配置持久化 `localStorage`
- [ ] 使用 `@tanstack/svelte-query` 管理远程数据获取与缓存：
  - `pull_records`
  - `pull_progress`
  - `preview_keyword_groups`
  - `pull_record_detail`
- [ ] 定义缓存失效策略：`pull_start` / `pull_retry_detail` 后刷新列表

## 5. 后端接口对接

- [ ] 统一 Tauri invoke 封装成 `src/api.ts`
- [ ] 校验后端接口参数与返回结构
- [ ] 处理错误提示与重试逻辑

## 6. 迁移验证

- [ ] 登录流程测试
- [ ] 拉取任务启动与进度查询测试
- [ ] 列表分页和详情查看测试
- [ ] 详情重试按钮测试
- [ ] 关键词预览功能测试

## 7. 产物清理与提交

- [ ] 删除遗留的原生 `app.js` 逻辑或旧 DOM 页面
- [ ] 保留可复用数据文件（`jxemall-district-options.js`）
- [ ] 提交迁移基础结构变更

---

### 备注

- 当前目标是先完成基本页面与路由，再逐步补全业务逻辑。
- 该清单可作为一次完整迁移迭代的第一期任务。
