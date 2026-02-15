# Mytool-GPUI

## 当前架构预览

```mermaid
graph TB
    subgraph "视图层 Views"
        V1[InboxBoard]
        V2[TodayBoard]
        V3[ScheduledBoard]
        V4[CompletedBoard]
        V5[PinnedBoard]
    end

    subgraph "状态层 todo_state"
        S1[ItemState]
        S2[InboxItemState]
        S3[TodayItemState]
        S4[ScheduledItemState]
        S5[CompleteItemState]
        S6[PinnedItemState]
        S7[ProjectState]
        S8[SectionState]
    end

    subgraph "操作层 todo_actions"
        A1[add_item]
        A2[update_item]
        A3[delete_item]
        A4[refresh_*]
    end

    subgraph "服务层 service"
        F1[load_items]
        F2[get_inbox_items]
        F3[get_items_today]
        F4[get_items_scheduled]
    end

    V1 --> S2
    V2 --> S3
    V3 --> S4
    V4 --> S5
    V5 --> S6

    S2 --> S1
    S3 --> S1
    S4 --> S1
    S5 --> S1
    S6 --> S1

    A1 --> F1
    A2 --> F1
    A3 --> F1
```

![image-20260215141057753](imgs/image-20260215141057753.png)

## toolchain

x86_64-pc-windows-gnu

## gcc 1.15 编译问题

1. syntect： 改为使用rust实现的正则

```bash
syntect = { version = "5.2", default-features = false, features = [
    "default-fancy",
] }
```

2. gpui.rc问题
   回退Msys2到13.2.0

```bash
# 下载旧版本包
wget https://repo.msys2.org/mingw/x86_64/mingw-w64-x86_64-gcc-13.2.0-1-any.pkg.tar.zst
wget https://repo.msys2.org/mingw/x86_64/mingw-w64-x86_64-gcc-libs-13.2.0-1-any.pkg.tar.zst

# 强制降级
pacman -U --nodeps --force mingw-w64-x86_64-gcc-13.2.0-1-any.pkg.tar.zst
pacman -U --nodeps --force mingw-w64-x86_64-gcc-libs-13.2.0-1-any.pkg.tar.zst

# 锁定版本
echo "IgnorePkg = mingw-w64-x86_64-gcc mingw-w64-x86_64-gcc-libs" >> /etc/pacman.conf
```

## 批量修复

```bash
### 删除未使用的依赖项
cargo install cargo-machete && cargo machete
### 格式化
#### 全部格式化
cargo fmt --all && cargo clippy --fix --allow-dirty --allow-staged

####
# 仅检查修改过的文件（配合git）
git diff --name-only --diff-filter=ACM | grep '\.rs$' | xargs cargo clippy --fix

# 按严重程度分级处理
cargo clippy -- -D clippy::correctness # 先解决正确性问题
cargo clippy -- -W clippy::style       # 再处理风格问题

### 按目录修复
cargo clippy --fix --lib -p todos --allow-dirty
cargo clippy --fix --lib -p mytool --allow-dirty

```

## 代码修复流程

```bash
# 1. 先格式化代码
cargo fmt

# 2. 尝试自动修复所有 Clippy 警告
cargo clippy --fix

# 3. 检查剩余的 style 警告
cargo clippy -- -W clippy::style

# 4. 手动修复不能自动修复的
#    - 使用 IDE 的快速修复
#    - 或者根据建议手动修改

# 5. 再次检查
cargo clippy -- -W clippy::style
```

## 示例

中文日历
![calendar](assets/screenshots/calendar.png)
planify 类似界面 - 开发中...
