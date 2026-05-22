# jxemall 登录请求抓包解析（MVP）

## 1. 已确认请求

- Method: POST
- URL: /login?current_uri=https%3A%2F%2Flogin.jxemall.com%2Fuser-login%2F%23%2F
- Host: login.jxemall.com
- Content-Type: multipart/form-data
- Referer: https://login.jxemall.com/user-login/
- Origin: https://login.jxemall.com

### 1.1 已确认 Form Data（multipart）

字段顺序（按抓包观测）：
1. `platformCode = zcy`
2. `loginType = password`
3. `requestType = async`
4. `username = <账号>`
5. `password = <密文值>`

补充说明：
- `password` 字段观测值为 `zcyFront::...` 形式，当前更像前端预处理后的密文/摘要值，而非明文密码。
- `current_uri` 已确认在 query string 透传，建议保持与抓包一致。

## 2. 头字段分层

### 2.1 业务必要（建议保留）
- Content-Type（multipart/form-data; boundary 由客户端自动生成）
- Origin
- Referer
- Cookie（登录流程需要会话上下文）
- Accept（可保留）

### 2.2 浏览器噪声（建议不强依赖）
- Accept-Encoding
- Accept-Language
- Cache-Control / Pragma
- Sec-Fetch-*
- sec-ch-ua*
- User-Agent（可选，必要时使用固定桌面 UA）
- Connection / Content-Length（由客户端库自动处理）

## 3. Cookie 观察结论

当前抓包 Cookie 中同时出现了：
- districtCode / districtName / districtType
- uid / user_type / tenant_code / institution_id
- SSOSESSION / SESSION

实现建议：
- 不要在代码里硬编码这些 Cookie 键值。
- 首次登录前若服务端要求前置 Cookie，应先访问登录页获取初始 Set-Cookie，再发登录请求。
- 登录成功判定优先依据：
  1) 响应状态码
  2) 响应体业务码
  3) Set-Cookie 中会话键更新（例如 SSOSESSION/SESSION）

## 4. 仍需补抓的数据（才能实现真实登录）

1. `password` 的生成规则（明文 -> `zcyFront::...` 的算法与参数）
2. 登录失败响应体样例（JSON）
3. 登录后首个受保护接口的请求与返回（用于会话有效性校验）

## 4.1 已确认成功响应样例

HTTP 状态码：`200 OK`

响应体（脱敏后）：

```json
{
  "code": "0000",
  "data": {
    "processingUrl": "https://member.jxemall.com/login",
    "redirect": true,
    "showCaptcha": false,
    "userId": 10008679985
  },
  "message": "成功",
  "success": true
}
```

响应头关键信息：
- `Set-Cookie: SSOSESSION=...; HttpOnly; Secure`
- `Set-Cookie: platform_code=zcy; Domain=jxemall.com; Path=/; Secure`

登录成功判定建议（按优先级）：
1. `HTTP 200`
2. `code == "0000" && success == true`
3. 返回 `data.redirect == true` 且存在 `data.processingUrl`
4. `Set-Cookie` 中出现新的会话键（例如 `SSOSESSION`）

## 7. 密码加密算法定位结果（已确认）

通过分析登录页加载的 `chunk-vendors.a4745cd8.js`，已确认：

1. 前端调用链
- 登录表单提交时调用：`password: encrypt(明文密码)`。
- 该 `encrypt` 来自模块 `e1b6`。

2. 算法实现
- 使用 SM4 对称加密实现（代码中 `S.encrypt` / `S.decrypt`）。
- 密钥常量：`edbd2139d9a7766e0382a2e6f92e9113`（16 字节 hex，SM4-128）。
- 输入编码：`utf8`。
- 输出编码：`hex`。
- 分组填充：PKCS7（实现中按 16 字节补位）。
- 结果前缀：`zcyFront::`。

3. 是否启用加密的开关
- 前端会先请求：
  - `/magic/front/service/static/zcy.getPasswordConfig.getPasswordStatus/api`
  - 特定平台可能改为供应链路径（代码中有分支）。
- 响应中的 `result.openEncrypt` 控制是否启用加密。
- 当 `openEncrypt=true` 时：发送 `zcyFront::<sm4_hex>`。
- 当 `openEncrypt=false` 时：前端可能直接发送原值。

4. 工程落地建议
- 默认按 `openEncrypt=true` 实现（与当前抓包一致）。
- 保留开关能力，便于兼容 `openEncrypt=false` 场景。

## 5. Rust 实现落地策略（待你补 body 后直接接）

- crawler 层新增 LoginClient：
  - 先 GET 登录页拿初始 Cookie（如果必需）
  - 再 POST /login 发送 multipart 字段
  - 保存会话 Cookie 到凭证存储引用
- gui 层复用现有 LoginPageState：
  - submit -> 调 LoginApiClient -> 返回 credential_ref

当前实现状态：
- 已实现登录请求与响应解析。
- 密码字段已支持“明文自动加密模式”：用户输入明文后自动转换为 `zcyFront::...`。
- 仍兼容密文直传：若输入已是 `zcyFront::...`，则直接透传。

## 6. 风险点

- current_uri 可能影响登录后跳转与票据写入，建议按抓包原值透传。
- 某些字段可能带时间戳或签名，需要抓至少两次成功样本做 diff。
- 若出现验证码/风控字段，需切换为“半自动登录”方案。
