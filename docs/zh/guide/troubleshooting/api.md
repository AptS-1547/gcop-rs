# API 问题

## 问题: "401 Unauthorized"

**原因**: API key 无效或已过期

**解决方案**:
1. 验证 API key 是否正确
2. 检查 key 是否过期
3. 从 provider 控制台重新生成 key
4. 更新 config.toml 中的新 key

## 问题: "429 Rate Limit Exceeded"

**原因**: 请求过多

**解决方案**:
1. 稍等片刻再重试
2. 升级你的 API 计划
3. 临时切换到其他 provider

## 问题: "500 Internal Server Error"

**原因**: API 服务暂时不可用

**解决方案**:
1. 等待并重试
2. 检查 provider 的状态页面
3. 尝试其他 provider

