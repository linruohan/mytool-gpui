# 任务管理核心库测试

## 测试文件说明

本目录包含 todos 核心库的单元测试和集成测试。

### 测试结构

- `unit/` - 单元测试
  - `error_test.rs` - 错误类型测试
  - `event_bus_test.rs` - 事件总线测试
  
- `integration/` - 集成测试
  - `repository_test.rs` - Repository 层测试

### 运行测试

```bash
# 运行所有测试
cargo test -p todos

# 运行单元测试
cargo test -p todos --test '*'

# 运行特定测试
cargo test error_test
```

### 测试数据库

集成测试使用内存数据库进行测试，数据库连接字符串为 `sqlite::memory:`。
